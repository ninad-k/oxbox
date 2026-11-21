//! First-install startup policy, separate from platform registration.
#[derive(Debug, Clone, Copy, Default)]
pub struct StartupPolicy {
    pub installed_layout: bool,
    pub explicit_profile: bool,
    pub previous_profile: bool,
    pub saved_choice: bool,
    pub registered: bool,
}
use crate::{CoreError, CoreResult};
pub trait Registration {
    fn enabled(&self) -> CoreResult<bool>;
    fn set_enabled(&mut self, enabled: bool) -> CoreResult<()>;
}
pub fn remove_owned_command(
    current: &str,
    expected: &str,
    remove: &mut dyn FnMut() -> CoreResult<()>,
) -> CoreResult<bool> {
    if expected.is_empty() || current != expected {
        return Ok(false);
    }
    remove()?;
    Ok(true)
}
/// Quote the executable argument for Windows Run; other adapters accept a path directly.
pub fn executable_path(path: &str, windows: bool) -> CoreResult<String> {
    if path.is_empty() || path.chars().any(char::is_control) || windows && path.contains('"') {
        return Err(CoreError::InvalidInput(
            "Application path is not a valid startup executable".into(),
        ));
    }
    Ok(if windows {
        format!("\"{path}\"")
    } else {
        path.to_owned()
    })
}
pub fn configure(
    registration: &mut dyn Registration,
    enabled: bool,
    persist: &mut dyn FnMut(bool) -> CoreResult<()>,
) -> CoreResult<bool> {
    let before = registration.enabled()?;
    if before != enabled
        && let Err(error) = registration.set_enabled(enabled)
    {
        return Err(restore(registration, before, error));
    }
    if let Err(error) = persist(enabled) {
        return Err(if before != enabled {
            restore(registration, before, error)
        } else {
            error
        });
    }
    Ok(enabled)
}
fn restore(registration: &mut dyn Registration, before: bool, original: CoreError) -> CoreError {
    match registration.set_enabled(before) {
        Ok(()) => original,
        Err(rollback) => CoreError::InvalidInput(format!(
            "{original}. Startup registration could not be restored: {rollback}"
        )),
    }
}
impl StartupPolicy {
    pub fn enable_by_default(self) -> bool {
        self.installed_layout
            && !self.explicit_profile
            && !self.previous_profile
            && !self.saved_choice
            && !self.registered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_installed_app_enables_startup_by_default() {
        assert!(
            StartupPolicy {
                installed_layout: true,
                ..Default::default()
            }
            .enable_by_default()
        );
    }
    #[test]
    fn windows_startup_quotes_spaces_and_other_platforms_keep_the_path() {
        let path = r"C:\Program Files\Oxbox\oxbox-native.exe";
        assert_eq!(executable_path(path, true).unwrap(), format!("\"{path}\""));
        assert_eq!(
            executable_path("/Applications/Oxbox.app/Contents/MacOS/oxbox-native", false).unwrap(),
            "/Applications/Oxbox.app/Contents/MacOS/oxbox-native"
        );
        assert!(executable_path("bad\"path", true).is_err());
    }
    #[test]
    fn uninstall_removes_only_the_exact_owned_startup_command() {
        let expected = r#""C:\Program Files\Oxbox\oxbox-native.exe" --autostart"#;
        let mut removed = 0;
        assert!(
            remove_owned_command(expected, expected, &mut || {
                removed += 1;
                Ok(())
            })
            .unwrap()
        );
        for command in [
            r#""C:\Other\oxbox-native.exe" --autostart"#,
            r#""C:\Program Files\Oxbox\oxbox-native.exe" --profile alternate"#,
            "",
        ] {
            assert!(
                !remove_owned_command(command, expected, &mut || {
                    removed += 1;
                    Ok(())
                })
                .unwrap()
            );
        }
        assert_eq!(removed, 1);
    }
    #[test]
    fn development_qa_existing_choices_and_existing_profiles_are_preserved() {
        for policy in [
            StartupPolicy::default(),
            StartupPolicy {
                installed_layout: true,
                explicit_profile: true,
                ..Default::default()
            },
            StartupPolicy {
                installed_layout: true,
                previous_profile: true,
                ..Default::default()
            },
            StartupPolicy {
                installed_layout: true,
                saved_choice: true,
                ..Default::default()
            },
            StartupPolicy {
                installed_layout: true,
                registered: true,
                ..Default::default()
            },
        ] {
            assert!(!policy.enable_by_default());
        }
    }
    #[derive(Default)]
    struct Fake {
        enabled: bool,
        fail_once: bool,
    }
    impl Registration for Fake {
        fn enabled(&self) -> CoreResult<bool> {
            Ok(self.enabled)
        }
        fn set_enabled(&mut self, enabled: bool) -> CoreResult<()> {
            self.enabled = enabled;
            if std::mem::take(&mut self.fail_once) {
                Err(CoreError::InvalidInput(
                    "partial registration failure".into(),
                ))
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn enable_and_user_opt_out_commit_registration_and_choice() {
        let mut registration = Fake::default();
        let mut stored = None;
        configure(&mut registration, true, &mut |value| {
            stored = Some(value);
            Ok(())
        })
        .unwrap();
        assert!(registration.enabled);
        assert_eq!(stored, Some(true));
        configure(&mut registration, false, &mut |value| {
            stored = Some(value);
            Ok(())
        })
        .unwrap();
        assert!(!registration.enabled);
        assert_eq!(stored, Some(false));
    }
    #[test]
    fn failed_storage_restores_the_previous_registration() {
        let mut registration = Fake::default();
        assert!(
            configure(&mut registration, true, &mut |_| Err(
                CoreError::InvalidInput("storage failed".into())
            ))
            .is_err()
        );
        assert!(!registration.enabled);
    }
    #[test]
    fn partially_failed_registration_rolls_back_without_saving_choice() {
        let mut registration = Fake {
            fail_once: true,
            ..Default::default()
        };
        let mut wrote = false;
        assert!(
            configure(&mut registration, true, &mut |_| {
                wrote = true;
                Ok(())
            })
            .is_err()
        );
        assert!(!registration.enabled);
        assert!(!wrote);
    }
}

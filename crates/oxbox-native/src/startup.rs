//! Installed-copy login defaults and explicit, reversible startup preferences.
use crate::host::Host;
use oxbox_core::{
    CoreError, CoreResult,
    startup::{self, Registration, StartupPolicy},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Choice {
    enabled: bool,
}
#[derive(Deserialize)]
struct Installed {
    app: String,
    schema: u32,
}
pub struct Status {
    pub enabled: bool,
    pub error: Option<String>,
}

struct Launcher(auto_launch::AutoLaunch);
impl Registration for Launcher {
    fn enabled(&self) -> CoreResult<bool> {
        #[cfg(windows)]
        if !windows_registration::user_run_exists() {
            return Ok(false);
        }
        self.0
            .is_enabled()
            .map_err(|error| CoreError::InvalidInput(error.to_string()))
    }
    fn set_enabled(&mut self, enabled: bool) -> CoreResult<()> {
        #[cfg(windows)]
        if !enabled {
            return windows_registration::disable_current_user();
        }
        (if enabled {
            self.0.enable()
        } else {
            self.0.disable()
        })
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
    }
}
fn launcher() -> CoreResult<Launcher> {
    let path = std::env::current_exe()?;
    let path = path
        .to_str()
        .ok_or_else(|| CoreError::InvalidInput("Application path is not valid Unicode".into()))?;
    let path = startup::executable_path(path, cfg!(windows))?;
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Oxbox")
        .set_app_path(&path)
        .set_args(&["--autostart"])
        .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser)
        .build()
        .map(Launcher)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))
}
fn installed_layout(executable: &Path) -> bool {
    let Some(directory) = executable.parent() else {
        return false;
    };
    let resources = if directory.file_name().is_some_and(|name| name == "MacOS") {
        directory.parent().map(|parent| parent.join("Resources"))
    } else {
        Some(directory.to_path_buf())
    };
    resources
        .and_then(|directory| {
            oxbox_core::preferences::load::<Installed>(&directory, "oxbox-installed.json")
                .ok()
                .flatten()
        })
        .is_some_and(|marker| marker.app == "Oxbox" && marker.schema == 1)
}
pub fn enabled() -> Result<bool, String> {
    launcher()
        .and_then(|launcher| launcher.enabled())
        .map_err(|error| error.to_string())
}
pub fn settings_available() -> bool {
    !cfg!(debug_assertions) && !std::env::args().any(|arg| arg == "--profile")
}
/// Installer-only cleanup: no window, profile access, or startup re-registration.
pub fn remove_if_owned() -> Result<(), String> {
    #[cfg(windows)]
    {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let path = executable
            .to_str()
            .ok_or("Application path is not valid Unicode")?;
        let expected = format!(
            "{} --autostart",
            startup::executable_path(path, true).map_err(|error| error.to_string())?
        );
        if let Some(current) =
            windows_registration::current_user_command().map_err(|error| error.to_string())?
        {
            startup::remove_owned_command(
                &current,
                &expected,
                &mut windows_registration::disable_current_user,
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
pub fn set_enabled(host: &Host, enabled: bool) -> Result<bool, String> {
    if !settings_available() {
        return Err("Login startup is disabled for development and isolated QA profiles.".into());
    }
    // Preserve a malformed existing store; do not mutate the OS before validation.
    let _choice = host
        .load::<Choice>("native-startup.json")
        .map_err(|error| error.to_string())?;
    let mut launcher = launcher().map_err(|error| error.to_string())?;
    startup::configure(&mut launcher, enabled, &mut |enabled| {
        host.save("native-startup.json", &Choice { enabled })
    })
    .map_err(|error| error.to_string())
}
pub fn initialize(host: &Host, default_allowed: bool) -> Status {
    let result = (|| -> CoreResult<bool> {
        let mut launcher = launcher()?;
        let registered = launcher.enabled()?;
        let choice = host.load::<Choice>("native-startup.json")?;
        let installed = std::env::current_exe().is_ok_and(|path| installed_layout(&path));
        let previous_profile = ["settings.json", "native-shell.json"]
            .iter()
            .any(|name| host.data_dir().join(name).exists());
        #[cfg(windows)]
        let previous_profile = previous_profile || windows_registration::any_previous_choice();
        let policy = StartupPolicy {
            installed_layout: installed,
            explicit_profile: !default_allowed || cfg!(debug_assertions),
            previous_profile,
            saved_choice: choice.is_some(),
            registered,
        };
        if policy.enable_by_default() {
            startup::configure(&mut launcher, true, &mut |enabled| {
                host.save("native-startup.json", &Choice { enabled })
            })
        } else {
            // Record the actual pre-existing choice once. Never re-enable an upgrade's opt-out.
            if installed && default_allowed && choice.is_none() {
                host.save(
                    "native-startup.json",
                    &Choice {
                        enabled: registered,
                    },
                )?;
            }
            Ok(registered)
        }
    })();
    match result {
        Ok(enabled) => Status {
            enabled,
            error: None,
        },
        Err(error) => Status {
            enabled: enabled().unwrap_or(false),
            error: Some(format!("Login startup could not be configured: {error}")),
        },
    }
}

#[cfg(windows)]
mod windows_registration {
    use oxbox_core::{CoreError, CoreResult};
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_ANY, RRF_RT_REG_SZ,
            RegDeleteKeyValueW, RegGetValueW,
        },
    };
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn exists(root: HKEY, key: &str) -> bool {
        let key = wide(key);
        let name = wide("Oxbox");
        let mut size = 0;
        // Read existence only; never modify OS StartupApproved preferences.
        unsafe {
            RegGetValueW(
                root,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_ANY,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            ) == ERROR_SUCCESS
        }
    }
    pub fn user_run_exists() -> bool {
        exists(HKEY_CURRENT_USER, RUN)
    }
    pub fn current_user_command() -> CoreResult<Option<String>> {
        let key = wide(RUN);
        let name = wide("Oxbox");
        let mut buffer = vec![0u16; 32768];
        let mut size = (buffer.len() * 2) as u32;
        // A bounded UTF-16 buffer; oversized/malformed entries are preserved.
        let code = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if [ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND].contains(&code) {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(CoreError::InvalidInput(format!(
                "Could not inspect this user's startup entry: {}",
                std::io::Error::from_raw_os_error(code as i32)
            )));
        }
        buffer.truncate((size as usize / 2).min(buffer.len()));
        while buffer.last() == Some(&0) {
            buffer.pop();
        }
        String::from_utf16(&buffer).map(Some).map_err(|_| {
            CoreError::InvalidInput(
                "Startup entry contains invalid Unicode and was preserved".into(),
            )
        })
    }
    pub fn any_previous_choice() -> bool {
        [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
            .into_iter()
            .any(|root| exists(root, RUN) || exists(root, APPROVED))
    }
    pub fn disable_current_user() -> CoreResult<()> {
        let key = wide(RUN);
        let name = wide("Oxbox");
        // The upstream adapter removes machine-wide entries too. This app only owns HKCU.
        let code = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
        if [ERROR_SUCCESS, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND].contains(&code) {
            Ok(())
        } else {
            Err(CoreError::InvalidInput(format!(
                "Could not remove this user's startup entry: {}",
                std::io::Error::from_raw_os_error(code as i32)
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_packaged_markers_enable_installation_detection() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("oxbox-native.exe");
        assert!(!installed_layout(&executable));
        std::fs::write(
            root.path().join("oxbox-installed.json"),
            r#"{"app":"Other","schema":1}"#,
        )
        .unwrap();
        assert!(!installed_layout(&executable));
        std::fs::write(
            root.path().join("oxbox-installed.json"),
            r#"{"app":"Oxbox","schema":1}"#,
        )
        .unwrap();
        assert!(installed_layout(&executable));
    }
    #[test]
    fn mac_bundle_uses_the_resources_directory() {
        let root = tempfile::tempdir().unwrap();
        let resources = root.path().join("Oxbox.app/Contents/Resources");
        std::fs::create_dir_all(&resources).unwrap();
        std::fs::write(
            resources.join("oxbox-installed.json"),
            r#"{"app":"Oxbox","schema":1}"#,
        )
        .unwrap();
        assert!(installed_layout(
            &root.path().join("Oxbox.app/Contents/MacOS/oxbox-native")
        ));
    }
}

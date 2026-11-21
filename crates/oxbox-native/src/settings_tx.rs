//! Toolkit-independent global-shortcut registration/persistence transaction.
pub(crate) trait ShortcutBackend<Key> {
    fn active(&self) -> Option<Key>;
    fn register(&mut self, key: &Key) -> Result<(), String>;
    fn unregister(&mut self, key: &Key) -> Result<(), String>;
    fn set_active(&mut self, key: Option<Key>);
}

/// The callback writes a complete candidate preference snapshot for the supplied
/// binding. It is also called with the preceding saved value when rollback is needed.
/// Backend registration ownership is independent from the stored preference string.
pub(crate) fn configure_shortcut<Key: Clone + Eq>(
    backend: &mut impl ShortcutBackend<Key>,
    new: Key,
    value: &str,
    previous_saved: &str,
    mut persist: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let previous_active = backend.active();
    if previous_active.as_ref() == Some(&new) {
        if let Err(error) = persist(value) {
            let mut message = format!("Could not save launcher shortcut: {error}");
            if let Err(rollback) = persist(previous_saved) {
                message.push_str(&format!(
                    "; previous saved binding restore failed: {rollback}"
                ));
            }
            return Err(message);
        }
        return Ok(());
    }
    backend
        .register(&new)
        .map_err(|error| format!("Could not register launcher shortcut: {error}"))?;
    if let Err(error) = persist(value) {
        let mut message = format!("Could not save launcher shortcut: {error}");
        rollback(
            backend,
            &new,
            previous_active,
            previous_saved,
            &mut persist,
            &mut message,
        );
        return Err(message);
    }
    if let Some(previous) = &previous_active
        && let Err(error) = backend.unregister(previous)
    {
        let mut message = format!("Could not release the preceding launcher shortcut: {error}");
        rollback(
            backend,
            &new,
            previous_active,
            previous_saved,
            &mut persist,
            &mut message,
        );
        return Err(message);
    }
    backend.set_active(Some(new));
    Ok(())
}
fn rollback<Key: Clone>(
    backend: &mut impl ShortcutBackend<Key>,
    new: &Key,
    previous_active: Option<Key>,
    previous_saved: &str,
    persist: &mut impl FnMut(&str) -> Result<(), String>,
    message: &mut String,
) {
    if let Err(error) = backend.unregister(new) {
        message.push_str(&format!(
            "; new registration cleanup failed: {error}. Restart Oxbox to retry cleanup"
        ));
    }
    if let Err(error) = persist(previous_saved) {
        message.push_str(&format!("; previous saved binding restore failed: {error}. Repair preference storage before restarting"));
    }
    backend.set_active(previous_active);
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[derive(Default)]
    struct Fake {
        active: Option<u8>,
        registered: BTreeSet<u8>,
        fail_register: Option<u8>,
        fail_unregister: BTreeSet<u8>,
        events: Vec<String>,
    }
    impl Fake {
        fn existing() -> Self {
            Self {
                active: Some(1),
                registered: BTreeSet::from([1]),
                ..Default::default()
            }
        }
    }
    impl ShortcutBackend<u8> for Fake {
        fn active(&self) -> Option<u8> {
            self.active
        }
        fn register(&mut self, key: &u8) -> Result<(), String> {
            self.events.push(format!("register {key}"));
            if self.fail_register == Some(*key) {
                Err("registration conflict".into())
            } else {
                self.registered.insert(*key);
                Ok(())
            }
        }
        fn unregister(&mut self, key: &u8) -> Result<(), String> {
            self.events.push(format!("unregister {key}"));
            if self.fail_unregister.contains(key) {
                Err("OS release refusal".into())
            } else {
                self.registered.remove(key);
                Ok(())
            }
        }
        fn set_active(&mut self, key: Option<u8>) {
            self.active = key;
        }
    }
    #[test]
    fn success_commits_new_binding_after_registration_and_persistence() {
        let mut fake = Fake::existing();
        let mut saved = vec![];
        configure_shortcut(&mut fake, 2, "new", "old", |v| {
            saved.push(v.to_owned());
            Ok(())
        })
        .unwrap();
        assert_eq!(fake.active, Some(2));
        assert_eq!(fake.registered, BTreeSet::from([2]));
        assert_eq!(saved, vec!["new"]);
        assert_eq!(fake.events, vec!["register 2", "unregister 1"]);
    }
    #[test]
    fn registration_conflict_preserves_active_and_never_writes_preferences() {
        let mut fake = Fake::existing();
        fake.fail_register = Some(2);
        let mut saved = vec![];
        assert!(
            configure_shortcut(&mut fake, 2, "new", "old", |v| {
                saved.push(v.to_owned());
                Ok(())
            })
            .is_err()
        );
        assert_eq!(fake.active, Some(1));
        assert_eq!(fake.registered, BTreeSet::from([1]));
        assert!(saved.is_empty());
    }
    #[test]
    fn failed_save_restores_store_and_unregisters_only_the_new_binding() {
        let mut fake = Fake::existing();
        let mut saved = "old".to_owned();
        let error = configure_shortcut(&mut fake, 2, "new", "old", |v| {
            saved = v.to_owned();
            if v == "new" {
                Err("disk full after attempted write".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert!(error.contains("save"));
        assert_eq!(saved, "old");
        assert_eq!(fake.active, Some(1));
        assert_eq!(fake.registered, BTreeSet::from([1]));
        assert_eq!(fake.events, vec!["register 2", "unregister 2"]);
    }
    #[test]
    fn old_release_failure_rolls_back_new_registration_and_saved_value() {
        let mut fake = Fake::existing();
        fake.fail_unregister.insert(1);
        let mut saved = vec![];
        let error = configure_shortcut(&mut fake, 2, "new", "old", |v| {
            saved.push(v.to_owned());
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("release"));
        assert_eq!(saved, vec!["new", "old"]);
        assert_eq!(fake.active, Some(1));
        assert_eq!(fake.registered, BTreeSet::from([1]));
    }
    #[test]
    fn rollback_failures_are_reported_without_claiming_cleanup_succeeded() {
        let mut fake = Fake::existing();
        fake.fail_unregister.insert(2);
        let error = configure_shortcut(&mut fake, 2, "new", "old", |_| {
            Err("storage unavailable".into())
        })
        .unwrap_err();
        assert!(error.contains("new registration cleanup failed"));
        assert!(error.contains("previous saved binding restore failed"));
        assert_eq!(fake.active, Some(1));
        assert_eq!(fake.registered, BTreeSet::from([1, 2]));
    }
    #[test]
    fn no_active_binding_stays_unregistered_after_failed_save() {
        let mut fake = Fake::default();
        let mut saved = vec![];
        assert!(
            configure_shortcut(&mut fake, 2, "new", "previous-conflict", |v| {
                saved.push(v.to_owned());
                if v == "new" {
                    Err("disk full".into())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(fake.active, None);
        assert!(fake.registered.is_empty());
        assert_eq!(saved, vec!["new", "previous-conflict"]);
    }
    #[test]
    fn same_binding_alias_persists_without_registering_again() {
        let mut fake = Fake::existing();
        let mut saved = vec![];
        configure_shortcut(&mut fake, 1, "equivalent alias", "old", |v| {
            saved.push(v.to_owned());
            Ok(())
        })
        .unwrap();
        assert_eq!(saved, vec!["equivalent alias"]);
        assert!(fake.events.is_empty());
        assert_eq!(fake.active, Some(1));
    }
    #[test]
    fn same_binding_failed_save_restores_store_without_releasing_active() {
        let mut fake = Fake::existing();
        let mut saved = vec![];
        assert!(
            configure_shortcut(&mut fake, 1, "alias", "old", |v| {
                saved.push(v.to_owned());
                if v == "alias" {
                    Err("disk full".into())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(saved, vec!["alias", "old"]);
        assert!(fake.events.is_empty());
        assert_eq!(fake.active, Some(1));
    }
}

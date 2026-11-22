//! Conservative HKCU App Paths review. No broad registry/performance heuristics.

use super::maintenance_support::{
    self as support, APP_PATHS_KEY, NativeRegistry, RegistryArea, RegistryBackend, RegistryValue,
    invalid,
};
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
    platform::Os,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Candidate {
    pub location: String,
    pub target: PathBuf,
    pub reason: String,
    value: RegistryValue,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Backup {
    schema: u8,
    purpose: String,
    value: RegistryValue,
}
#[derive(Clone, Debug)]
pub struct RecoveryReview {
    pub description: String,
    path: PathBuf,
    saved: Backup,
}
fn app_mapping(value: &RegistryValue) -> CoreResult<String> {
    support::validate_registry_value(value)?;
    if !value.key.starts_with(&format!("{APP_PATHS_KEY}\\"))
        || !value.name.is_empty()
        || value.kind != 1
    {
        return Err(invalid(
            "Only default REG_SZ current-user App Paths mappings are eligible.",
        ));
    }
    let text = support::registry_text(value)?;
    let bytes = text.as_bytes();
    if bytes.len() < 7
        || bytes.len() > 32_768
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
        || !text.to_ascii_lowercase().ends_with(".exe")
        || text[2..].contains([':', '%', '"', '*', '?'])
        || text.chars().any(char::is_control)
        || text
            .split(['\\', '/'])
            .any(|part| part == "." || part == "..")
    {
        return Err(invalid(
            "The mapping is not an unambiguous absolute local executable path.",
        ));
    }
    Ok(text)
}
fn missing_fixed_target(path: &Path) -> CoreResult<bool> {
    support::local_path(path)?;
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        let letter = text.as_bytes()[0];
        let drive = [u16::from(letter), b':' as u16, b'\\' as u16, 0];
        // Only an available fixed local volume can establish an obsolete mapping.
        if unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(drive.as_ptr()) } != 3 {
            return Ok(false);
        }
    }
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e.into()),
    }
}
fn scan_with(
    backend: &mut dyn RegistryBackend,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    missing: &mut dyn FnMut(&Path) -> CoreResult<bool>,
) -> CoreResult<Vec<Candidate>> {
    cancel.check()?;
    let values = backend.list(RegistryArea::AppPaths)?;
    if values.len() > 1024 {
        return Err(invalid("Registry review is limited to 1,024 mappings."));
    }
    let mut result = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        cancel.check()?;
        if let Ok(target) = app_mapping(&value) {
            let target = PathBuf::from(target);
            if missing(&target).unwrap_or(false) {
                result.push(Candidate { location: format!("HKCU\\{} (default value)", value.key), target,
                    reason: "The registered executable is missing on an available fixed local drive. Only this exact default mapping is removed; other values and the key are kept.".into(), value });
            }
        }
        progress(Progress {
            done: index as u64 + 1,
            total: 0,
            message: "Reviewing current-user executable mappings; no changes".into(),
        });
    }
    Ok(result)
}
pub fn scan(
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<Candidate>> {
    if Os::current() != Os::Windows {
        return Err(CoreError::Unsupported(Os::current().display_name()));
    }
    scan_with(
        &mut NativeRegistry,
        cancel,
        progress,
        &mut missing_fixed_target,
    )
}
fn revalidate(
    review: &Candidate,
    backend: &mut dyn RegistryBackend,
    missing: &mut dyn FnMut(&Path) -> CoreResult<bool>,
) -> CoreResult<()> {
    if app_mapping(&review.value)? != review.target.to_string_lossy()
        || backend.read(&review.value)?.as_ref() != Some(&review.value)
        || !missing(&review.target)?
    {
        return Err(invalid(
            "The mapping or target changed; scan and review again.",
        ));
    }
    Ok(())
}
fn remove_with(
    review: &Candidate,
    backup: &Path,
    confirmed: bool,
    cancel: &CancelToken,
    backend: &mut dyn RegistryBackend,
    missing: &mut dyn FnMut(&Path) -> CoreResult<bool>,
) -> CoreResult<()> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid(
            "Confirm the exact missing-target mapping and backup location.",
        ));
    }
    revalidate(review, backend, missing)?;
    support::write_backup(
        backup,
        &Backup {
            schema: 1,
            purpose: "oxbox-registry-mapping".into(),
            value: review.value.clone(),
        },
    )?;
    cancel.check()?;
    revalidate(review, backend, missing)?;
    backend.change(Some(&review.value), None)
}
pub fn remove(
    review: &Candidate,
    backup: &Path,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    if Os::current() != Os::Windows {
        return Err(CoreError::Unsupported(Os::current().display_name()));
    }
    remove_with(
        review,
        backup,
        confirmed,
        cancel,
        &mut NativeRegistry,
        &mut missing_fixed_target,
    )
}
fn load_recovery(backup: &Path) -> CoreResult<Backup> {
    let saved: Backup = support::read_backup(backup)?;
    if saved.schema != 1 || saved.purpose != "oxbox-registry-mapping" {
        return Err(invalid("Choose an Oxbox registry-mapping backup."));
    }
    app_mapping(&saved.value)?;
    Ok(saved)
}
pub fn review_recovery(backup: &Path) -> CoreResult<RecoveryReview> {
    let saved = load_recovery(backup)?;
    let description = format!(
        "Restore HKCU\\{} (default) to {}? The current value must be absent; existing data is never overwritten.",
        saved.value.key,
        support::registry_text(&saved.value)?
    );
    Ok(RecoveryReview {
        description,
        path: backup.to_path_buf(),
        saved,
    })
}
pub fn recovery_description(backup: &Path) -> CoreResult<String> {
    Ok(review_recovery(backup)?.description)
}
#[cfg(test)]
fn restore_with(
    backup: &Path,
    confirmed: bool,
    cancel: &CancelToken,
    backend: &mut dyn RegistryBackend,
) -> CoreResult<()> {
    restore_reviewed_with(&review_recovery(backup)?, confirmed, cancel, backend)
}
fn restore_reviewed_with(
    review: &RecoveryReview,
    confirmed: bool,
    cancel: &CancelToken,
    backend: &mut dyn RegistryBackend,
) -> CoreResult<()> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid(
            "Confirm the exact mapping before restoring its backup.",
        ));
    }
    if load_recovery(&review.path)? != review.saved {
        return Err(invalid(
            "The recovery backup changed after review. Review it again.",
        ));
    }
    let saved = &review.saved;
    if backend.read(&saved.value)?.is_some() {
        return Err(invalid(
            "A value already exists; restoration will not overwrite it.",
        ));
    }
    cancel.check()?;
    backend.change(None, Some(&saved.value))
}
pub fn restore(backup: &Path, confirmed: bool, cancel: &CancelToken) -> CoreResult<()> {
    restore_reviewed(&review_recovery(backup)?, confirmed, cancel)
}
pub fn restore_reviewed(
    review: &RecoveryReview,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    if Os::current() != Os::Windows {
        return Err(CoreError::Unsupported(Os::current().display_name()));
    }
    restore_reviewed_with(review, confirmed, cancel, &mut NativeRegistry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CoreResult,
        jobs::CancelToken,
        tools::maintenance_support::{APP_PATHS_KEY, RegistryArea, RegistryBackend, RegistryValue},
    };

    #[derive(Default)]
    struct FakeRegistry {
        value: Option<RegistryValue>,
        changes: usize,
    }
    impl RegistryBackend for FakeRegistry {
        fn list(&mut self, _: RegistryArea) -> CoreResult<Vec<RegistryValue>> {
            Ok(self.value.iter().cloned().collect())
        }
        fn read(&mut self, _: &RegistryValue) -> CoreResult<Option<RegistryValue>> {
            Ok(self.value.clone())
        }
        fn change(
            &mut self,
            expected: Option<&RegistryValue>,
            desired: Option<&RegistryValue>,
        ) -> CoreResult<()> {
            assert_eq!(self.value.as_ref(), expected);
            self.value = desired.cloned();
            self.changes += 1;
            Ok(())
        }
    }
    fn mapping() -> RegistryValue {
        RegistryValue {
            key: format!("{APP_PATHS_KEY}\\example.exe"),
            name: String::new(),
            kind: 1,
            data: "C:\\Missing\\example.exe\0"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect(),
        }
    }

    #[test]
    fn registry_review_requires_missing_target_confirmation_backup_and_exact_revalidation() {
        let root = tempfile::tempdir().unwrap();
        let original = mapping();
        let mut backend = FakeRegistry {
            value: Some(original.clone()),
            ..Default::default()
        };
        let candidates = scan_with(
            &mut backend,
            &CancelToken::default(),
            &mut |_| {},
            &mut |_| Ok(true),
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);
        let review = &candidates[0];
        let backup = root.path().join("registry.json");
        assert!(
            remove_with(
                review,
                &backup,
                false,
                &CancelToken::default(),
                &mut backend,
                &mut |_| Ok(true)
            )
            .is_err()
        );
        assert!(!backup.exists());
        assert_eq!(backend.changes, 0);
        remove_with(
            review,
            &backup,
            true,
            &CancelToken::default(),
            &mut backend,
            &mut |_| Ok(true),
        )
        .unwrap();
        assert!(backup.is_file());
        assert!(backend.value.is_none());
        assert_eq!(backend.changes, 1);
        restore_with(&backup, true, &CancelToken::default(), &mut backend).unwrap();
        assert_eq!(backend.value, Some(original.clone()));
        assert!(restore_with(&backup, true, &CancelToken::default(), &mut backend).is_err());
        backend.value.as_mut().unwrap().data.push(0);
        assert!(
            remove_with(
                review,
                &root.path().join("changed.json"),
                true,
                &CancelToken::default(),
                &mut backend,
                &mut |_| Ok(true)
            )
            .is_err()
        );
        assert!(!root.path().join("changed.json").exists());
        assert_eq!(backend.changes, 2);
    }

    #[test]
    fn registry_review_excludes_present_ambiguous_and_non_app_path_values() {
        let mut backend = FakeRegistry {
            value: Some(mapping()),
            ..Default::default()
        };
        assert!(
            scan_with(
                &mut backend,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| Ok(false)
            )
            .unwrap()
            .is_empty()
        );
        backend.value.as_mut().unwrap().kind = 2;
        assert!(
            scan_with(
                &mut backend,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| Ok(true)
            )
            .unwrap()
            .is_empty()
        );
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            scan_with(&mut backend, &cancel, &mut |_| {}, &mut |_| Ok(true))
                .unwrap_err()
                .code(),
            "cancelled"
        );
    }
    #[test]
    fn registry_review_restore_rejects_backup_changed_after_its_review() {
        let root = tempfile::tempdir().unwrap();
        let backup = root.path().join("review.json");
        let mut backend = FakeRegistry {
            value: Some(mapping()),
            ..Default::default()
        };
        let candidate = scan_with(
            &mut backend,
            &CancelToken::default(),
            &mut |_| {},
            &mut |_| Ok(true),
        )
        .unwrap()
        .remove(0);
        remove_with(
            &candidate,
            &backup,
            true,
            &CancelToken::default(),
            &mut backend,
            &mut |_| Ok(true),
        )
        .unwrap();
        let review = review_recovery(&backup).unwrap();
        let mut saved: Backup = crate::tools::maintenance_support::read_backup(&backup).unwrap();
        saved.value.data = "C:\\Other\\other.exe\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        std::fs::write(&backup, serde_json::to_vec(&saved).unwrap()).unwrap();
        assert!(
            restore_reviewed_with(&review, true, &CancelToken::default(), &mut backend).is_err()
        );
        assert!(backend.value.is_none());
        assert_eq!(backend.changes, 1);
    }
}

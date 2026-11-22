//! Validated application/file launch sets; configuration has no shell strings.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchKind {
    Application,
    File,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchEntry {
    pub kind: LaunchKind,
    pub target: String,
    pub args: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSet {
    pub id: String,
    pub name: String,
    pub entries: Vec<LaunchEntry>,
}
pub fn validate_set(set: &AppSet) -> CoreResult<()> {
    if set.id.is_empty()
        || set.id.len() > 80
        || !set
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || set.name.trim().is_empty()
        || set.name.len() > 200
        || set.entries.is_empty()
        || set.entries.len() > 32
    {
        return Err(CoreError::InvalidInput(
            "a launch set needs a name and 1–32 local applications/files".into(),
        ));
    }
    for entry in &set.entries {
        if entry.target.len() > 32768
            || entry.target.contains('\0')
            || !Path::new(&entry.target).is_absolute()
            || entry.target.starts_with("\\\\")
            || entry.target.contains("://")
            || entry.args.len() > 64
            || entry
                .args
                .iter()
                .any(|a| a.len() > 8192 || a.contains('\0'))
            || (entry.kind == LaunchKind::File && !entry.args.is_empty())
        {
            return Err(CoreError::InvalidInput("choose absolute local paths and literal argument lists; URLs and network paths are not accepted".into()));
        }
        #[cfg(windows)]
        if entry.kind == LaunchKind::Application
            && !Path::new(&entry.target)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        {
            return Err(CoreError::InvalidInput(
                "choose a native .exe application; shell scripts are not launched".into(),
            ));
        }
    }
    Ok(())
}
pub fn launch(set: &AppSet, open: &mut dyn FnMut(&str) -> CoreResult<()>) -> CoreResult<usize> {
    launch_with_cancel(set, &crate::jobs::CancelToken::default(), open)
}
/// Stop admitting additional targets when the owning job is cancelled or quits.
/// Already launched external applications remain under the user's control.
pub fn launch_with_cancel(
    set: &AppSet,
    cancel: &crate::jobs::CancelToken,
    open: &mut dyn FnMut(&str) -> CoreResult<()>,
) -> CoreResult<usize> {
    cancel.check()?;
    validate_set(set)?;
    for entry in &set.entries {
        cancel.check()?;
        let metadata = std::fs::metadata(&entry.target)?;
        let is_bundle = cfg!(target_os = "macos")
            && entry.kind == LaunchKind::Application
            && metadata.is_dir()
            && Path::new(&entry.target)
                .extension()
                .is_some_and(|e| e == "app");
        if !metadata.is_file() && !is_bundle {
            return Err(CoreError::InvalidInput(
                "launch targets must be files".into(),
            ));
        }
    }
    let mut count = 0;
    for entry in &set.entries {
        cancel.check()?;
        match entry.kind {
            LaunchKind::Application => {
                #[cfg(target_os = "macos")]
                if Path::new(&entry.target)
                    .extension()
                    .is_some_and(|e| e == "app")
                {
                    std::process::Command::new("/usr/bin/open")
                        .args(["-a", &entry.target, "--args"])
                        .args(&entry.args)
                        .spawn()?;
                    count += 1;
                    continue;
                }
                std::process::Command::new(&entry.target)
                    .args(&entry.args)
                    .spawn()?;
            }
            LaunchKind::File => open(&entry.target)?,
        }
        count += 1;
    }
    Ok(count)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_sets_relative_paths_and_embedded_nul() {
        let mut set = AppSet {
            id: "work".into(),
            name: "Work".into(),
            entries: vec![],
        };
        assert!(validate_set(&set).is_err());
        set.entries.push(LaunchEntry {
            kind: LaunchKind::File,
            target: "relative.txt".into(),
            args: vec![],
        });
        assert!(validate_set(&set).is_err());
        set.entries[0].target = std::env::temp_dir()
            .join("file.txt")
            .to_string_lossy()
            .into();
        assert!(validate_set(&set).is_ok());
        set.entries[0].args.push("a\0b".into());
        assert!(validate_set(&set).is_err());
    }

    #[test]
    fn cancellation_after_first_open_prevents_later_launch_targets() {
        let folder = tempfile::tempdir().unwrap();
        let mut entries = vec![];
        for name in ["first.txt", "second.txt"] {
            let filename = folder.path().join(name);
            std::fs::write(&filename, "generated launch fixture").unwrap();
            entries.push(LaunchEntry {
                kind: LaunchKind::File,
                target: filename.to_string_lossy().into_owned(),
                args: vec![],
            });
        }
        let set = AppSet {
            id: "cancel-fixture".into(),
            name: "Cancellation fixture".into(),
            entries,
        };
        let cancel = crate::jobs::CancelToken::default();
        let mut opened = vec![];
        let result = launch_with_cancel(&set, &cancel, &mut |target| {
            opened.push(target.to_owned());
            cancel.cancel();
            Ok(())
        });
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert_eq!(opened, vec![set.entries[0].target.clone()]);
        let mut called = false;
        assert_eq!(
            launch_with_cancel(&set, &cancel, &mut |_| {
                called = true;
                Ok(())
            })
            .unwrap_err()
            .code(),
            "cancelled"
        );
        assert!(!called);
    }
}

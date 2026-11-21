//! File-system guards and Recycle Bin / Trash operations.

use crate::{CoreError, CoreResult};
use std::{
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
};

/// Validate a single filename that remains a plain file on Windows and POSIX.
/// Archive entries use this regardless of the host OS so ADS/device syntax cannot
/// become dangerous when an archive is moved between platforms.
pub fn validate_portable_filename(name: &str) -> CoreResult<()> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    let numbered_device = ["COM", "LPT"].iter().any(|prefix| {
        matches!(
            stem.strip_prefix(prefix),
            Some("0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
        )
    });
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|value| value <= '\u{1f}' || value == '\u{7f}' || "<>:\"/\\|?*".contains(value))
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
        )
        || numbered_device
    {
        return Err(CoreError::InvalidInput("choose a plain filename without device names, alternate streams, separators or reserved characters".into()));
    }
    Ok(())
}

/// Validate only the final output component, retaining native POSIX filename
/// support while rejecting Windows alternate streams and DOS device aliases.
pub fn validate_output_filename(name: &OsStr) -> CoreResult<()> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || name
            .as_encoded_bytes()
            .iter()
            .any(|value| *value == 0 || *value == b'/')
    {
        return Err(CoreError::InvalidInput(
            "choose a plain output filename".into(),
        ));
    }
    #[cfg(windows)]
    validate_portable_filename(&name.to_string_lossy())?;
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn is_standard_macos_alias(path: &Path, target: &Path) -> bool {
    [
        ("/var", "/private/var", "private/var"),
        ("/tmp", "/private/tmp", "private/tmp"),
        ("/etc", "/private/etc", "private/etc"),
    ]
    .iter()
    .any(|(alias, absolute, relative)| {
        path == Path::new(alias) && (target == Path::new(absolute) || target == Path::new(relative))
    })
}

pub(crate) fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Reject symlinks anywhere in a path before operating on user-controlled paths.
pub fn reject_symlink_components(path: &Path) -> CoreResult<()> {
    if path.components().any(|c| c == Component::ParentDir) {
        return Err(CoreError::InvalidInput(
            "parent-directory components are not allowed".into(),
        ));
    }
    let mut cursor = PathBuf::new();
    for component in path.components() {
        if component == Component::ParentDir {
            return Err(CoreError::InvalidInput(
                "parent-directory components are not allowed".into(),
            ));
        }
        cursor.push(component);
        // Windows verbatim prefixes (\\?\C:) are not valid standalone paths.
        // Only inspect complete normal components, after the volume/root prefix.
        if !matches!(component, Component::Normal(_)) {
            continue;
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if is_link(&meta) => {
                #[cfg(target_os = "macos")]
                if fs::read_link(&cursor)
                    .is_ok_and(|target| is_standard_macos_alias(&cursor, &target))
                {
                    continue;
                }
                return Err(CoreError::InvalidInput(
                    "symbolic links and reparse points are not allowed".into(),
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

pub fn validate_trash_path(path: &Path) -> CoreResult<PathBuf> {
    if !path.is_absolute() {
        return Err(CoreError::InvalidInput(
            "choose an absolute file or folder path".into(),
        ));
    }
    reject_symlink_components(path)?;
    let canonical = fs::canonicalize(path)?;
    if canonical.parent().is_none() {
        return Err(CoreError::InvalidInput(
            "disk roots cannot be moved to Trash".into(),
        ));
    }
    if let Some(home) = dirs::home_dir() {
        let home = fs::canonicalize(home)?;
        if home.starts_with(&canonical) {
            return Err(CoreError::InvalidInput(
                "your home folder and its parents are protected".into(),
            ));
        }
    }
    // Protect system trees, even when a caller bypasses the UI.
    let lower = canonical
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let lower = lower.strip_prefix("//?/").unwrap_or(&lower);
    #[cfg(target_os = "windows")]
    {
        let system_drive = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".into())
            .to_lowercase();
        for suffix in [
            "/windows",
            "/program files",
            "/program files (x86)",
            "/programdata",
            "/users",
        ] {
            let protected = format!("{system_drive}{suffix}");
            if lower == protected
                || (suffix != "/users" && lower.starts_with(&format!("{protected}/")))
            {
                return Err(CoreError::InvalidInput(
                    "system folders are protected".into(),
                ));
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // macOS resolves user temporary directories under /private/var/folders.
        let user_temp = fs::canonicalize(std::env::temp_dir()).ok();
        let inside_user_temp =
            user_temp.is_some_and(|temp| canonical.starts_with(&temp) && canonical != temp);
        for protected in [
            "/bin",
            "/boot",
            "/dev",
            "/etc",
            "/lib",
            "/lib64",
            "/proc",
            "/sbin",
            "/sys",
            "/usr",
            "/var",
            "/private/etc",
            "/private/var",
            "/system",
            "/library",
            "/applications",
        ] {
            if (lower == protected || lower.starts_with(&format!("{protected}/")))
                && !inside_user_temp
            {
                return Err(CoreError::InvalidInput(
                    "system folders are protected".into(),
                ));
            }
        }
    }
    Ok(canonical)
}

/// Preflight every path before sending any of them to the OS Trash API.
/// The caller must obtain an explicit confirmation first.
pub fn move_to_trash(paths: &[PathBuf]) -> CoreResult<usize> {
    move_to_trash_with(paths, |checked| {
        trash::delete_all(checked).map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))
    })
}

fn move_to_trash_with(
    paths: &[PathBuf],
    trash: impl FnOnce(&[PathBuf]) -> CoreResult<()>,
) -> CoreResult<usize> {
    if paths.is_empty() {
        return Ok(0);
    }
    let checked = paths
        .iter()
        .map(|p| validate_trash_path(p))
        .collect::<CoreResult<Vec<_>>>()?;
    trash(&checked)?;
    Ok(checked.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_output_paths_refuse_streams_devices_and_non_leaf_names() {
        for name in [
            "",
            ".",
            "..",
            "folder/file.txt",
            "folder\\file.txt",
            "base.txt:stream.age",
            "CON",
            "con.txt",
            "NUL .png",
            "CONIN$",
            "CONOUT$.wav",
            "COM1.mp4",
            "LPT9.txt",
            "COM¹.png",
            "LPT².wav",
            "COM³",
            "trailing.",
            "trailing ",
            "bad|name",
            "bad\0name",
        ] {
            assert!(validate_portable_filename(name).is_err(), "{name:?}");
        }
        for name in ["output.age", "COM10.mp4", "conference.png", "Résumé.webp"] {
            assert!(validate_portable_filename(name).is_ok(), "{name:?}");
            assert!(
                validate_output_filename(std::ffi::OsStr::new(name)).is_ok(),
                "{name:?}"
            );
        }
        #[cfg(windows)]
        assert!(
            validate_output_filename(std::ffi::OsStr::new("existing.bin:encrypted.age")).is_err()
        );
        #[cfg(not(windows))]
        assert!(validate_output_filename(std::ffi::OsStr::new("report:today.txt")).is_ok());
    }

    #[test]
    fn release_output_paths_allow_only_standard_macos_system_aliases() {
        for (path, target) in [
            ("/var", "private/var"),
            ("/tmp", "/private/tmp"),
            ("/etc", "private/etc"),
        ] {
            assert!(is_standard_macos_alias(Path::new(path), Path::new(target)));
        }
        for (path, target) in [
            ("/var", "/user/elsewhere"),
            ("/tmp", "/private/var"),
            ("/user/var", "/private/var"),
            ("/etc/other", "/private/etc"),
        ] {
            assert!(!is_standard_macos_alias(Path::new(path), Path::new(target)));
        }
    }

    #[test]
    fn refuses_roots_and_home() {
        let home = dirs::home_dir().unwrap();
        assert!(validate_trash_path(&home).is_err());
        let root = home.ancestors().last().unwrap();
        assert!(validate_trash_path(root).is_err());
    }

    #[test]
    fn accepts_only_existing_non_symlink_children() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        std::fs::write(&file, b"keep safe").unwrap();
        assert!(validate_trash_path(&file).is_ok());
        assert!(validate_trash_path(&temp.path().join("missing")).is_err());
    }

    #[test]
    fn invalid_batch_never_calls_the_trash_backend() {
        let temp = tempfile::tempdir().unwrap();
        let valid = temp.path().join("keep");
        std::fs::write(&valid, b"user data").unwrap();
        let paths = [valid.clone(), temp.path().join("missing")];
        let mut called = false;
        assert!(
            move_to_trash_with(&paths, |_| {
                called = true;
                Ok(())
            })
            .is_err()
        );
        assert!(!called);
        assert!(valid.exists());
        assert_eq!(
            move_to_trash_with(std::slice::from_ref(&valid), |checked| {
                assert_eq!(checked.len(), 1);
                Ok(())
            })
            .unwrap(),
            1
        );
    }

    #[test]
    fn parent_components_after_missing_directories_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        assert!(reject_symlink_components(&temp.path().join("missing/../escape")).is_err());
    }
}

//! Review registered uninstall commands and exact-name user-data candidates.
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: String,
    pub command: String,
    pub install_location: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Leftover {
    pub path: PathBuf,
    pub bytes: u64,
    pub files: u64,
}

/// Windows CRT quoting rules. Output is passed directly to Command::args, never a shell.
pub fn parse_command_line(line: &str) -> CoreResult<Vec<String>> {
    if line.len() > 32768 || line.contains('\0') {
        return Err(CoreError::InvalidInput("invalid uninstall command".into()));
    }
    let chars: Vec<char> = line.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i == chars.len() {
            break;
        }
        let mut argument = String::new();
        let mut quoted = false;
        while i < chars.len() {
            if !quoted && chars[i].is_whitespace() {
                break;
            }
            let mut slashes = 0;
            while i < chars.len() && chars[i] == '\\' {
                slashes += 1;
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                argument.extend(std::iter::repeat_n('\\', slashes / 2));
                if slashes % 2 == 1 {
                    argument.push('"');
                    i += 1;
                } else if quoted && i + 1 < chars.len() && chars[i + 1] == '"' {
                    argument.push('"');
                    i += 2;
                } else {
                    quoted = !quoted;
                    i += 1;
                }
            } else {
                argument.extend(std::iter::repeat_n('\\', slashes));
                if i < chars.len() && (quoted || !chars[i].is_whitespace()) {
                    argument.push(chars[i]);
                    i += 1;
                }
            }
        }
        if quoted {
            return Err(CoreError::InvalidInput(
                "uninstall command contains an unmatched quote".into(),
            ));
        }
        result.push(argument);
        if result.len() > 256 {
            return Err(CoreError::InvalidInput(
                "uninstall command has too many arguments".into(),
            ));
        }
    }
    if result.is_empty() || result[0].is_empty() {
        return Err(CoreError::InvalidInput(
            "the app has no usable uninstall command".into(),
        ));
    }
    Ok(result)
}
pub fn safe_folder_name(name: &str) -> bool {
    if crate::tools::archive::safe_entry_path(name).is_err() {
        return false;
    }
    !name.trim().is_empty()
        && name.len() <= 200
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
}
pub fn apps() -> CoreResult<Vec<InstalledApp>> {
    crate::platform::desktop::installed_apps()
}
pub fn uninstall(id: &str, reviewed_command: &str, confirmed: bool) -> CoreResult<u32> {
    if !confirmed {
        return Err(CoreError::InvalidInput(
            "confirm the exact registered uninstall command".into(),
        ));
    }
    let app = apps()?.into_iter().find(|a| a.id == id).ok_or_else(|| {
        CoreError::InvalidInput("this registered app has changed; refresh the list".into())
    })?;
    if app.command != reviewed_command {
        return Err(CoreError::InvalidInput(
            "the uninstall command changed; review it again".into(),
        ));
    }
    crate::platform::desktop::run_uninstaller(&app.command)
}
pub fn leftover_paths(name: &str, publisher: &str, home: &Path) -> CoreResult<Vec<PathBuf>> {
    if !safe_folder_name(name) {
        return Err(CoreError::InvalidInput(
            "this app name cannot safely identify a data folder".into(),
        ));
    }
    let mut result = vec![
        home.join("AppData/Local").join(name),
        home.join("AppData/Roaming").join(name),
    ];
    if safe_folder_name(publisher) {
        result.push(home.join("AppData/Local").join(publisher).join(name));
        result.push(home.join("AppData/Roaming").join(publisher).join(name));
    }
    Ok(result)
}
pub fn scan_leftovers(
    name: &str,
    publisher: &str,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<Leftover>> {
    if !cfg!(windows) {
        return Err(CoreError::Unsupported(
            crate::platform::Os::current().display_name(),
        ));
    }
    let home = dirs::home_dir()
        .ok_or_else(|| CoreError::InvalidInput("home folder is unavailable".into()))?;
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut done = 0_u64;
    for path in leftover_paths(name, publisher, &home)? {
        cancel.check()?;
        if !path.exists() {
            continue;
        }
        crate::fs_safe::reject_symlink_components(&path)?;
        let canonical = path.canonicalize()?;
        if !visited.insert(canonical.clone()) {
            continue;
        }
        crate::fs_safe::validate_trash_path(&canonical)?;
        let mut item = Leftover {
            path: canonical,
            bytes: 0,
            files: 0,
        };
        for entry in walkdir::WalkDir::new(&path).follow_links(false) {
            cancel.check()?;
            let entry = entry.map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
            if entry.file_type().is_symlink() {
                return Err(CoreError::InvalidInput(
                    "a candidate contains a symbolic link; it was excluded".into(),
                ));
            }
            crate::fs_safe::reject_symlink_components(entry.path())?;
            if entry.file_type().is_file() {
                let size = entry
                    .metadata()
                    .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?
                    .len();
                item.bytes = item.bytes.saturating_add(size);
                item.files += 1;
            }
            done += 1;
            if done > 200_000 {
                return Err(CoreError::InvalidInput(
                    "candidate scan exceeds 200,000 entries".into(),
                ));
            }
            progress(Progress {
                done,
                total: 0,
                message: "Scanning exact-name user-data candidates".into(),
            });
        }
        result.push(item);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_argument_parser_preserves_literals_without_a_shell() {
        let args = parse_command_line(
            "\"C:\\Program Files\\Vendor\\uninstall.exe\" /remove \"a b\" \"\" \"x&y\"",
        )
        .unwrap();
        assert_eq!(
            args,
            vec![
                "C:\\Program Files\\Vendor\\uninstall.exe",
                "/remove",
                "a b",
                "",
                "x&y"
            ]
        );
        assert!(parse_command_line("\"unterminated").is_err());
    }
    #[test]
    fn leftover_names_cannot_escape_user_data_directories() {
        for name in ["..", "a/b", "a\\b", "C:", "", "."] {
            assert!(!safe_folder_name(name));
        }
        assert!(safe_folder_name("Example App"));
    }
}

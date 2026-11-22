//! Explicit, read-only browser extension review. No preferences, cookies, history or credentials are edited.
use crate::{
    CoreError, CoreResult, fs_safe,
    jobs::{CancelToken, Progress},
    platform::Os,
};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Browser {
    Chrome,
    Chromium,
    Edge,
    Brave,
    Firefox,
}
impl Browser {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chrome => "Chrome",
            Self::Chromium => "Chromium",
            Self::Edge => "Edge",
            Self::Brave => "Brave",
            Self::Firefox => "Firefox",
        }
    }
    pub fn settings(self) -> &'static str {
        match self {
            Self::Chrome | Self::Chromium => "chrome://extensions/",
            Self::Edge => "edge://extensions/",
            Self::Brave => "brave://extensions/",
            Self::Firefox => "about:addons",
        }
    }
}
#[derive(Debug, Clone)]
pub struct BrowserRoot {
    pub browser: Browser,
    pub path: PathBuf,
}
#[derive(Debug, Clone, Serialize)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub permissions: Vec<String>,
    pub enabled: Option<bool>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ProfileReview {
    pub browser: Browser,
    pub path: PathBuf,
    pub extensions: Vec<Extension>,
}
#[derive(Debug, Default, Clone, Serialize)]
pub struct BrowserReport {
    pub profiles: Vec<ProfileReview>,
    pub skipped: usize,
    pub limited: bool,
    pub metadata_files: usize,
    pub visited_entries: usize,
}
pub fn known_roots() -> Vec<BrowserRoot> {
    let mut roots = Vec::new();
    let home = dirs::home_dir();
    let base = match Os::current() {
        Os::Windows => dirs::data_local_dir(),
        Os::Macos => home.as_ref().map(|h| h.join("Library/Application Support")),
        Os::Linux => dirs::config_dir(),
        _ => None,
    };
    if let Some(base) = base {
        let candidates = match Os::current() {
            Os::Windows => vec![
                (Browser::Chrome, "Google/Chrome/User Data"),
                (Browser::Chromium, "Chromium/User Data"),
                (Browser::Edge, "Microsoft/Edge/User Data"),
                (Browser::Brave, "BraveSoftware/Brave-Browser/User Data"),
            ],
            Os::Macos => vec![
                (Browser::Chrome, "Google/Chrome"),
                (Browser::Chromium, "Chromium"),
                (Browser::Edge, "Microsoft Edge"),
                (Browser::Brave, "BraveSoftware/Brave-Browser"),
            ],
            _ => vec![
                (Browser::Chrome, "google-chrome"),
                (Browser::Chromium, "chromium"),
                (Browser::Edge, "microsoft-edge"),
                (Browser::Brave, "BraveSoftware/Brave-Browser"),
            ],
        };
        for (browser, relative) in candidates {
            roots.push(BrowserRoot {
                browser,
                path: base.join(relative),
            });
        }
    }
    let firefox = match Os::current() {
        Os::Windows => dirs::data_dir().map(|p| p.join("Mozilla/Firefox/Profiles")),
        Os::Macos => home
            .as_ref()
            .map(|p| p.join("Library/Application Support/Firefox/Profiles")),
        Os::Linux => home.map(|p| p.join(".mozilla/firefox")),
        _ => None,
    };
    if let Some(path) = firefox {
        roots.push(BrowserRoot {
            browser: Browser::Firefox,
            path,
        });
    }
    roots
}
fn local(path: &Path) -> CoreResult<()> {
    if !path.is_absolute()
        || path.to_string_lossy().starts_with("\\\\")
        || path
            .components()
            .any(|p| matches!(p, std::path::Component::ParentDir))
    {
        return Err(CoreError::InvalidInput(
            "Choose an absolute local browser folder.".into(),
        ));
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
                _ => {
                    return Err(CoreError::InvalidInput(
                        "Network and device browser folders are not supported.".into(),
                    ));
                }
            },
            _ => {
                return Err(CoreError::InvalidInput(
                    "Choose a local drive folder.".into(),
                ));
            }
        };
        let root = [u16::from(drive), 58, 92, 0];
        if !matches!(
            unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) },
            2 | 3 | 6
        ) {
            return Err(CoreError::InvalidInput(
                "Choose a folder on a local drive.".into(),
            ));
        }
    }
    fs_safe::reject_symlink_components(path)?;
    Ok(())
}
fn read_json(path: &Path) -> CoreResult<serde_json::Value> {
    local(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(CoreError::InvalidInput(
            "Browser extension metadata exceeds the bounded review limit.".into(),
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(CoreError::InvalidInput(
            "Browser extension metadata grew beyond its limit.".into(),
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| CoreError::InvalidInput("Browser extension metadata is invalid.".into()))
}
fn string(value: &serde_json::Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(fallback)
        .chars()
        .filter(|c| !c.is_control())
        .take(256)
        .collect()
}
fn directories(path: &Path, limit: usize, report: &mut BrowserReport) -> Vec<PathBuf> {
    if local(path).is_err() {
        report.skipped += 1;
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for (index, entry) in entries.take(limit + 1).enumerate() {
        if index >= limit || report.visited_entries >= 8192 {
            report.limited = true;
            break;
        }
        report.visited_entries += 1;
        match entry {
            Ok(entry) => match entry.file_type() {
                Ok(kind) if kind.is_dir() && !kind.is_symlink() => result.push(entry.path()),
                _ => report.skipped += 1,
            },
            Err(_) => report.skipped += 1,
        }
    }
    result.sort();
    result
}
pub fn scan_roots(
    roots: &[BrowserRoot],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<BrowserReport> {
    cancel.check()?;
    if roots.len() > 16 {
        return Err(CoreError::InvalidInput(
            "At most 16 browser roots can be reviewed.".into(),
        ));
    }
    let mut report = BrowserReport::default();
    for (index, root) in roots.iter().enumerate() {
        cancel.check()?;
        for profile in directories(&root.path, 32, &mut report) {
            cancel.check()?;
            if report.metadata_files >= 512 {
                report.limited = true;
                return Ok(report);
            }
            let name = profile.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(root.browser, Browser::Firefox)
                && name != "Default"
                && !name
                    .strip_prefix("Profile ")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
            {
                continue;
            }
            if matches!(root.browser, Browser::Firefox) && !profile.join("extensions.json").exists()
            {
                continue;
            }
            let mut extensions = Vec::new();
            if matches!(root.browser, Browser::Firefox) {
                report.metadata_files += 1;
                match read_json(&profile.join("extensions.json")) {
                    Ok(value) => {
                        if let Some(addons) = value.get("addons").and_then(|v| v.as_array()) {
                            if addons.len() > 256 {
                                report.limited = true;
                            }
                            for addon in addons.iter().take(256) {
                                cancel.check()?;
                                let locale = addon.get("defaultLocale").unwrap_or(addon);
                                extensions.push(Extension {
                                    id: string(addon, "id", "unknown"),
                                    name: string(locale, "name", "Unnamed extension"),
                                    version: string(addon, "version", "unknown"),
                                    permissions: Vec::new(),
                                    enabled: addon.get("active").and_then(|v| v.as_bool()),
                                });
                            }
                        }
                    }
                    Err(_) => report.skipped += 1,
                }
            } else {
                for extension in directories(&profile.join("Extensions"), 256, &mut report) {
                    cancel.check()?;
                    let id = extension.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if id.len() != 32 || !id.bytes().all(|c| (b'a'..=b'p').contains(&c)) {
                        report.skipped += 1;
                        continue;
                    }
                    for version in directories(&extension, 16, &mut report) {
                        cancel.check()?;
                        if report.metadata_files >= 512 {
                            report.limited = true;
                            report.profiles.push(ProfileReview {
                                browser: root.browser,
                                path: profile.clone(),
                                extensions,
                            });
                            return Ok(report);
                        }
                        report.metadata_files += 1;
                        match read_json(&version.join("manifest.json")) {
                            Ok(value) => {
                                let permissions = value
                                    .get("permissions")
                                    .and_then(|v| v.as_array())
                                    .into_iter()
                                    .flatten()
                                    .filter_map(|v| v.as_str())
                                    .take(32)
                                    .map(|s| s.chars().take(128).collect())
                                    .collect();
                                let mut label = string(&value, "name", "Unnamed extension");
                                if label.starts_with("__MSG_") {
                                    label = "Localized extension (check name in browser)".into();
                                }
                                extensions.push(Extension {
                                    id: id.into(),
                                    name: label,
                                    version: string(&value, "version", "unknown"),
                                    permissions,
                                    enabled: None,
                                });
                            }
                            Err(_) => report.skipped += 1,
                        }
                    }
                }
            }
            report.profiles.push(ProfileReview {
                browser: root.browser,
                path: profile,
                extensions,
            });
        }
        progress(Progress {
            done: (index + 1) as u64,
            total: roots.len() as u64,
            message: "Reviewing local extension metadata".into(),
        });
    }
    cancel.check()?;
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_review_reads_only_bounded_extension_manifests_and_reports_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("Default");
        let version = profile.join("Extensions").join("a".repeat(32)).join("1.0");
        std::fs::create_dir_all(&version).unwrap();
        let original = r#"{"name":"Generated fixture","version":"1.0","permissions":["storage","tabs"],"background":{"scripts":["do-not-run.js"]}}"#;
        std::fs::write(version.join("manifest.json"), original).unwrap();
        let roots = [BrowserRoot {
            browser: Browser::Chrome,
            path: temp.path().into(),
        }];
        let report = scan_roots(&roots, &crate::jobs::CancelToken::default(), &mut |_| {}).unwrap();
        assert_eq!(report.profiles.len(), 1);
        assert_eq!(report.profiles[0].extensions.len(), 1);
        assert_eq!(report.profiles[0].extensions[0].name, "Generated fixture");
        assert_eq!(
            report.profiles[0].extensions[0].permissions,
            vec!["storage", "tabs"]
        );
        assert_eq!(
            std::fs::read_to_string(version.join("manifest.json")).unwrap(),
            original
        );
    }
    #[test]
    fn browser_review_cancels_and_never_interprets_bookmark_or_password_files() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("Default")).unwrap();
        std::fs::write(
            temp.path().join("Default").join("Login Data"),
            b"not extension metadata",
        )
        .unwrap();
        let roots = [BrowserRoot {
            browser: Browser::Chrome,
            path: temp.path().into(),
        }];
        let cancel = crate::jobs::CancelToken::default();
        cancel.cancel();
        assert_eq!(
            scan_roots(&roots, &cancel, &mut |_| {}).unwrap_err().code(),
            "cancelled"
        );
        assert!(
            scan_roots(&roots, &crate::jobs::CancelToken::default(), &mut |_| {})
                .unwrap()
                .profiles[0]
                .extensions
                .is_empty()
        );
    }
}

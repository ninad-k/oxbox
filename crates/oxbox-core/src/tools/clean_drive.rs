//! Tool: **Clean Drive** (`clean-drive`).

use crate::{
    CoreError, CoreResult,
    fs_safe::{move_to_trash, reject_symlink_components},
    jobs::{CancelToken, Progress},
    platform::Os,
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct CleanRoot {
    pub id: &'static str,
    pub name: &'static str,
    pub path: PathBuf,
    pub older_than_day: bool,
    pub read_only: bool,
    pub file_prefix: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanCategory {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub files: u64,
    pub skipped: u64,
    pub read_only: bool,
    #[serde(skip)]
    pub candidates: Vec<PathBuf>,
}

/// Pure allow-list. No arbitrary paths supplied by the frontend are accepted.
pub fn locations(os: Os, home: &Path) -> Vec<CleanRoot> {
    let specs: &[(&str, &str, &str, bool)] = match os {
        Os::Windows => &[
            (
                "temporary",
                "User temporary files (older than 24 hours)",
                "AppData/Local/Temp",
                true,
            ),
            (
                "browser-cache",
                "Microsoft Edge cache",
                "AppData/Local/Microsoft/Edge/User Data/Default/Cache",
                false,
            ),
            (
                "chrome-cache",
                "Google Chrome cache",
                "AppData/Local/Google/Chrome/User Data/Default/Cache",
                false,
            ),
            (
                "thumbnails",
                "Windows thumbnail cache",
                "AppData/Local/Microsoft/Windows/Explorer",
                false,
            ),
            (
                "package-cache",
                "npm cache",
                "AppData/Local/npm-cache/_cacache",
                false,
            ),
        ],
        Os::Macos => &[
            (
                "temporary",
                "User temporary cache (older than 24 hours)",
                "Library/Caches/TemporaryItems",
                true,
            ),
            (
                "browser-cache",
                "Google Chrome cache",
                "Library/Caches/Google/Chrome/Default/Cache",
                false,
            ),
            (
                "safari-cache",
                "Safari cache",
                "Library/Caches/com.apple.Safari",
                false,
            ),
            ("package-cache", "npm cache", ".npm/_cacache", false),
            ("recycle-bin", "Trash (size only)", ".Trash", false),
        ],
        Os::Linux => &[
            (
                "temporary",
                "User temporary cache (older than 24 hours)",
                ".cache/tmp",
                true,
            ),
            (
                "browser-cache",
                "Chromium cache",
                ".cache/chromium/Default/Cache",
                false,
            ),
            (
                "chrome-cache",
                "Google Chrome cache",
                ".cache/google-chrome/Default/Cache",
                false,
            ),
            ("thumbnails", "Thumbnail cache", ".cache/thumbnails", false),
            ("package-cache", "npm cache", ".npm/_cacache", false),
            (
                "recycle-bin",
                "Trash (size only)",
                ".local/share/Trash/files",
                false,
            ),
        ],
        Os::Other => &[],
    };
    specs
        .iter()
        .map(|&(id, name, path, older_than_day)| CleanRoot {
            id,
            name,
            path: home.join(path),
            older_than_day,
            read_only: id == "recycle-bin",
            file_prefix: (os == Os::Windows && id == "thumbnails").then_some("thumbcache_"),
        })
        .collect()
}

pub fn current_locations() -> CoreResult<Vec<CleanRoot>> {
    let home = dirs::home_dir()
        .ok_or_else(|| CoreError::InvalidInput("cannot find your home folder".into()))?;
    Ok(locations(Os::current(), &home))
}

pub fn scan_roots(
    roots: &[CleanRoot],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<CleanCategory>> {
    let mut categories = Vec::new();
    let now = SystemTime::now();
    for (index, root) in roots.iter().enumerate() {
        cancel.check()?;
        let mut category = CleanCategory {
            id: root.id.into(),
            name: root.name.into(),
            path: root.path.clone(),
            bytes: 0,
            files: 0,
            skipped: 0,
            read_only: root.read_only,
            candidates: Vec::new(),
        };
        if !root.path.exists() {
            categories.push(category);
            continue;
        }
        // A cache root redirected to another location is never eligible.
        if reject_symlink_components(&root.path).is_err() {
            category.skipped = 1;
            categories.push(category);
            continue;
        }
        for entry in WalkDir::new(&root.path)
            .min_depth(1)
            .max_depth(if root.file_prefix.is_some() {
                1
            } else {
                usize::MAX
            })
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0
                    || fs::symlink_metadata(entry.path())
                        .is_ok_and(|metadata| !crate::fs_safe::is_link(&metadata))
            })
        {
            cancel.check()?;
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    category.skipped += 1;
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            if root.file_prefix.is_some_and(|prefix| {
                !entry.file_name().to_string_lossy().starts_with(prefix)
                    || entry
                        .path()
                        .extension()
                        .is_none_or(|extension| extension != "db")
            }) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(_) => {
                    category.skipped += 1;
                    continue;
                }
            };
            if root.older_than_day
                && metadata
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .is_none_or(|age| age < Duration::from_secs(24 * 60 * 60))
            {
                continue;
            }
            category.bytes = category.bytes.saturating_add(metadata.len());
            category.files += 1;
            if !root.read_only {
                category.candidates.push(entry.path().to_path_buf());
            }
            progress(Progress {
                done: index as u64,
                total: roots.len() as u64,
                message: format!("Scanning {} ({} files)", root.name, category.files),
            });
        }
        categories.push(category);
    }
    progress(Progress {
        done: roots.len() as u64,
        total: roots.len() as u64,
        message: "Scan complete; review before cleaning".into(),
    });
    Ok(categories)
}

pub fn scan_current(
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<CleanCategory>> {
    let categories = scan_roots(&current_locations()?, cancel, progress)?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::{SHQUERYRBINFO, SHQueryRecycleBinW};
        let mut categories = categories;
        let mut info = SHQUERYRBINFO {
            cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
            i64Size: 0,
            i64NumItems: 0,
        };
        // A null root queries the current user's bins across all drives. No mutation.
        let succeeded = unsafe { SHQueryRecycleBinW(std::ptr::null(), &mut info) } >= 0;
        categories.push(CleanCategory {
            id: "recycle-bin".into(),
            name: "Recycle Bin (size only, all drives)".into(),
            path: PathBuf::from("Windows Recycle Bin"),
            bytes: if succeeded {
                info.i64Size.max(0) as u64
            } else {
                0
            },
            files: if succeeded {
                info.i64NumItems.max(0) as u64
            } else {
                0
            },
            skipped: u64::from(!succeeded),
            read_only: true,
            candidates: Vec::new(),
        });
        Ok(categories)
    }
    #[cfg(not(windows))]
    {
        Ok(categories)
    }
}

/// Re-scan only known category identifiers immediately before trashing. The
/// confirmation budget prevents a later scan from deleting more than reviewed.
pub fn clean_categories(
    ids: &[String],
    max_files: u64,
    max_bytes: u64,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<usize> {
    let all_roots = current_locations()?;
    if ids.is_empty()
        || ids
            .iter()
            .any(|id| !all_roots.iter().any(|r| r.id == id && !r.read_only))
    {
        return Err(CoreError::InvalidInput(
            "select known, cleanable cache categories".into(),
        ));
    }
    let roots: Vec<_> = all_roots
        .into_iter()
        .filter(|r| ids.iter().any(|id| id == r.id))
        .collect();
    let categories = scan_roots(&roots, cancel, progress)?;
    let files: u64 = categories.iter().map(|c| c.files).sum();
    let bytes: u64 = categories.iter().map(|c| c.bytes).sum();
    if files > max_files || bytes > max_bytes {
        return Err(CoreError::InvalidInput(
            "cache contents changed; scan and confirm again".into(),
        ));
    }
    let candidates: Vec<_> = categories.into_iter().flat_map(|c| c.candidates).collect();
    // Validate the entire allow-list membership before starting any operation.
    for path in &candidates {
        cancel.check()?;
        reject_symlink_components(path)?;
        let canonical = fs::canonicalize(path)?;
        let permitted = roots.iter().any(|root| {
            fs::canonicalize(&root.path).is_ok_and(|p| canonical.starts_with(&p) && canonical != p)
        });
        if !permitted {
            return Err(CoreError::InvalidInput(
                "cache path is outside the allow-list".into(),
            ));
        }
        crate::fs_safe::validate_trash_path(path)?;
    }
    let mut count = 0;
    for path in candidates {
        cancel.check()?;
        move_to_trash(&[path])?;
        count += 1;
        progress(Progress {
            done: count as u64,
            total: files,
            message: "Moving reviewed cache files to Trash".into(),
        });
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_remain_under_the_supplied_home() {
        let temp = tempfile::tempdir().unwrap();
        for os in [Os::Windows, Os::Macos, Os::Linux] {
            let roots = locations(os, temp.path());
            assert!(!roots.is_empty());
            assert!(
                roots
                    .iter()
                    .all(|r| r.path.starts_with(temp.path()) && r.path != temp.path())
            );
        }
    }

    #[test]
    fn fake_cache_scan_never_includes_unlisted_siblings() {
        let temp = tempfile::tempdir().unwrap();
        let roots = locations(Os::Linux, temp.path());
        let cache = roots.iter().find(|r| r.id == "browser-cache").unwrap();
        std::fs::create_dir_all(&cache.path).unwrap();
        std::fs::write(cache.path.join("cached"), b"12345").unwrap();
        std::fs::write(temp.path().join("important"), b"123456789").unwrap();
        let result = scan_roots(&roots, &CancelToken::default(), &mut |_| {}).unwrap();
        assert_eq!(result.iter().map(|r| r.bytes).sum::<u64>(), 5);
    }

    #[test]
    fn windows_thumbnail_category_only_includes_known_database_files() {
        let temp = tempfile::tempdir().unwrap();
        let roots = locations(Os::Windows, temp.path());
        let root = roots.iter().find(|r| r.id == "thumbnails").unwrap();
        std::fs::create_dir_all(&root.path).unwrap();
        for name in [
            "thumbcache_96.db",
            "iconcache_96.db",
            "user-document.txt",
            "thumbcache_not-a-database.txt",
        ] {
            std::fs::write(root.path.join(name), b"12345").unwrap();
        }
        let result = scan_roots(&roots, &CancelToken::default(), &mut |_| {}).unwrap();
        let scanned = result.iter().find(|r| r.id == "thumbnails").unwrap();
        assert_eq!(scanned.files, 1);
        assert_eq!(scanned.bytes, 5);
    }

    #[test]
    fn trash_categories_are_never_cleanup_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let roots = locations(Os::Linux, temp.path());
        let root = roots.iter().find(|r| r.id == "recycle-bin").unwrap();
        std::fs::create_dir_all(&root.path).unwrap();
        std::fs::write(root.path.join("trashed"), b"12345").unwrap();
        let result = scan_roots(&roots, &CancelToken::default(), &mut |_| {}).unwrap();
        let scanned = result.iter().find(|r| r.id == "recycle-bin").unwrap();
        assert!(scanned.read_only);
        assert_eq!(scanned.bytes, 5);
        assert!(scanned.candidates.is_empty());
    }
}

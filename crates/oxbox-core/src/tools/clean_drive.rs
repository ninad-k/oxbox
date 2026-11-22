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
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub older_than_day: bool,
    pub read_only: bool,
    pub file_prefix: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScanBudget {
    pub max_entries_per_category: u64,
    pub max_depth: usize,
}
impl Default for ScanBudget {
    fn default() -> Self {
        Self {
            max_entries_per_category: 50_000,
            max_depth: 20,
        }
    }
}
#[derive(Debug, Clone)]
struct CandidateStamp {
    path: PathBuf,
    bytes: u64,
    modified: Option<SystemTime>,
    accessed: Option<SystemTime>,
    identity: CacheIdentity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheIdentity {
    device: u64,
    file: u64,
}
fn cache_metadata(path: &Path) -> CoreResult<(fs::Metadata, CacheIdentity)> {
    #[cfg(windows)]
    {
        use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
        };
        let file = fs::OpenOptions::new()
            .read(true)
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || crate::fs_safe::is_link(&metadata) {
            return Err(CoreError::InvalidInput(
                "Only regular non-link cache files can be reviewed.".into(),
            ));
        }
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        // SAFETY: the handle is owned and the typed output storage is writable.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let identity = CacheIdentity {
            device: u64::from(info.dwVolumeSerialNumber),
            file: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        };
        if identity.device == 0 || identity.file == 0 {
            return Err(CoreError::InvalidInput(
                "The cache filesystem does not provide a usable file identity.".into(),
            ));
        }
        Ok((metadata, identity))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(path)?;
        let identity = CacheIdentity {
            device: metadata.dev(),
            file: metadata.ino(),
        };
        if !metadata.is_file()
            || crate::fs_safe::is_link(&metadata)
            || identity.device == 0
            || identity.file == 0
        {
            return Err(CoreError::InvalidInput(
                "The cache path has no usable regular-file identity.".into(),
            ));
        }
        Ok((metadata, identity))
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        Err(CoreError::Unsupported(
            "cache file identity on this platform",
        ))
    }
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
    pub limited: bool,
    #[serde(skip)]
    pub candidates: Vec<PathBuf>,
    #[serde(skip)]
    stamps: Vec<CandidateStamp>,
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
                true,
            ),
            (
                "chrome-cache",
                "Google Chrome cache",
                "AppData/Local/Google/Chrome/User Data/Default/Cache",
                true,
            ),
            (
                "thumbnails",
                "Windows thumbnail cache",
                "AppData/Local/Microsoft/Windows/Explorer",
                true,
            ),
            (
                "package-cache",
                "npm cache",
                "AppData/Local/npm-cache/_cacache",
                true,
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
                true,
            ),
            (
                "safari-cache",
                "Safari cache",
                "Library/Caches/com.apple.Safari",
                true,
            ),
            ("package-cache", "npm cache", ".npm/_cacache", true),
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
                true,
            ),
            (
                "chrome-cache",
                "Google Chrome cache",
                ".cache/google-chrome/Default/Cache",
                true,
            ),
            ("thumbnails", "Thumbnail cache", ".cache/thumbnails", true),
            ("package-cache", "npm cache", ".npm/_cacache", true),
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
            id: id.into(),
            name: name.into(),
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
    let os = Os::current();
    let mut roots = locations(os, &home);
    discover_browser_caches(os, &home, &mut roots)?;
    if os == Os::Windows {
        roots.push(CleanRoot {
            id: "shader-cache".into(),
            name: "Direct3D shader cache (older than 24 hours)".into(),
            path: home.join("AppData/Local/D3DSCache"),
            older_than_day: true,
            read_only: false,
            file_prefix: None,
        });
    }
    if os == Os::Linux {
        roots.push(CleanRoot {
            id: "shader-cache".into(),
            name: "Mesa shader cache (older than 24 hours)".into(),
            path: home.join(".cache/mesa_shader_cache"),
            older_than_day: true,
            read_only: false,
            file_prefix: None,
        });
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let temporary = std::env::temp_dir();
        if reject_symlink_components(&temporary).is_ok()
            && fs::metadata(&temporary)
                .is_ok_and(|m| m.is_dir() && m.uid() == unsafe { libc::geteuid() })
        {
            let canonical = temporary.canonicalize()?;
            if canonical.starts_with("/private/var/folders") {
                roots.push(CleanRoot {
                    id: "user-temp".into(),
                    name: "Current-user temporary files (older than 24 hours)".into(),
                    path: canonical,
                    older_than_day: true,
                    read_only: false,
                    file_prefix: None,
                });
            }
        }
    }
    Ok(roots)
}

fn discover_browser_caches(os: Os, home: &Path, roots: &mut Vec<CleanRoot>) -> CoreResult<()> {
    let specs: &[(&str, &str)] = match os {
        Os::Windows => &[
            ("Edge", "AppData/Local/Microsoft/Edge/User Data"),
            ("Chrome", "AppData/Local/Google/Chrome/User Data"),
            ("Chromium", "AppData/Local/Chromium/User Data"),
            ("Firefox", "AppData/Local/Mozilla/Firefox/Profiles"),
        ],
        Os::Macos => &[
            ("Edge", "Library/Caches/Microsoft Edge"),
            ("Chrome", "Library/Caches/Google/Chrome"),
            ("Chromium", "Library/Caches/Chromium"),
            ("Firefox", "Library/Caches/Firefox/Profiles"),
        ],
        Os::Linux => &[
            ("Chrome", ".cache/google-chrome"),
            ("Chromium", ".cache/chromium"),
            ("Edge", ".cache/microsoft-edge"),
            ("Firefox", ".cache/mozilla/firefox"),
        ],
        _ => &[],
    };
    for &(browser, relative) in specs {
        let base = home.join(relative);
        if super::maintenance_support::local_path(&base).is_err() {
            continue;
        }
        let reader = match fs::read_dir(&base) {
            Ok(reader) => reader,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        };
        for (index, child) in reader.enumerate() {
            if index >= 128 {
                return Err(CoreError::InvalidInput(
                    "Browser cache profile discovery exceeds 128 entries; no cleanup ran.".into(),
                ));
            }
            let child = match child {
                Ok(child) => child,
                Err(_) => continue,
            };
            if !child.file_type().is_ok_and(|kind| kind.is_dir())
                || reject_symlink_components(&child.path()).is_err()
            {
                continue;
            }
            let name = child.file_name().to_string_lossy().into_owned();
            if name.len() > 200
                || crate::fs_safe::validate_portable_filename(&name).is_err()
                || (browser != "Firefox"
                    && name != "Default"
                    && !name
                        .strip_prefix("Profile ")
                        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())))
            {
                continue;
            }
            let cache_leaves: &[&str] = if browser == "Firefox" {
                &["cache2/entries"]
            } else {
                &["Cache", "Code Cache", "GPUCache"]
            };
            for (cache_index, leaf) in cache_leaves.iter().enumerate() {
                let path = child.path().join(leaf);
                if roots.iter().any(|root| root.path == path) {
                    continue;
                }
                let profile = blake3::hash(name.as_bytes()).to_hex();
                roots.push(CleanRoot {
                    id: format!(
                        "browser-{}-{}-{cache_index}",
                        browser.to_lowercase(),
                        &profile[..16]
                    ),
                    name: format!("{browser} {name}: {leaf} (older than 24 hours)"),
                    path,
                    older_than_day: true,
                    read_only: false,
                    file_prefix: None,
                });
            }
        }
    }
    Ok(())
}

fn age_eligible(
    modified: Option<SystemTime>,
    accessed: Option<SystemTime>,
    now: SystemTime,
    minimum: Duration,
) -> bool {
    modified
        .into_iter()
        .chain(accessed)
        .max()
        .and_then(|last| now.duration_since(last).ok())
        .is_some_and(|age| age >= minimum)
}

pub fn scan_roots(
    roots: &[CleanRoot],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<CleanCategory>> {
    scan_roots_with_budget(roots, ScanBudget::default(), cancel, progress)
}
pub fn scan_roots_with_budget(
    roots: &[CleanRoot],
    budget: ScanBudget,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<CleanCategory>> {
    if !(1..=200_000).contains(&budget.max_entries_per_category)
        || !(1..=64).contains(&budget.max_depth)
        || roots.len() > 1024
    {
        return Err(CoreError::InvalidInput(
            "Choose bounded cleanup categories (1–200,000 entries and depth 1–64).".into(),
        ));
    }
    let mut categories = Vec::new();
    let now = SystemTime::now();
    for (index, root) in roots.iter().enumerate() {
        cancel.check()?;
        let mut category = CleanCategory {
            id: root.id.clone(),
            name: root.name.clone(),
            path: root.path.clone(),
            bytes: 0,
            files: 0,
            skipped: 0,
            read_only: root.read_only,
            limited: false,
            candidates: Vec::new(),
            stamps: Vec::new(),
        };
        if super::maintenance_support::local_path(&root.path).is_err() {
            category.skipped = 1;
            categories.push(category);
            continue;
        }
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
        let mut visited = 0_u64;
        let mut walk = WalkDir::new(&root.path)
            .min_depth(1)
            .max_depth(if root.file_prefix.is_some() {
                1
            } else {
                budget.max_depth
            })
            .follow_links(false)
            .into_iter();
        while let Some(entry) = walk.next() {
            cancel.check()?;
            if visited >= budget.max_entries_per_category {
                category.limited = true;
                break;
            }
            visited += 1;
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    category.skipped += 1;
                    continue;
                }
            };
            if !fs::symlink_metadata(entry.path())
                .is_ok_and(|metadata| !crate::fs_safe::is_link(&metadata))
                || reject_symlink_components(entry.path()).is_err()
            {
                if entry.file_type().is_dir() {
                    walk.skip_current_dir();
                }
                category.skipped += 1;
                continue;
            }
            if !entry.file_type().is_file() {
                if entry.file_type().is_dir() && entry.depth() == budget.max_depth {
                    category.limited = true;
                }
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
            let (metadata, identity) = match cache_metadata(entry.path()) {
                Ok(value) => value,
                Err(_) => {
                    category.skipped += 1;
                    continue;
                }
            };
            if root.older_than_day
                && !age_eligible(
                    metadata.modified().ok(),
                    metadata.accessed().ok(),
                    now,
                    Duration::from_secs(86400),
                )
            {
                continue;
            }
            category.bytes = category.bytes.saturating_add(metadata.len());
            category.files += 1;
            if !root.read_only {
                category.candidates.push(entry.path().to_path_buf());
                category.stamps.push(CandidateStamp {
                    path: entry.path().to_path_buf(),
                    bytes: metadata.len(),
                    modified: metadata.modified().ok(),
                    accessed: metadata.accessed().ok(),
                    identity,
                });
            }
            progress(Progress {
                done: index as u64,
                total: roots.len() as u64,
                message: format!("Scanning {} ({} files)", root.name, category.files),
            });
        }
        if category.limited {
            category.read_only = true;
            category.candidates.clear();
            category.stamps.clear();
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

fn category_browser(os: Os, id: &str) -> Option<&'static str> {
    match id {
        "browser-cache" => match os {
            Os::Windows => Some("edge"),
            Os::Macos => Some("chrome"),
            Os::Linux => Some("chromium"),
            _ => None,
        },
        "chrome-cache" => Some("chrome"),
        "safari-cache" => Some("safari"),
        _ => ["chrome", "chromium", "edge", "firefox"]
            .into_iter()
            .find(|browser| id.starts_with(&format!("browser-{browser}-"))),
    }
}
fn browser_is_running(browser: &str, names: &[String]) -> bool {
    names.iter().any(|name| {
        let name = name.to_ascii_lowercase();
        let name = name.strip_suffix(".exe").unwrap_or(&name);
        match browser {
            "chrome" => {
                matches!(
                    name,
                    "chrome" | "google chrome" | "google-chrome" | "google-chrome-stable"
                ) || name.starts_with("google chrome helper")
            }
            "chromium" => {
                matches!(name, "chromium" | "chromium-browser")
                    || name.starts_with("chromium helper")
            }
            "edge" => {
                matches!(name, "msedge" | "microsoft edge")
                    || name.starts_with("microsoft edge helper")
            }
            "firefox" => matches!(name, "firefox" | "firefox-bin" | "firefox content"),
            "safari" => matches!(name, "safari" | "safari web content"),
            _ => false,
        }
    })
}
fn apply_active_browser_guard(os: Os, categories: &mut [CleanCategory], names: &[String]) {
    for category in categories {
        if category_browser(os, &category.id)
            .is_some_and(|browser| browser_is_running(browser, names))
        {
            category.read_only = true;
            category.candidates.clear();
            category.stamps.clear();
        }
    }
}
fn running_process_names(system: &mut sysinfo::System) -> CoreResult<Vec<String>> {
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    if system.processes().is_empty() {
        return Err(CoreError::InvalidInput(
            "Running browser inspection is unavailable; no cache cleanup was permitted.".into(),
        ));
    }
    Ok(system
        .processes()
        .values()
        .map(|process| process.name().to_string_lossy().into_owned())
        .collect())
}
fn check_browser_activity(
    root: &CleanRoot,
    os: Os,
    activity: &mut dyn FnMut() -> CoreResult<Vec<String>>,
) -> CoreResult<()> {
    if let Some(browser) = category_browser(os, &root.id)
        && browser_is_running(browser, &activity()?)
    {
        return Err(CoreError::InvalidInput(format!(
            "A recognized {browser} process is running. Close the browser and scan again; remaining cache requests were refused."
        )));
    }
    Ok(())
}
fn clean_current_review(
    roots: &[CleanRoot],
    categories: &[CleanCategory],
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<usize> {
    let mut system = sysinfo::System::new();
    clean_reviewed_with_activity(
        roots,
        categories,
        confirmed,
        cancel,
        progress,
        &mut |path| move_to_trash(&[path.to_path_buf()]).map(|_| ()),
        (Os::current(), &mut || running_process_names(&mut system)),
    )
}

pub fn scan_current(
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<CleanCategory>> {
    let mut categories = scan_roots(&current_locations()?, cancel, progress)?;
    cancel.check()?;
    apply_active_browser_guard(
        Os::current(),
        &mut categories,
        &running_process_names(&mut sysinfo::System::new())?,
    );
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
            limited: false,
            candidates: Vec::new(),
            stamps: Vec::new(),
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
        || ids.iter().any(|id| {
            !all_roots
                .iter()
                .any(|r| r.id.as_str() == id.as_str() && !r.read_only)
        })
    {
        return Err(CoreError::InvalidInput(
            "select known, cleanable cache categories".into(),
        ));
    }
    let roots: Vec<_> = all_roots
        .into_iter()
        .filter(|r| ids.iter().any(|id| id.as_str() == r.id.as_str()))
        .collect();
    let categories = scan_roots(&roots, cancel, progress)?;
    let files: u64 = categories.iter().map(|c| c.files).sum();
    let bytes: u64 = categories.iter().map(|c| c.bytes).sum();
    if files > max_files || bytes > max_bytes {
        return Err(CoreError::InvalidInput(
            "cache contents changed; scan and confirm again".into(),
        ));
    }
    clean_current_review(&roots, &categories, true, cancel, progress)
}

fn check_reviewed_stamp(
    root: &CleanRoot,
    stamp: &CandidateStamp,
    now: SystemTime,
) -> CoreResult<()> {
    super::maintenance_support::local_path(&stamp.path)?;
    let (metadata, identity) = cache_metadata(&stamp.path)?;
    let root_path = root.path.canonicalize()?;
    let path = stamp.path.canonicalize()?;
    if !metadata.is_file()
        || crate::fs_safe::is_link(&metadata)
        || path == root_path
        || !path.starts_with(&root_path)
        || metadata.len() != stamp.bytes
        || metadata.modified().ok() != stamp.modified
        || metadata.accessed().ok() != stamp.accessed
        || identity != stamp.identity
        || (root.older_than_day
            && !age_eligible(
                metadata.modified().ok(),
                metadata.accessed().ok(),
                now,
                Duration::from_secs(86400),
            ))
    {
        return Err(CoreError::InvalidInput(
            "A reviewed cache file changed or became recently used; scan and confirm again.".into(),
        ));
    }
    crate::fs_safe::validate_trash_path(&stamp.path)?;
    Ok(())
}
#[cfg(test)]
fn clean_reviewed_with(
    roots: &[CleanRoot],
    categories: &[CleanCategory],
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    trash: &mut dyn FnMut(&Path) -> CoreResult<()>,
) -> CoreResult<usize> {
    clean_reviewed_with_activity(
        roots,
        categories,
        confirmed,
        cancel,
        progress,
        trash,
        (Os::current(), &mut || Ok(Vec::new())),
    )
}
fn clean_reviewed_with_activity(
    roots: &[CleanRoot],
    categories: &[CleanCategory],
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    trash: &mut dyn FnMut(&Path) -> CoreResult<()>,
    activity: (Os, &mut dyn FnMut() -> CoreResult<Vec<String>>),
) -> CoreResult<usize> {
    let (os, activity) = activity;
    cancel.check()?;
    if !confirmed || categories.is_empty() {
        return Err(CoreError::InvalidInput(
            "Confirm the exact reviewed cleanup files.".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut reviewed = Vec::new();
    let now = SystemTime::now();
    for category in categories {
        let root = roots
            .iter()
            .find(|root| root.id == category.id && root.path == category.path && !root.read_only)
            .ok_or_else(|| {
                CoreError::InvalidInput(
                    "The category is outside the current OS cleanup allow-list.".into(),
                )
            })?;
        cancel.check()?;
        check_browser_activity(root, os, activity)?;
        if category.read_only
            || category.limited
            || category.files != category.stamps.len() as u64
            || category.candidates.len() != category.stamps.len()
            || category
                .stamps
                .iter()
                .map(|stamp| stamp.bytes)
                .fold(0_u64, u64::saturating_add)
                != category.bytes
        {
            return Err(CoreError::InvalidInput(
                "Incomplete or changed cleanup review; scan again.".into(),
            ));
        }
        for (path, stamp) in category.candidates.iter().zip(&category.stamps) {
            cancel.check()?;
            if path != &stamp.path || !seen.insert(path.clone()) {
                return Err(CoreError::InvalidInput(
                    "The cleanup review contains altered or duplicate paths.".into(),
                ));
            }
            check_reviewed_stamp(root, stamp, now)?;
            reviewed.push((root, stamp));
        }
    }
    let total = reviewed.len() as u64;
    let mut count = 0;
    for (root, stamp) in reviewed {
        cancel.check()?;
        check_browser_activity(root, os, activity)?;
        // Recheck immediately before each OS request too. This narrows ordinary
        // changes but is not an atomic lock against concurrent filesystem writes.
        check_reviewed_stamp(root, stamp, SystemTime::now())?;
        trash(&stamp.path)?;
        count += 1;
        progress(Progress {
            done: count as u64,
            total,
            message: "Moving reviewed cache files to Trash".into(),
        });
    }
    Ok(count)
}
pub fn clean_reviewed_categories(
    categories: &[CleanCategory],
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<usize> {
    clean_current_review(
        &current_locations()?,
        categories,
        confirmed,
        cancel,
        progress,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn aged_file(path: &Path, content: &[u8]) {
        std::fs::write(path, content).unwrap();
        let old = SystemTime::now() - Duration::from_secs(3 * 86400);
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(old)
                    .set_accessed(old),
            )
            .unwrap();
    }

    #[test]
    fn cleanup_age_uses_latest_available_access_or_modification_time() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * 86400);
        let old = now - Duration::from_secs(3 * 86400);
        assert!(age_eligible(
            Some(old),
            Some(old),
            now,
            Duration::from_secs(86400)
        ));
        assert!(!age_eligible(
            Some(old),
            Some(now),
            now,
            Duration::from_secs(86400)
        ));
        assert!(!age_eligible(
            Some(now),
            Some(old),
            now,
            Duration::from_secs(86400)
        ));
        assert!(!age_eligible(None, None, now, Duration::from_secs(86400)));
        assert!(!age_eligible(
            Some(now + Duration::from_secs(1)),
            None,
            now,
            Duration::from_secs(86400)
        ));
    }

    #[test]
    fn cleanup_budget_makes_partial_category_read_only_and_preserves_unlisted_files() {
        let root = tempfile::tempdir().unwrap();
        let roots = locations(Os::Linux, root.path());
        let cache = roots.iter().find(|r| r.id == "browser-cache").unwrap();
        std::fs::create_dir_all(&cache.path).unwrap();
        for name in ["one", "two", "three"] {
            std::fs::write(cache.path.join(name), b"generated").unwrap();
        }
        let categories = scan_roots_with_budget(
            &roots,
            ScanBudget {
                max_entries_per_category: 1,
                max_depth: 8,
            },
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        let category = categories.iter().find(|c| c.id == "browser-cache").unwrap();
        assert!(category.limited);
        assert!(category.read_only);
        assert!(category.candidates.is_empty());
        assert_eq!(std::fs::read_dir(&cache.path).unwrap().count(), 3);
    }

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
        aged_file(&cache.path.join("cached"), b"12345");
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
            aged_file(&root.path.join(name), b"12345");
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

    #[test]
    fn cleanup_review_rejects_changed_files_before_any_fake_trash_and_keeps_new_files() {
        let home = tempfile::tempdir().unwrap();
        let roots = locations(Os::Linux, home.path());
        let root = roots
            .iter()
            .find(|root| root.id == "browser-cache")
            .unwrap();
        std::fs::create_dir_all(&root.path).unwrap();
        let original = root.path.join("reviewed");
        aged_file(&original, b"old");
        let review = scan_roots(
            std::slice::from_ref(root),
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        let unrelated = root.path.join("not-in-review");
        aged_file(&unrelated, b"new file");
        let mut requested = Vec::new();
        clean_reviewed_with(
            &roots,
            &review,
            true,
            &CancelToken::default(),
            &mut |_| {},
            &mut |path| {
                requested.push(path.to_path_buf());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(requested, vec![original.clone()]);
        std::fs::write(&original, b"new cache content").unwrap();
        requested.clear();
        assert!(
            clean_reviewed_with(
                &roots,
                &review,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |path| {
                    requested.push(path.to_path_buf());
                    Ok(())
                }
            )
            .is_err()
        );
        assert!(requested.is_empty());
        assert!(unrelated.exists());
    }
    #[test]
    fn cleanup_browser_profile_discovery_only_adds_cache_leaves_not_private_databases() {
        let home = tempfile::tempdir().unwrap();
        let chrome = home
            .path()
            .join("AppData/Local/Google/Chrome/User Data/Profile 2");
        let firefox = home
            .path()
            .join("AppData/Local/Mozilla/Firefox/Profiles/example.default-release");
        for directory in [
            chrome.join("Cache"),
            chrome.join("Code Cache"),
            firefox.join("cache2/entries"),
        ] {
            std::fs::create_dir_all(&directory).unwrap();
            aged_file(&directory.join("cached-response"), b"cache");
        }
        for file in [
            chrome.join("Cookies"),
            chrome.join("History"),
            chrome.join("Login Data"),
            firefox.join("cookies.sqlite"),
            firefox.join("places.sqlite"),
        ] {
            aged_file(&file, b"private fixture kept");
        }
        let mut roots = locations(Os::Windows, home.path());
        discover_browser_caches(Os::Windows, home.path(), &mut roots).unwrap();
        let categories = scan_roots(&roots, &CancelToken::default(), &mut |_| {}).unwrap();
        let paths = categories
            .iter()
            .flat_map(|category| &category.candidates)
            .collect::<Vec<_>>();
        assert_eq!(paths.len(), 3);
        assert!(
            paths
                .iter()
                .all(|path| path.file_name().unwrap() == "cached-response")
        );
        assert!(chrome.join("Cookies").exists());
        assert!(firefox.join("cookies.sqlite").exists());
    }
    #[test]
    fn cleanup_active_browser_cache_is_read_only_and_refused_at_apply() {
        use std::cell::Cell;
        let home = tempfile::tempdir().unwrap();
        let roots = locations(Os::Linux, home.path());
        let root = roots
            .iter()
            .find(|root| root.id == "browser-cache")
            .unwrap();
        std::fs::create_dir_all(&root.path).unwrap();
        for name in ["first", "second"] {
            aged_file(&root.path.join(name), b"old cache");
        }
        let review = scan_roots(
            std::slice::from_ref(root),
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        let mut guarded = review.clone();
        apply_active_browser_guard(Os::Linux, &mut guarded, &["Chromium".into()]);
        assert!(guarded[0].read_only);
        assert!(guarded[0].candidates.is_empty());
        let count = Cell::new(0);
        assert!(
            clean_reviewed_with_activity(
                &roots,
                &review,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| {
                    count.set(count.get() + 1);
                    Ok(())
                },
                (Os::Linux, &mut || Ok(vec!["chromium".into()]))
            )
            .is_err()
        );
        assert_eq!(
            count.get(),
            0,
            "active browser must be refused before any fake Trash"
        );
        assert!(
            clean_reviewed_with_activity(
                &roots,
                &review,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| {
                    count.set(count.get() + 1);
                    Ok(())
                },
                (Os::Linux, &mut || Ok(if count.get() == 0 {
                    vec![]
                } else {
                    vec!["chromium".into()]
                }))
            )
            .is_err()
        );
        assert_eq!(
            count.get(),
            1,
            "a browser starting during cleanup must stop remaining requests"
        );
        let mut chrome = review.clone();
        chrome[0].id = "chrome-cache".into();
        apply_active_browser_guard(Os::Linux, &mut chrome, &["firefox".into()]);
        assert!(
            !chrome[0].read_only,
            "unrelated app must not relabel Chrome's cache"
        );
    }
    #[test]
    fn cleanup_review_rejects_replaced_file_with_preserved_size_and_timestamps() {
        let home = tempfile::tempdir().unwrap();
        let roots = locations(Os::Linux, home.path());
        let root = roots
            .iter()
            .find(|root| root.id == "browser-cache")
            .unwrap();
        std::fs::create_dir_all(&root.path).unwrap();
        let path = root.path.join("reviewed-cache");
        aged_file(&path, b"old");
        let categories = scan_roots(
            std::slice::from_ref(root),
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        std::fs::rename(&path, home.path().join("preserved-original")).unwrap();
        std::fs::write(&path, b"NEW").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(metadata.modified().unwrap())
                    .set_accessed(metadata.accessed().unwrap()),
            )
            .unwrap();
        let mut requested = Vec::new();
        assert!(
            clean_reviewed_with(
                &roots,
                &categories,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |path| {
                    requested.push(path.to_path_buf());
                    Ok(())
                }
            )
            .is_err(),
            "a replacement file inherited another file's cleanup review"
        );
        assert!(
            requested.is_empty(),
            "replacement must be refused before any fake Trash"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"NEW");
    }
}

//! Tool: **Find Duplicates** (`find-duplicates`).

use crate::{
    CoreError, CoreResult,
    fs_safe::reject_symlink_components,
    jobs::{CancelToken, Progress},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub path: PathBuf,
    pub modified_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub size: u64,
    pub files: Vec<DuplicateFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateResult {
    pub groups: Vec<DuplicateGroup>,
    pub wasted_bytes: u64,
    pub scanned_files: u64,
}

fn hash(path: &Path, prefix_only: bool, cancel: &CancelToken) -> CoreResult<String> {
    reject_symlink_components(path)?;
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        cancel.check()?;
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        if prefix_only {
            break;
        }
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn include_entry(entry: &walkdir::DirEntry, include_hidden: bool) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if entry.file_type().is_symlink() {
        return false;
    }
    #[cfg(not(windows))]
    if let Ok(metadata) = fs::symlink_metadata(entry.path())
        && crate::fs_safe::is_link(&metadata)
    {
        return false;
    }
    #[cfg(windows)]
    if let Ok(metadata) = fs::symlink_metadata(entry.path()) {
        if crate::fs_safe::is_link(&metadata) {
            return false;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if !include_hidden && metadata.file_attributes() & 2 != 0 {
                return false;
            }
        }
    }
    include_hidden || !entry.file_name().to_string_lossy().starts_with('.')
}

pub fn find_duplicates(
    roots: &[PathBuf],
    include_hidden: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<DuplicateResult> {
    cancel.check()?;
    if roots.is_empty() {
        return Err(CoreError::InvalidInput("choose at least one folder".into()));
    }
    let mut sizes: BTreeMap<u64, Vec<DuplicateFile>> = BTreeMap::new();
    let mut visited = HashSet::new();
    let mut scanned_files = 0;
    for root in roots {
        reject_symlink_components(root)?;
        if !root.is_dir() {
            return Err(CoreError::InvalidInput(
                "duplicate search needs folders".into(),
            ));
        }
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| include_entry(e, include_hidden))
        {
            cancel.check()?;
            let entry = entry.map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = fs::canonicalize(entry.path())?;
            if !visited.insert(path.clone()) {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|e| CoreError::Io(std::io::Error::other(e.to_string())))?;
            scanned_files += 1;
            if metadata.len() > 0 {
                let modified_ms = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |t| t.as_millis().min(u64::MAX as u128) as u64);
                sizes
                    .entry(metadata.len())
                    .or_default()
                    .push(DuplicateFile { path, modified_ms });
            }
            progress(Progress {
                done: scanned_files,
                total: 0,
                message: "Scanning file sizes".into(),
            });
        }
    }
    let total = sizes
        .values()
        .filter(|g| g.len() > 1)
        .map(|g| g.len() as u64)
        .sum();
    let mut done = 0;
    let mut groups = Vec::new();
    let mut wasted_bytes = 0_u64;
    for (size, files) in sizes.into_iter().filter(|(_, g)| g.len() > 1) {
        let mut prefixes: BTreeMap<String, Vec<DuplicateFile>> = BTreeMap::new();
        for file in files {
            prefixes
                .entry(hash(&file.path, true, cancel)?)
                .or_default()
                .push(file);
        }
        for files in prefixes.into_values().filter(|g| g.len() > 1) {
            let mut full: BTreeMap<String, Vec<DuplicateFile>> = BTreeMap::new();
            for file in files {
                full.entry(hash(&file.path, false, cancel)?)
                    .or_default()
                    .push(file);
                done += 1;
                progress(Progress {
                    done,
                    total,
                    message: "Comparing file contents".into(),
                });
            }
            for mut files in full.into_values().filter(|g| g.len() > 1) {
                files.sort_by(|a, b| a.path.cmp(&b.path));
                wasted_bytes =
                    wasted_bytes.saturating_add(size.saturating_mul((files.len() - 1) as u64));
                groups.push(DuplicateGroup { size, files });
            }
        }
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group.size));
    progress(Progress {
        done: total,
        total,
        message: "Comparison complete".into(),
    });
    Ok(DuplicateResult {
        groups,
        wasted_bytes,
        scanned_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::CancelToken;

    #[test]
    fn groups_identical_files_and_ignores_empty_and_different_content() {
        let temp = tempfile::tempdir().unwrap();
        for (name, content) in [
            ("a", "hello"),
            ("b", "hello"),
            ("c", "world"),
            ("d", ""),
            ("e", ""),
        ] {
            std::fs::write(temp.path().join(name), content).unwrap();
        }
        let result = find_duplicates(
            &[temp.path().to_path_buf()],
            false,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.wasted_bytes, 5);
    }

    #[test]
    fn overlapping_roots_do_not_duplicate_a_file() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("only"), b"same").unwrap();
        let result = find_duplicates(
            &[temp.path().to_path_buf(), temp.path().to_path_buf()],
            false,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert!(result.groups.is_empty());
    }

    #[test]
    fn cancellation_is_checked_before_scanning() {
        let token = CancelToken::default();
        token.cancel();
        assert_eq!(
            find_duplicates(&[], false, &token, &mut |_| {})
                .unwrap_err()
                .code(),
            "cancelled"
        );
    }
}

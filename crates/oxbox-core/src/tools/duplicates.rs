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
    time::{SystemTime, UNIX_EPOCH},
};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub path: PathBuf,
    pub modified_ms: u64,
    #[serde(skip)]
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub size: u64,
    pub files: Vec<DuplicateFile>,
    #[serde(skip)]
    fingerprint: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateResult {
    pub groups: Vec<DuplicateGroup>,
    pub wasted_bytes: u64,
    pub scanned_files: u64,
}

/// Verify a duplicate scan again before moving only the reviewed paths to Trash.
pub fn trash_selected(
    groups: &[DuplicateGroup],
    selected: &[PathBuf],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<usize> {
    trash_selected_with(
        groups,
        selected,
        cancel,
        progress,
        crate::fs_safe::move_to_trash,
    )
}

fn trash_selected_with(
    groups: &[DuplicateGroup],
    selected: &[PathBuf],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    trash: impl FnOnce(&[PathBuf]) -> CoreResult<usize>,
) -> CoreResult<usize> {
    cancel.check()?;
    let changed = || {
        CoreError::InvalidInput("duplicate files changed; scan and review the selection again before moving anything to Trash".into())
    };
    if selected.is_empty() {
        return Err(changed());
    }
    let mut checked = Vec::with_capacity(selected.len());
    for path in selected {
        cancel.check()?;
        checked.push(crate::fs_safe::validate_trash_path(path)?);
    }
    let chosen: HashSet<_> = checked.iter().cloned().collect();
    if chosen.len() != checked.len() {
        return Err(changed());
    }
    let affected: Vec<_> = groups
        .iter()
        .filter(|group| group.files.iter().any(|file| chosen.contains(&file.path)))
        .collect();
    let mut covered = HashSet::new();
    for group in &affected {
        if group.files.len() < 2
            || group.fingerprint.is_empty()
            || group.files.iter().all(|file| chosen.contains(&file.path))
        {
            return Err(changed());
        }
        for file in &group.files {
            if chosen.contains(&file.path) && !covered.insert(&file.path) {
                return Err(changed());
            }
        }
    }
    if covered.len() != chosen.len() {
        return Err(changed());
    }
    let total = affected.iter().fold(0_u64, |total, group| {
        total.saturating_add(group.files.len() as u64)
    });
    let mut done = 0;
    // Re-read both selected copies and the copies that must remain. No OS Trash
    // call is allowed until every affected group matches its original content.
    for group in &affected {
        for file in &group.files {
            cancel.check()?;
            validate_snapshot(file, group.size).map_err(|_| changed())?;
            if hash(&file.path, false, cancel)? != group.fingerprint {
                return Err(changed());
            }
            validate_snapshot(file, group.size).map_err(|_| changed())?;
            done += 1;
            progress(Progress {
                done,
                total,
                message: "Revalidating copies and kept files before Trash".into(),
            });
        }
    }
    // Also catch ordinary changes to earlier files while the other groups were
    // being read. This is a preflight, not an atomic multi-file transaction.
    for group in affected {
        for file in &group.files {
            cancel.check()?;
            validate_snapshot(file, group.size).map_err(|_| changed())?;
        }
    }
    cancel.check()?;
    trash(&checked)
}

fn validate_snapshot(file: &DuplicateFile, size: u64) -> CoreResult<()> {
    reject_symlink_components(&file.path)?;
    let metadata = fs::symlink_metadata(&file.path)?;
    if !metadata.is_file() || metadata.len() != size || metadata.modified().ok() != file.modified {
        return Err(CoreError::InvalidInput(
            "duplicate file no longer matches its scan snapshot".into(),
        ));
    }
    Ok(())
}

fn hash(path: &Path, prefix_only: bool, cancel: &CancelToken) -> CoreResult<String> {
    reject_symlink_components(path)?;
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(CoreError::InvalidInput(
            "duplicate comparison needs regular files".into(),
        ));
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(CoreError::InvalidInput(
            "duplicate comparison needs regular files".into(),
        ));
    }
    let mut reader = BufReader::new(file);
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
                let modified = metadata.modified().ok();
                let modified_ms = modified
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |t| t.as_millis().min(u64::MAX as u128) as u64);
                sizes
                    .entry(metadata.len())
                    .or_default()
                    .push(DuplicateFile {
                        path,
                        modified_ms,
                        modified,
                    });
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
            for (fingerprint, mut files) in full.into_iter().filter(|(_, group)| group.len() > 1) {
                files.sort_by(|a, b| a.path.cmp(&b.path));
                wasted_bytes =
                    wasted_bytes.saturating_add(size.saturating_mul((files.len() - 1) as u64));
                groups.push(DuplicateGroup {
                    size,
                    files,
                    fingerprint,
                });
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

    fn cleanup_fixture() -> (tempfile::TempDir, DuplicateResult, PathBuf, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let keeper = directory.path().join("keeper.bin");
        let selected = directory.path().join("selected.bin");
        fs::write(&keeper, b"aaaa").unwrap();
        fs::write(&selected, b"aaaa").unwrap();
        let result = find_duplicates(
            &[directory.path().to_path_buf()],
            false,
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        (
            directory,
            result,
            selected.canonicalize().unwrap(),
            keeper.canonicalize().unwrap(),
        )
    }

    #[test]
    fn trash_rejects_removed_or_changed_keeper_and_changed_selected_before_any_effect() {
        for mutation in [
            "remove-keeper",
            "change-keeper",
            "change-selected",
            "change-both-preserved-mtime",
        ] {
            let (_directory, result, selected, keeper) = cleanup_fixture();
            match mutation {
                "remove-keeper" => fs::remove_file(&keeper).unwrap(),
                "change-keeper" => fs::write(&keeper, b"bbbb").unwrap(),
                "change-both-preserved-mtime" => {
                    for file in &result.groups[0].files {
                        fs::write(&file.path, b"bbbb").unwrap();
                        let handle = fs::OpenOptions::new().write(true).open(&file.path).unwrap();
                        handle
                            .set_times(fs::FileTimes::new().set_modified(file.modified.unwrap()))
                            .unwrap();
                    }
                }
                _ => fs::write(&selected, b"bbbb").unwrap(),
            }
            let mut called = false;
            let outcome = trash_selected_with(
                &result.groups,
                std::slice::from_ref(&selected),
                &CancelToken::default(),
                &mut |_| {},
                |paths| {
                    called = true;
                    Ok(paths.len())
                },
            );
            assert!(
                outcome.is_err(),
                "stale duplicate scan accepted: {mutation}"
            );
            assert!(!called, "Trash invoked before full preflight: {mutation}");
            assert!(selected.is_file());
        }
    }

    #[test]
    fn trash_refuses_whole_groups_and_unreviewed_paths() {
        let (directory, result, selected, keeper) = cleanup_fixture();
        let other = directory.path().join("other.bin");
        fs::write(&other, b"aaaa").unwrap();
        for paths in [
            vec![selected.clone(), keeper],
            vec![other.canonicalize().unwrap()],
        ] {
            let mut called = false;
            assert!(
                trash_selected_with(
                    &result.groups,
                    &paths,
                    &CancelToken::default(),
                    &mut |_| {},
                    |_| {
                        called = true;
                        Ok(1)
                    }
                )
                .is_err()
            );
            assert!(!called);
        }
    }

    #[test]
    fn trash_validates_all_copies_but_sends_only_the_confirmed_selection() {
        let (_directory, result, selected, keeper) = cleanup_fixture();
        let mut received = Vec::new();
        let count = trash_selected_with(
            &result.groups,
            std::slice::from_ref(&selected),
            &CancelToken::default(),
            &mut |_| {},
            |paths| {
                received = paths.to_vec();
                Ok(paths.len())
            },
        )
        .unwrap();
        assert_eq!(count, 1);
        assert_eq!(received, [selected]);
        assert!(keeper.is_file());
    }

    #[test]
    fn trash_cancellation_during_revalidation_never_reaches_the_effect() {
        let (_directory, result, selected, keeper) = cleanup_fixture();
        let cancel = CancelToken::default();
        let stop = cancel.clone();
        let mut called = false;
        let outcome = trash_selected_with(
            &result.groups,
            std::slice::from_ref(&selected),
            &cancel,
            &mut |_| stop.cancel(),
            |_| {
                called = true;
                Ok(1)
            },
        );
        assert_eq!(outcome.unwrap_err().code(), "cancelled");
        assert!(!called);
        assert!(selected.is_file() && keeper.is_file());
    }

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

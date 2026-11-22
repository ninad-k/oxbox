//! Read-only, explicitly selected and budgeted storage analysis.

use super::maintenance_support::{invalid, local_path};
use crate::{
    CoreResult, fs_safe,
    jobs::{CancelToken, Progress},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AnalysisOptions {
    pub max_entries: u64,
    pub max_depth: usize,
    pub max_large_files: usize,
    pub minimum_file_bytes: u64,
}
impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            max_entries: 250_000,
            max_depth: 32,
            max_large_files: 100,
            minimum_file_bytes: 100 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct LargeFile {
    pub path: PathBuf,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct CategoryUsage {
    pub name: String,
    pub files: u64,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct StorageReport {
    pub root: PathBuf,
    pub total_bytes: u64,
    pub total_files: u64,
    pub directories: u64,
    pub skipped: u64,
    pub visited: u64,
    pub truncated: bool,
    pub large_files: Vec<LargeFile>,
    pub categories: Vec<CategoryUsage>,
}
fn category(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "bmp" | "tif" | "tiff" | "svg" => {
            "Images"
        }
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" => "Video",
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "opus" => "Audio",
        "txt" | "md" | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "csv" | "ppt" | "pptx" | "odt"
        | "rtf" => "Documents",
        "zip" | "7z" | "tar" | "gz" | "xz" | "bz2" | "rar" | "age" => "Archives",
        "exe" | "msi" | "dll" | "dmg" | "pkg" | "appimage" | "deb" | "rpm" => "Applications",
        _ => "Other",
    }
}
pub fn analyze(
    root: &Path,
    options: &AnalysisOptions,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<StorageReport> {
    cancel.check()?;
    local_path(root)?;
    if !(1..=1_000_000).contains(&options.max_entries)
        || !(1..=128).contains(&options.max_depth)
        || !(1..=1000).contains(&options.max_large_files)
    {
        return Err(invalid(
            "Choose 1–1,000,000 entries, depth 1–128 and 1–1,000 large-file results.",
        ));
    }
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || fs_safe::is_link(&metadata) {
        return Err(invalid(
            "Choose a regular local folder, not a file or link.",
        ));
    }
    let root = root.canonicalize()?;
    let mut report = StorageReport {
        root: root.clone(),
        total_bytes: 0,
        total_files: 0,
        directories: 0,
        skipped: 0,
        visited: 0,
        truncated: false,
        large_files: vec![],
        categories: vec![],
    };
    let mut totals: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    let mut walk = walkdir::WalkDir::new(&root)
        .min_depth(1)
        .max_depth(options.max_depth)
        .follow_links(false)
        .same_file_system(true)
        .into_iter();
    while let Some(entry) = walk.next() {
        cancel.check()?;
        if report.visited >= options.max_entries {
            report.truncated = true;
            break;
        }
        report.visited += 1;
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        if fs_safe::is_link(&metadata) || fs_safe::reject_symlink_components(entry.path()).is_err()
        {
            if metadata.is_dir() {
                walk.skip_current_dir();
            }
            report.skipped += 1;
            continue;
        }
        if metadata.is_dir() {
            report.directories += 1;
            if entry.depth() == options.max_depth {
                report.truncated = true;
            }
        } else if metadata.is_file() {
            report.total_files += 1;
            report.total_bytes = report.total_bytes.saturating_add(metadata.len());
            let total = totals.entry(category(entry.path())).or_default();
            total.0 += 1;
            total.1 = total.1.saturating_add(metadata.len());
            if metadata.len() >= options.minimum_file_bytes {
                report.large_files.push(LargeFile {
                    path: entry.path().to_path_buf(),
                    bytes: metadata.len(),
                });
                report
                    .large_files
                    .sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
                report.large_files.truncate(options.max_large_files);
            }
        } else {
            report.skipped += 1;
        }
        progress(Progress {
            done: report.visited,
            total: options.max_entries,
            message: format!(
                "Analyzed {} paths ({} files); no files changed",
                report.visited, report.total_files
            ),
        });
    }
    cancel.check()?;
    report.categories = totals
        .into_iter()
        .map(|(name, (files, bytes))| CategoryUsage {
            name: name.into(),
            files,
            bytes,
        })
        .collect();
    report
        .categories
        .sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::CancelToken;

    #[test]
    fn storage_analysis_reports_large_files_and_categories_without_modification() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("nested")).unwrap();
        for (name, size) in [
            ("photo.png", 100),
            ("nested/movie.mp4", 200),
            ("notes.txt", 30),
        ] {
            std::fs::write(root.path().join(name), vec![1_u8; size]).unwrap();
        }
        let report = analyze(
            root.path(),
            &AnalysisOptions {
                minimum_file_bytes: 0,
                max_large_files: 2,
                ..Default::default()
            },
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(report.total_files, 3);
        assert_eq!(report.total_bytes, 330);
        assert_eq!(report.large_files.len(), 2);
        assert_eq!(report.large_files[0].bytes, 200);
        assert!(
            report
                .categories
                .iter()
                .any(|c| c.name == "Images" && c.bytes == 100)
        );
        assert!(!report.truncated);
        assert_eq!(
            std::fs::read(root.path().join("photo.png")).unwrap(),
            vec![1_u8; 100]
        );
    }

    #[test]
    fn storage_analysis_limits_traversal_and_honors_cancel_and_invalid_roots() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..5 {
            std::fs::write(root.path().join(format!("{index}.bin")), b"fixture").unwrap();
        }
        let options = AnalysisOptions {
            max_entries: 2,
            ..Default::default()
        };
        let report = analyze(root.path(), &options, &CancelToken::default(), &mut |_| {}).unwrap();
        assert!(report.truncated);
        assert!(report.total_files <= 2);
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            analyze(root.path(), &options, &cancel, &mut |_| {})
                .unwrap_err()
                .code(),
            "cancelled"
        );
        assert!(
            analyze(
                &root.path().join("0.bin"),
                &options,
                &CancelToken::default(),
                &mut |_| {}
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn storage_analysis_never_follows_symlinked_directories() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("private.txt"), b"do not traverse").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("redirect")).unwrap();
        let report = analyze(
            root.path(),
            &AnalysisOptions::default(),
            &CancelToken::default(),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(report.total_files, 0);
        assert!(report.skipped > 0);
    }
}

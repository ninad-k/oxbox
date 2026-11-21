//! File-tool command adapters. All disk traversal and formats live in core.
use crate::{error::ToolError, jobs::run_job};
use oxbox_core::{
    CoreError,
    tools::{archive, checksum, clean_drive, duplicates},
};
use std::path::PathBuf;

#[tauri::command]
pub async fn checksum_file_job(
    app: tauri::AppHandle,
    job_id: String,
    path: PathBuf,
    algorithm: checksum::Algorithm,
) -> Result<String, ToolError> {
    run_job(app, job_id, "verify-checksum", move |cancel, progress| {
        checksum::hash_file_with_progress(&path, algorithm, &cancel, progress)
    })
    .await
}

#[tauri::command]
pub async fn find_duplicates(
    app: tauri::AppHandle,
    job_id: String,
    roots: Vec<PathBuf>,
    include_hidden: bool,
) -> Result<duplicates::DuplicateResult, ToolError> {
    if roots.len() > 64 {
        return Err(CoreError::InvalidInput("choose at most 64 folders".into()).into());
    }
    run_job(app, job_id, "find-duplicates", move |cancel, progress| {
        duplicates::find_duplicates(&roots, include_hidden, &cancel, progress)
    })
    .await
}

#[tauri::command]
pub async fn trash_files(paths: Vec<PathBuf>, confirmed: bool) -> Result<usize, ToolError> {
    if !confirmed || paths.len() > 100_000 {
        return Err(CoreError::InvalidInput(
            "confirm the reviewed files before moving them to Trash".into(),
        )
        .into());
    }
    tauri::async_runtime::spawn_blocking(move || oxbox_core::fs_safe::move_to_trash(&paths))
        .await
        .map_err(|e| ToolError::internal(e.to_string()))?
        .map_err(ToolError::from)
}

#[tauri::command]
pub async fn create_archive(
    app: tauri::AppHandle,
    job_id: String,
    sources: Vec<PathBuf>,
    output: PathBuf,
    format: archive::ArchiveFormat,
) -> Result<PathBuf, ToolError> {
    if sources.len() > 100_000 {
        return Err(CoreError::InvalidInput("too many source paths".into()).into());
    }
    run_job(app, job_id, "archive", move |cancel, progress| {
        archive::create_archive(&sources, &output, format, &cancel, progress)
    })
    .await
}

#[tauri::command]
pub async fn extract_archive(
    app: tauri::AppHandle,
    job_id: String,
    input: PathBuf,
    output: PathBuf,
    max_bytes: u64,
) -> Result<PathBuf, ToolError> {
    run_job(app, job_id, "archive", move |cancel, progress| {
        archive::extract_archive(&input, &output, max_bytes, &cancel, progress)
    })
    .await
}

#[tauri::command]
pub async fn scan_clean_drive(
    app: tauri::AppHandle,
    job_id: String,
) -> Result<Vec<clean_drive::CleanCategory>, ToolError> {
    run_job(app, job_id, "clean-drive", move |cancel, progress| {
        clean_drive::scan_current(&cancel, progress)
    })
    .await
}

#[tauri::command]
pub async fn clean_drive(
    app: tauri::AppHandle,
    job_id: String,
    ids: Vec<String>,
    max_files: u64,
    max_bytes: u64,
    confirmed: bool,
) -> Result<usize, ToolError> {
    if !confirmed {
        return Err(CoreError::InvalidInput(
            "confirm the cleanup preview before continuing".into(),
        )
        .into());
    }
    run_job(app, job_id, "clean-drive", move |cancel, progress| {
        clean_drive::clean_categories(&ids, max_files, max_bytes, &cancel, progress)
    })
    .await
}

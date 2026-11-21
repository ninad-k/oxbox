//! Desktop adapters for capture, clipboard, images and age encryption.

use std::path::PathBuf;

use oxbox_core::jobs::CancelToken;
use oxbox_core::tools::{clipboard_history, encrypt, resize, screenshot};
use oxbox_core::{CoreError, CoreResult};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::ToolError;

#[derive(Default)]
pub struct MediaState {
    pub clipboard: clipboard_history::ClipboardService,
    pub cancel: CancelToken,
}

pub fn start_background(app: &AppHandle) -> CoreResult<()> {
    app.state::<MediaState>().clipboard.start()
}

pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<MediaState>() {
        state.cancel.cancel();
        state.clipboard.shutdown();
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> Result<T, ToolError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| ToolError::internal(error.to_string()))?
        .map_err(Into::into)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub clipboard_error: Option<String>,
}

pub fn finish_capture(
    app: &AppHandle,
    image: screenshot::RgbaImage,
    folder: Option<PathBuf>,
) -> CoreResult<CaptureResult> {
    let state = app.state::<MediaState>();
    state.cancel.check()?;
    let folder = folder
        .map(Ok)
        .unwrap_or_else(screenshot::output_directory)?;
    state.cancel.check()?;
    let path = screenshot::save_capture(&image, &folder)?;
    state.cancel.check()?;
    let clipboard_error = state
        .clipboard
        .copy_image(&image)
        .err()
        .map(|error| error.to_string());
    Ok(CaptureResult {
        path,
        width: image.width(),
        height: image.height(),
        clipboard_error,
    })
}

pub fn restore_launcher(app: &AppHandle) {
    if let Some(state) = app.try_state::<MediaState>()
        && state.cancel.check().is_err()
    {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub async fn capture_targets() -> Result<Vec<screenshot::TargetInfo>, ToolError> {
    blocking(screenshot::targets).await
}

#[tauri::command]
pub async fn capture_screenshot(
    app: AppHandle,
    target: screenshot::CaptureTarget,
    delay: u64,
    output_dir: Option<PathBuf>,
) -> Result<CaptureResult, ToolError> {
    let cancel = app.state::<MediaState>().cancel.clone();
    cancel.check()?;
    if ![0, 3, 5, 10].contains(&delay) {
        return Err(
            CoreError::InvalidInput("choose a delay of 0, 3, 5 or 10 seconds".into()).into(),
        );
    }
    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|error| ToolError::internal(error.to_string()))?;
    }
    let capture_app = app.clone();
    let result = blocking(move || {
        screenshot::capture_after_delay(
            std::time::Duration::from_millis(delay * 1000 + 250),
            &cancel,
            || {
                let image = screenshot::capture_image(target)?;
                cancel.check()?;
                finish_capture(&capture_app, image, output_dir)
            },
        )
    })
    .await;
    restore_launcher(&app);
    result
}

#[tauri::command]
pub async fn start_area_capture(
    app: AppHandle,
    delay: u64,
    output_dir: Option<PathBuf>,
) -> Result<(), ToolError> {
    crate::overlay::start(&app, delay, output_dir).await
}

#[tauri::command]
pub fn open_screenshot_folder(app: AppHandle) -> Result<(), ToolError> {
    let folder = screenshot::output_directory()?;
    std::fs::create_dir_all(&folder).map_err(CoreError::from)?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|error| ToolError::internal(error.to_string()))
}

#[tauri::command]
pub async fn clipboard_snapshot(
    app: AppHandle,
    query: String,
) -> Result<clipboard_history::Snapshot, ToolError> {
    if query.len() > 4096 {
        return Err(CoreError::InvalidInput("search is too long".into()).into());
    }
    blocking(move || app.state::<MediaState>().clipboard.snapshot(&query)).await
}

#[tauri::command]
pub fn clipboard_set_paused(state: State<'_, MediaState>, paused: bool) -> Result<(), ToolError> {
    Ok(state.clipboard.set_paused(paused)?)
}

#[tauri::command]
pub fn clipboard_pin(state: State<'_, MediaState>, id: u64, pinned: bool) -> Result<(), ToolError> {
    Ok(state.clipboard.pin(id, pinned)?)
}

#[tauri::command]
pub fn clipboard_delete(state: State<'_, MediaState>, id: u64) -> Result<(), ToolError> {
    Ok(state.clipboard.delete(id)?)
}

#[tauri::command]
pub fn clipboard_clear(state: State<'_, MediaState>) -> Result<(), ToolError> {
    Ok(state.clipboard.clear()?)
}

#[tauri::command]
pub async fn clipboard_copy(app: AppHandle, id: u64) -> Result<(), ToolError> {
    blocking(move || app.state::<MediaState>().clipboard.copy(id)).await
}

#[tauri::command]
pub async fn resize_images(
    app: AppHandle,
    job_id: String,
    request: resize::ResizeRequest,
) -> Result<Vec<PathBuf>, ToolError> {
    crate::jobs::run_job(app, job_id, "resize-images", move |cancel, progress| {
        resize::resize_batch(&request, &cancel, progress)
    })
    .await
}

#[tauri::command]
pub async fn crypt_file(
    app: AppHandle,
    job_id: String,
    source: PathBuf,
    output: PathBuf,
    passphrase: String,
    decrypt: bool,
) -> Result<(), ToolError> {
    crate::jobs::run_job(app, job_id, "encrypt-files", move |cancel, progress| {
        encrypt::transform_file(&source, &output, passphrase, decrypt, &cancel, progress)
    })
    .await
}

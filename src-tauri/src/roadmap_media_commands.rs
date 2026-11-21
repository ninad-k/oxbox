//! Thin adapters for local media providers and explicit recording sessions.
use crate::error::ToolError;
use oxbox_core::{
    CoreError, CoreResult,
    jobs::CancelToken,
    platform::Os,
    tools::{
        barcode_reader, color_picker, convert_media, make_gif,
        media_provider::{self, ProviderStatus, Recorder, RecordingStatus},
        mute_microphone, ocr, record_audio, record_screen, screenshot, take_photo, take_video,
    },
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
#[derive(Default)]
pub struct RoadmapMediaState {
    pub recorder: Recorder,
    pub cancel: CancelToken,
}
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<RoadmapMediaState>() {
        state.cancel.cancel();
        state.recorder.shutdown();
    }
}
pub fn stop_from_tray(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let handle = app.clone();
        let result = blocking(move || {
            let state = handle.state::<RoadmapMediaState>();
            let status = state.recorder.status()?;
            if status.active {
                state.recorder.stop(&status.id)?;
            }
            Ok(())
        })
        .await;
        if let Err(error) = result {
            use tauri::Emitter;
            let _ = app.emit("tool://recording/error", format!("{error:?}"));
        }
    });
}
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> Result<T, ToolError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| ToolError::internal(error.to_string()))?
        .map_err(Into::into)
}
#[tauri::command]
pub async fn roadmap_media_providers() -> Result<Vec<ProviderStatus>, ToolError> {
    blocking(|| {
        Ok(vec![
            media_provider::provider_status("ffmpeg"),
            media_provider::provider_status("tesseract"),
        ])
    })
    .await
}
#[tauri::command]
pub async fn roadmap_microphone_status() -> Result<bool, ToolError> {
    blocking(mute_microphone::status).await
}
#[tauri::command]
pub async fn roadmap_microphone_set(muted: bool) -> Result<bool, ToolError> {
    blocking(move || mute_microphone::set_muted(muted)).await
}
#[tauri::command]
pub fn roadmap_recording_status(
    state: State<'_, RoadmapMediaState>,
) -> Result<RecordingStatus, ToolError> {
    Ok(state.recorder.status()?)
}
#[tauri::command]
pub async fn roadmap_recording_stop(
    app: AppHandle,
    id: String,
) -> Result<RecordingStatus, ToolError> {
    blocking(move || app.state::<RoadmapMediaState>().recorder.stop(&id)).await
}
fn confirmed(value: bool) -> CoreResult<()> {
    if !value {
        Err(CoreError::InvalidInput(
            "confirm the device or screen recording before starting".into(),
        ))
    } else {
        Ok(())
    }
}
#[tauri::command]
pub async fn roadmap_record_audio_start(
    app: AppHandle,
    device: String,
    seconds: u64,
    output: PathBuf,
    consent: bool,
) -> Result<RecordingStatus, ToolError> {
    confirmed(consent)?;
    blocking(move || {
        let args = record_audio::plan(Os::current(), &device, seconds)?;
        app.state::<RoadmapMediaState>()
            .recorder
            .start("microphone", args, &output, "wav")
    })
    .await
}
#[tauri::command]
pub async fn roadmap_record_screen_start(
    app: AppHandle,
    request: record_screen::ScreenRequest,
    output: PathBuf,
    consent: bool,
) -> Result<RecordingStatus, ToolError> {
    confirmed(consent)?;
    blocking(move || {
        let args = record_screen::plan(Os::current(), &request)?;
        app.state::<RoadmapMediaState>()
            .recorder
            .start("screen", args, &output, "mp4")
    })
    .await
}
#[tauri::command]
pub async fn roadmap_take_video_start(
    app: AppHandle,
    device: String,
    seconds: u64,
    output: PathBuf,
    consent: bool,
) -> Result<RecordingStatus, ToolError> {
    confirmed(consent)?;
    blocking(move || {
        let args = take_video::plan(Os::current(), &device, seconds)?;
        app.state::<RoadmapMediaState>()
            .recorder
            .start("camera", args, &output, "mp4")
    })
    .await
}
#[tauri::command]
pub async fn roadmap_convert_media(
    app: AppHandle,
    job_id: String,
    input: PathBuf,
    output: PathBuf,
    format: convert_media::MediaFormat,
) -> Result<PathBuf, ToolError> {
    crate::jobs::run_job(
        app,
        job_id,
        "convert-audio-video",
        move |cancel, progress| convert_media::convert(&input, &output, format, &cancel, progress),
    )
    .await
}
#[tauri::command]
pub async fn roadmap_make_gif(
    app: AppHandle,
    job_id: String,
    input: PathBuf,
    output: PathBuf,
    options: make_gif::GifOptions,
) -> Result<PathBuf, ToolError> {
    crate::jobs::run_job(app, job_id, "make-gif", move |cancel, progress| {
        make_gif::make(&input, &output, &options, &cancel, progress)
    })
    .await
}
#[tauri::command]
pub async fn roadmap_take_photo(
    app: AppHandle,
    job_id: String,
    device: String,
    output: PathBuf,
    consent: bool,
) -> Result<PathBuf, ToolError> {
    confirmed(consent)?;
    crate::jobs::run_job(app, job_id, "take-photo", move |cancel, progress| {
        take_photo::capture(&device, &output, &cancel, progress)
    })
    .await
}
#[tauri::command]
pub async fn roadmap_recognize_text(
    app: AppHandle,
    job_id: String,
    path: PathBuf,
    provider: String,
    language: String,
) -> Result<String, ToolError> {
    crate::jobs::run_job(app, job_id, "recognize-text", move |cancel, progress| {
        ocr::recognize(&path, &provider, &language, &cancel, progress)
    })
    .await
}
#[tauri::command]
pub async fn roadmap_read_barcode(
    path: PathBuf,
) -> Result<Vec<barcode_reader::BarcodeResult>, ToolError> {
    blocking(move || barcode_reader::read(&path)).await
}
#[tauri::command]
pub async fn roadmap_color_sample(
    id: u32,
    x: u32,
    y: u32,
) -> Result<color_picker::ColorValue, ToolError> {
    blocking(move || color_picker::sample_monitor(id, x, y)).await
}
#[tauri::command]
pub async fn roadmap_color_cursor(app: AppHandle) -> Result<color_picker::ColorValue, ToolError> {
    let cancel = app.state::<RoadmapMediaState>().cancel.clone();
    let restore = cancel.clone();
    if let Some(window) = app.get_webview_window("main") {
        window
            .hide()
            .map_err(|error| ToolError::internal(error.to_string()))?;
    }
    let result = blocking(move || {
        for _ in 0..30 {
            cancel.check()?;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        cancel.check()?;
        color_picker::sample_cursor()
    })
    .await;
    if restore.check().is_ok() {
        crate::media_commands::restore_launcher(&app);
    }
    result
}
#[tauri::command]
pub async fn roadmap_color_targets() -> Result<Vec<screenshot::TargetInfo>, ToolError> {
    blocking(|| {
        Ok(screenshot::targets()?
            .into_iter()
            .filter(|target| target.kind == screenshot::CaptureKind::Monitor)
            .collect())
    })
    .await
}

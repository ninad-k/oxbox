//! Frozen-frame area selection: one native overlay for each monitor.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use oxbox_core::tools::screenshot::{self, CaptureKind, CaptureTarget, RgbaImage, Selection};
use oxbox_core::{CoreError, CoreResult};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, LogicalSize};
#[cfg(not(target_os = "macos"))]
use tauri::{PhysicalPosition, PhysicalSize};

use crate::error::ToolError;
use crate::media_commands::{CaptureResult, finish_capture, restore_launcher};

struct Frame {
    image: RgbaImage,
    scale: f64,
}
struct Session {
    frames: HashMap<String, Frame>,
    output_dir: Option<PathBuf>,
}

#[derive(Default)]
pub struct AreaCaptureState {
    session: Mutex<Option<Session>>,
    starting: Mutex<bool>,
    quitting: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureEvent {
    result: Option<CaptureResult>,
    error: Option<String>,
}

fn state_error() -> ToolError {
    ToolError::internal("area capture state is unavailable".into())
}

fn is_quitting(app: &AppHandle) -> bool {
    app.try_state::<AreaCaptureState>()
        .is_some_and(|state| state.quitting.load(Ordering::Acquire))
        || app
            .try_state::<crate::media_commands::MediaState>()
            .is_some_and(|state| state.cancel.check().is_err())
}

/// Remove ownership before closing windows, since close events can be reentrant.
fn close_session(app: &AppHandle) -> bool {
    let session = app.try_state::<AreaCaptureState>().and_then(|state| {
        state
            .session
            .lock()
            .ok()
            .and_then(|mut session| session.take())
    });
    if let Some(session) = session {
        close_overlays(app, session.frames.into_keys());
        return true;
    }
    false
}

fn emit_cancelled(app: &AppHandle) {
    if is_quitting(app) {
        return;
    }
    restore_launcher(app);
    let _ = app.emit_to(
        "main",
        "screenshot://complete",
        CaptureEvent {
            result: None,
            error: Some("Area capture cancelled.".into()),
        },
    );
}

#[tauri::command]
pub async fn area_capture_frame(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<tauri::ipc::Response, ToolError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AreaCaptureState>();
        let guard = state.session.lock().map_err(|_| state_error())?;
        let frame = guard
            .as_ref()
            .and_then(|session| session.frames.get(window.label()))
            .ok_or_else(|| CoreError::InvalidInput("the area capture has ended".into()))?;
        Ok(tauri::ipc::Response::new(screenshot::encode_png(
            &frame.image,
        )?))
    })
    .await
    .map_err(|error| ToolError::internal(error.to_string()))?
}

fn close_overlays(app: &AppHandle, labels: impl Iterator<Item = String>) {
    for label in labels {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.close();
        }
    }
}

pub async fn start(
    app: &AppHandle,
    delay: u64,
    output_dir: Option<PathBuf>,
) -> Result<(), ToolError> {
    if is_quitting(app) {
        return Err(CoreError::Cancelled.into());
    }
    if ![0, 3, 5, 10].contains(&delay) {
        return Err(
            CoreError::InvalidInput("choose a delay of 0, 3, 5 or 10 seconds".into()).into(),
        );
    }
    let state = app.state::<AreaCaptureState>();
    {
        let mut starting = state.starting.lock().map_err(|_| state_error())?;
        if *starting || state.session.lock().map_err(|_| state_error())?.is_some() {
            return Err(CoreError::InvalidInput("an area capture is already open".into()).into());
        }
        *starting = true;
    }
    if let Some(window) = app.get_webview_window("main")
        && let Err(error) = window.hide()
    {
        if let Ok(mut starting) = state.starting.lock() {
            *starting = false;
        }
        return Err(ToolError::internal(error.to_string()));
    }
    let result = start_inner(app, delay, output_dir).await;
    if let Ok(mut starting) = state.starting.lock() {
        *starting = false;
    }
    if result.is_err() {
        close_session(app);
        if !is_quitting(app) {
            restore_launcher(app);
        }
    }
    result
}

async fn start_inner(
    app: &AppHandle,
    delay: u64,
    output_dir: Option<PathBuf>,
) -> Result<(), ToolError> {
    let cancel = app
        .state::<crate::media_commands::MediaState>()
        .cancel
        .clone();
    let frames = tauri::async_runtime::spawn_blocking(move || -> CoreResult<_> {
        screenshot::capture_after_delay(std::time::Duration::from_millis(delay * 1000 + 250),&cancel,||{
        let mut frames = Vec::new();
        let mut bytes = 0_usize;
        for target in screenshot::targets()?.into_iter().filter(|target| target.kind == CaptureKind::Monitor) {
            cancel.check()?;
            let image = screenshot::capture_image(CaptureTarget { kind: target.kind, id: target.id })?;
            cancel.check()?;
            bytes = bytes.saturating_add(image.as_raw().len());
            if bytes > 256 * 1024 * 1024 { return Err(CoreError::InvalidInput("area capture is limited to 256 MB of display pixels; capture a single display instead".into())); }
            frames.push((target, image));
        }
        if frames.is_empty() { return Err(CoreError::InvalidInput("no displays are available for capture".into())); }
        Ok(frames)
        })
    }).await.map_err(|error| ToolError::internal(error.to_string()))??;
    if is_quitting(app) {
        return Err(CoreError::Cancelled.into());
    }
    let mut session = Session {
        frames: HashMap::new(),
        output_dir,
    };
    let mut positions = Vec::new();
    for (target, image) in frames {
        let label = format!("capture-{}", target.id);
        positions.push((
            label.clone(),
            target.x,
            target.y,
            target.width,
            target.height,
        ));
        session.frames.insert(
            label,
            Frame {
                image,
                scale: f64::from(target.scale_factor),
            },
        );
    }
    *app.state::<AreaCaptureState>()
        .session
        .lock()
        .map_err(|_| state_error())? = Some(session);
    for (label, x, y, width, height) in positions {
        if is_quitting(app) {
            return Err(CoreError::Cancelled.into());
        }
        let window = crate::runtime_privacy::create_window(
            app,
            label.clone(),
            "overlay".into(),
            |builder| {
                let builder = builder
                    .title("Oxbox area selection")
                    .decorations(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(false)
                    .visible(false);
                // macOS transparency requires private APIs; retain the opaque fallback.
                #[cfg(not(target_os = "macos"))]
                let builder = builder.transparent(true);
                builder
            },
        )?;
        // Xcap uses CoreGraphics logical points on macOS, physical pixels elsewhere.
        // Giving Retina points to PhysicalSize would cover only part of the display.
        #[cfg(target_os = "macos")]
        {
            window
                .set_position(LogicalPosition::new(f64::from(x), f64::from(y)))
                .map_err(|error| ToolError::internal(error.to_string()))?;
            window
                .set_size(LogicalSize::new(f64::from(width), f64::from(height)))
                .map_err(|error| ToolError::internal(error.to_string()))?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            window
                .set_position(PhysicalPosition::new(x, y))
                .map_err(|error| ToolError::internal(error.to_string()))?;
            window
                .set_size(PhysicalSize::new(width, height))
                .map_err(|error| ToolError::internal(error.to_string()))?;
        }
        let scale = window
            .scale_factor()
            .map_err(|error| ToolError::internal(error.to_string()))?;
        if let Some(session) = app
            .state::<AreaCaptureState>()
            .session
            .lock()
            .map_err(|_| state_error())?
            .as_mut()
            && let Some(frame) = session.frames.get_mut(&label)
        {
            frame.scale = scale;
        }
        let close_app = app.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. })
                && close_session(&close_app)
            {
                emit_cancelled(&close_app);
            }
        });
        window
            .show()
            .map_err(|error| ToolError::internal(error.to_string()))?;
        window
            .set_focus()
            .map_err(|error| ToolError::internal(error.to_string()))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn complete_area_capture(
    app: AppHandle,
    window: WebviewWindow,
    selection: Selection,
) -> Result<(), ToolError> {
    if !window.label().starts_with("capture-") {
        return Err(CoreError::InvalidInput(
            "area selection must come from the capture overlay".into(),
        )
        .into());
    }
    let state = app.state::<AreaCaptureState>();
    let session = state
        .session
        .lock()
        .map_err(|_| state_error())?
        .take()
        .ok_or_else(|| CoreError::InvalidInput("the area capture has ended".into()))?;
    close_overlays(&app, session.frames.keys().cloned());
    let frame = session
        .frames
        .get(window.label())
        .ok_or_else(|| CoreError::InvalidInput("unknown capture display".into()))?;
    let crop = screenshot::crop_selection(&frame.image, selection, frame.scale);
    let capture_app = app.clone();
    let result = match crop {
        Ok(image) => match tauri::async_runtime::spawn_blocking(move || {
            finish_capture(&capture_app, image, session.output_dir)
        })
        .await
        {
            Ok(result) => result.map_err(ToolError::from),
            Err(error) => Err(ToolError::internal(error.to_string())),
        },
        Err(error) => Err(error.into()),
    };
    let payload = match &result {
        Ok(result) => CaptureEvent {
            result: Some(result.clone()),
            error: None,
        },
        Err(_) => CaptureEvent {
            result: None,
            error: Some(
                "Area capture could not be saved. Try again or choose another folder.".into(),
            ),
        },
    };
    if !is_quitting(&app) {
        restore_launcher(&app);
        let _ = app.emit_to("main", "screenshot://complete", payload);
    }
    result.map(|_| ())
}

#[tauri::command]
pub fn cancel_area_capture(
    app: AppHandle,
    state: State<'_, AreaCaptureState>,
) -> Result<(), ToolError> {
    let session = state.session.lock().map_err(|_| state_error())?.take();
    if let Some(session) = session {
        close_overlays(&app, session.frames.into_keys());
        emit_cancelled(&app);
    }
    Ok(())
}

pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<AreaCaptureState>() {
        state.quitting.store(true, Ordering::Release);
        close_session(app);
    }
}

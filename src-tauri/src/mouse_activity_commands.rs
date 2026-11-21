//! Thin adapters for explicit Mouse Activity sessions. Core owns its worker.
use crate::error::ToolError;
use oxbox_core::tools::mouse_activity::{Action, Config, Request, Service, Status};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct MouseActivityState(pub Service);
#[tauri::command]
pub fn mouse_activity_reserve(
    state: State<'_, MouseActivityState>,
    action: Action,
) -> Result<Request, ToolError> {
    Ok(state.0.reserve(action)?)
}
#[tauri::command]
pub async fn mouse_activity_start(
    app: AppHandle,
    config: Config,
    request: Request,
) -> Result<Status, ToolError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MouseActivityState>();
        let status = state.0.start(request, config)?;
        let _ = app.emit("tool://mouse-activity/status", &status);
        Ok(status)
    })
    .await
    .map_err(|error| ToolError::internal(error.to_string()))?
}
#[tauri::command]
pub async fn mouse_activity_stop(app: AppHandle, request: Request) -> Result<Status, ToolError> {
    stop_reserved(app, request).await
}
async fn stop_reserved(app: AppHandle, request: Request) -> Result<Status, ToolError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MouseActivityState>();
        let status = state.0.stop(request)?;
        let _ = app.emit("tool://mouse-activity/status", &status);
        Ok(status)
    })
    .await
    .map_err(|error| ToolError::internal(error.to_string()))?
}
#[tauri::command]
pub fn mouse_activity_status(state: State<'_, MouseActivityState>) -> Result<Status, ToolError> {
    Ok(state.0.status()?)
}
pub fn stop_from_tray(app: &AppHandle) {
    // Reserve at the native click boundary, before asynchronous scheduling.
    let request = app.state::<MouseActivityState>().0.reserve(Action::Stop);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = match request {
            Ok(request) => stop_reserved(app.clone(), request).await,
            Err(error) => Err(error.into()),
        };
        if let Err(error) = result {
            let _ = app.emit("tool://mouse-activity/error", error);
        }
    });
}
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<MouseActivityState>() {
        state.0.shutdown();
    }
}

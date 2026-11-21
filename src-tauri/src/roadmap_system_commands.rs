//! Thin command adapters for Phase 5 window, system and app utilities.
use crate::error::ToolError;
use oxbox_core::{
    CoreError, CoreResult,
    tools::{
        close_apps, cpu_temperature, eject_volumes, energy_saver, hidden_files, launch_apps,
        resolution, show_desktop, uninstall, window_manager, window_top,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_store::StoreExt;

#[derive(Clone)]
struct TopmostChange {
    pid: u32,
    original: bool,
    last: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingResolution {
    pub mode: resolution::DisplayMode,
    pub deadline: u64,
    pub remaining_seconds: u64,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct ResolutionStatus {
    pub pending: Option<PendingResolution>,
    pub error: Option<String>,
    pub kept: Option<bool>,
}
#[derive(Default)]
struct ResolutionState {
    status: ResolutionStatus,
    generation: u64,
    expires: Option<Instant>,
    worker: Option<JoinHandle<()>>,
    quitting: bool,
}
impl ResolutionState {
    fn expired_at(&self, now: Instant) -> bool {
        self.expires.is_none_or(|deadline| now >= deadline)
    }
    fn snapshot_at(&self, now: Instant) -> ResolutionStatus {
        let mut status = self.status.clone();
        if let Some(pending) = &mut status.pending {
            let duration = self.expires.map_or(Duration::ZERO, |deadline| {
                deadline.saturating_duration_since(now)
            });
            pending.remaining_seconds = duration.as_secs() + u64::from(duration.subsec_nanos() > 0);
        }
        status
    }
    fn snapshot(&self) -> ResolutionStatus {
        self.snapshot_at(Instant::now())
    }
}
#[derive(Default)]
pub struct RoadmapSystemState {
    topmost: Mutex<HashMap<u64, TopmostChange>>,
    minimized: Mutex<Vec<(u64, u32)>>,
    resolution: Arc<Mutex<ResolutionState>>,
}
fn state_error() -> ToolError {
    ToolError::internal("desktop tool state is unavailable".into())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |t| t.as_secs())
}
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> Result<T, ToolError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| ToolError::internal(e.to_string()))?
        .map_err(Into::into)
}
pub fn initialize(app: &AppHandle) {
    app.manage(RoadmapSystemState::default());
    app.manage(crate::focus_overlay::FocusState::default());
}
pub fn shutdown(app: &AppHandle) {
    crate::focus_overlay::shutdown(app);
    if let Some(state) = app.try_state::<RoadmapSystemState>() {
        let resolution_worker = if let Ok(mut modes) = state.resolution.lock() {
            modes.quitting = true;
            modes.generation = modes.generation.wrapping_add(1);
            if let Some(pending) = modes.status.pending.clone() {
                match resolution::restore(&pending.mode) {
                    Ok(()) => {
                        modes.status.pending = None;
                        modes.expires = None;
                        modes.status.kept = Some(false);
                    }
                    Err(error) => {
                        modes.status.error =
                            Some(format!("Display restore on Quit failed: {error}"))
                    }
                }
            }
            modes.worker.take()
        } else {
            None
        };
        if let Some(worker) = resolution_worker {
            let _ = worker.join();
        }
        if let Ok(mut minimized) = state.minimized.lock() {
            let _ = show_desktop::restore(&std::mem::take(&mut *minimized));
        }
        if let Ok(mut changes) = state.topmost.lock() {
            for (id, change) in changes.drain() {
                if window_top::window(id, change.pid).is_ok_and(|w| w.topmost == change.last) {
                    let _ = window_top::set(id, change.pid, change.original);
                }
            }
        }
    }
}

#[tauri::command]
pub async fn roadmap_windows() -> Result<Vec<window_top::WindowInfo>, ToolError> {
    blocking(window_top::windows).await
}
#[tauri::command]
pub fn roadmap_window_top(
    state: State<'_, RoadmapSystemState>,
    id: u64,
    pid: u32,
    enabled: bool,
) -> Result<(), ToolError> {
    let current = window_top::window(id, pid)?;
    let mut changes = state.topmost.lock().map_err(|_| state_error())?;
    let original = changes
        .get(&id)
        .filter(|change| change.pid == pid)
        .map_or(current.topmost, |change| change.original);
    window_top::set(id, pid, enabled)?;
    if enabled == original {
        changes.remove(&id);
    } else {
        changes.insert(
            id,
            TopmostChange {
                pid,
                original,
                last: enabled,
            },
        );
    }
    Ok(())
}
#[tauri::command]
pub fn roadmap_snap(
    id: u64,
    pid: u32,
    placement: window_manager::Snap,
) -> Result<window_manager::Rect, ToolError> {
    Ok(window_manager::snap(id, pid, placement)?)
}
#[tauri::command]
pub fn roadmap_show_desktop(
    state: State<'_, RoadmapSystemState>,
    restore: bool,
) -> Result<usize, ToolError> {
    let mut saved = state.minimized.lock().map_err(|_| state_error())?;
    if restore {
        let outcome = show_desktop::restore_tracked(&saved);
        *saved = outcome.remaining;
        Ok(outcome.restored)
    } else {
        if !saved.is_empty() {
            return Err(CoreError::InvalidInput(
                "restore the previous group before minimizing again".into(),
            )
            .into());
        }
        *saved = show_desktop::minimize()?;
        Ok(saved.len())
    }
}
#[derive(Deserialize)]
pub struct WindowTarget {
    pub id: u64,
    pub pid: u32,
}
#[tauri::command]
pub fn roadmap_close_apps(targets: Vec<WindowTarget>, confirmed: bool) -> Result<usize, ToolError> {
    Ok(close_apps::close(
        &targets
            .into_iter()
            .map(|w| (w.id, w.pid))
            .collect::<Vec<_>>(),
        confirmed,
    )?)
}
#[tauri::command]
pub async fn roadmap_displays() -> Result<Vec<resolution::Display>, ToolError> {
    blocking(resolution::displays).await
}
#[tauri::command]
pub fn roadmap_resolution_status(
    state: State<'_, RoadmapSystemState>,
) -> Result<ResolutionStatus, ToolError> {
    Ok(state
        .resolution
        .lock()
        .map_err(|_| state_error())?
        .snapshot())
}
#[tauri::command]
pub async fn roadmap_resolution_apply(
    app: AppHandle,
    mode: resolution::DisplayMode,
    confirmed: bool,
) -> Result<ResolutionStatus, ToolError> {
    if !confirmed {
        return Err(CoreError::InvalidInput("confirm the display change first".into()).into());
    }
    let shared = Arc::clone(&app.state::<RoadmapSystemState>().resolution);
    blocking(move || {
        // Join the previous generation outside the state lock before installing a new guard.
        let old_worker = {
            let mut state = shared.lock().map_err(|_| CoreError::InvalidInput("display state is unavailable".into()))?;
            if state.quitting || state.status.pending.is_some() {
                return Err(CoreError::InvalidInput("keep or revert the pending display change first".into()));
            }
            state.worker.take()
        };
        if let Some(worker) = old_worker { let _ = worker.join(); }
        let mut state = shared.lock().map_err(|_| CoreError::InvalidInput("display state is unavailable".into()))?;
        if state.quitting || state.status.pending.is_some() {
            return Err(CoreError::InvalidInput("another display change is already in progress".into()));
        }
        let previous = resolution::displays()?.into_iter().find(|display| display.device == mode.device)
            .ok_or_else(|| CoreError::InvalidInput("the display is no longer connected".into()))?.current;
        let deadline = resolution::rollback_deadline(now())?;
        resolution::apply(&mode)?;
        state.expires = Some(Instant::now() + Duration::from_secs(15));
        state.generation = state.generation.wrapping_add(1);
        let generation = state.generation;
        state.status = ResolutionStatus { pending: Some(PendingResolution { mode: previous, deadline, remaining_seconds: 15 }), error: None, kept: None };
        let thread_state = Arc::clone(&shared);
        match std::thread::Builder::new().name("oxbox-display-rollback".into()).spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(150));
                let Ok(mut state) = thread_state.lock() else { break; };
                if state.quitting || state.generation != generation { break; }
                let Some(pending) = state.status.pending.clone() else { break; };
                if !state.expired_at(Instant::now()) { continue; }
                match resolution::restore(&pending.mode) {
                    Ok(()) => { state.status.pending = None; state.expires = None; state.status.kept = Some(false); state.status.error = Some("Display change timed out and was reverted.".into()); }
                    Err(error) => state.status.error = Some(format!("Automatic restore failed: {error}. Choose Revert or use Windows display settings.")),
                }
                break;
            }
        }) {
            Ok(worker) => state.worker = Some(worker),
            Err(error) => {
                let pending = state.status.pending.clone().ok_or_else(|| CoreError::InvalidInput("display rollback state was lost".into()))?;
                match resolution::restore(&pending.mode) {
                    Ok(()) => { state.status.pending = None; state.expires = None; state.status.kept = Some(false); state.status.error = Some(format!("Could not start automatic rollback; the display was restored: {error}")); }
                    Err(restore) => state.status.error = Some(format!("Could not start automatic rollback ({error}) and restoring failed ({restore}). Choose Revert or use Windows display settings.")),
                }
                return Err(CoreError::InvalidInput(state.status.error.clone().unwrap_or_else(|| "display rollback could not start".into())));
            }
        }
        Ok(state.snapshot())
    }).await
}
#[tauri::command]
pub fn roadmap_resolution_finish(
    state: State<'_, RoadmapSystemState>,
    keep: bool,
) -> Result<ResolutionStatus, ToolError> {
    let mut modes = state.resolution.lock().map_err(|_| state_error())?;
    if let Some(pending) = modes.status.pending.clone() {
        let kept = keep && !modes.expired_at(Instant::now());
        if !kept && let Err(error) = resolution::restore(&pending.mode) {
            modes.status.error = Some(format!(
                "Display restore failed: {error}. Choose Revert to retry."
            ));
            return Err(error.into());
        }
        modes.status.pending = None;
        modes.expires = None;
        modes.status.kept = Some(kept);
        modes.status.error = if keep && !kept {
            Some("The confirmation period expired; the previous display mode was restored.".into())
        } else {
            None
        };
        modes.generation = modes.generation.wrapping_add(1);
    }
    let status = modes.snapshot();
    let worker = modes.worker.take();
    drop(modes);
    if let Some(worker) = worker {
        let _ = worker.join();
    }
    Ok(status)
}
#[tauri::command]
pub async fn roadmap_volumes() -> Result<Vec<eject_volumes::Volume>, ToolError> {
    blocking(eject_volumes::volumes).await
}
#[tauri::command]
pub async fn roadmap_eject(root: String, confirmed: bool) -> Result<(), ToolError> {
    blocking(move || eject_volumes::eject(&root, confirmed)).await
}
#[tauri::command]
pub async fn roadmap_power_profiles() -> Result<Vec<energy_saver::PowerProfile>, ToolError> {
    blocking(energy_saver::profiles).await
}
#[tauri::command]
pub async fn roadmap_power_set(id: String, confirmed: bool) -> Result<(), ToolError> {
    blocking(move || energy_saver::set(&id, confirmed)).await
}
#[tauri::command]
pub async fn roadmap_hidden_files(show: Option<bool>, confirmed: bool) -> Result<bool, ToolError> {
    if show.is_some() && !confirmed {
        return Err(
            CoreError::InvalidInput("confirm the file-manager setting change".into()).into(),
        );
    }
    blocking(move || match show {
        Some(show) => hidden_files::set(show),
        None => hidden_files::status(),
    })
    .await
}
#[tauri::command]
pub async fn roadmap_cpu_temperatures() -> Result<Vec<cpu_temperature::Sensor>, ToolError> {
    blocking(|| Ok(cpu_temperature::sample())).await
}

fn read_sets(app: &AppHandle) -> Result<Vec<launch_apps::AppSet>, ToolError> {
    let store = app
        .store("app-sets.json")
        .map_err(|e| ToolError::internal(e.to_string()))?;
    let sets = store
        .get("sets")
        .map(serde_json::from_value::<Vec<launch_apps::AppSet>>)
        .transpose()
        .map_err(|_| {
            CoreError::InvalidInput(
                "saved launch sets are invalid; restore your app-sets.json backup".into(),
            )
        })?
        .unwrap_or_default();
    for set in &sets {
        launch_apps::validate_set(set)?;
    }
    Ok(sets)
}
#[tauri::command]
pub fn roadmap_launch_sets(app: AppHandle) -> Result<Vec<launch_apps::AppSet>, ToolError> {
    read_sets(&app)
}
#[tauri::command]
pub fn roadmap_launch_sets_save(
    app: AppHandle,
    sets: Vec<launch_apps::AppSet>,
) -> Result<(), ToolError> {
    if sets.len() > 64 {
        return Err(CoreError::InvalidInput("save at most 64 launch sets".into()).into());
    }
    let mut ids = std::collections::HashSet::new();
    for set in &sets {
        launch_apps::validate_set(set)?;
        if !ids.insert(&set.id) {
            return Err(
                CoreError::InvalidInput("launch-set identifiers must be unique".into()).into(),
            );
        }
    }
    let value = serde_json::to_value(sets).map_err(|e| ToolError::internal(e.to_string()))?;
    let store = app
        .store("app-sets.json")
        .map_err(|e| ToolError::internal(e.to_string()))?;
    store.set("sets", value);
    store.save().map_err(|e| ToolError::internal(e.to_string()))
}
#[tauri::command]
pub async fn roadmap_launch_set(app: AppHandle, id: String) -> Result<usize, ToolError> {
    let set = read_sets(&app)?
        .into_iter()
        .find(|set| set.id == id)
        .ok_or_else(|| CoreError::InvalidInput("the launch set is no longer available".into()))?;
    blocking(move || {
        launch_apps::launch(&set, &mut |path| {
            app.opener()
                .open_path(path, None::<&str>)
                .map_err(|e| CoreError::InvalidInput(e.to_string()))
        })
    })
    .await
}
#[tauri::command]
pub async fn roadmap_installed_apps() -> Result<Vec<uninstall::InstalledApp>, ToolError> {
    blocking(uninstall::apps).await
}
#[tauri::command]
pub async fn roadmap_uninstall(
    id: String,
    reviewed_command: String,
    confirmed: bool,
) -> Result<u32, ToolError> {
    blocking(move || uninstall::uninstall(&id, &reviewed_command, confirmed)).await
}
#[tauri::command]
pub async fn roadmap_leftovers(
    app: AppHandle,
    job_id: String,
    name: String,
    publisher: String,
) -> Result<Vec<uninstall::Leftover>, ToolError> {
    crate::jobs::run_job(app, job_id, "uninstall-apps", move |cancel, progress| {
        uninstall::scan_leftovers(&name, &publisher, &cancel, progress)
    })
    .await
}

#[cfg(test)]
mod resolution_guard_tests {
    use super::*;
    fn pending_state(start: Instant) -> ResolutionState {
        ResolutionState {
            status: ResolutionStatus {
                pending: Some(PendingResolution {
                    mode: resolution::DisplayMode {
                        device: "DISPLAY1".into(),
                        width: 1920,
                        height: 1080,
                        refresh: 60,
                        bits_per_pixel: 32,
                    },
                    // Deliberately unrelated wall time: only the monotonic guard controls rollback.
                    deadline: 0,
                    remaining_seconds: 15,
                }),
                error: None,
                kept: None,
            },
            expires: Some(start + Duration::from_secs(15)),
            ..ResolutionState::default()
        }
    }
    #[test]
    fn rollback_expiry_ignores_wall_time_and_rounds_remaining_seconds_up() {
        let start = Instant::now();
        let mut state = pending_state(start);
        assert!(!state.expired_at(start));
        assert_eq!(
            state.snapshot_at(start).pending.unwrap().remaining_seconds,
            15
        );
        assert_eq!(
            state
                .snapshot_at(start + Duration::from_millis(14_001))
                .pending
                .unwrap()
                .remaining_seconds,
            1
        );
        state.status.pending.as_mut().unwrap().deadline = u64::MAX;
        assert!(state.expired_at(start + Duration::from_secs(15)));
        assert_eq!(
            state
                .snapshot_at(start + Duration::from_secs(20))
                .pending
                .unwrap()
                .remaining_seconds,
            0
        );
    }
    #[test]
    fn missing_native_deadline_fails_closed() {
        let start = Instant::now();
        let mut state = pending_state(start);
        state.expires = None;
        assert!(state.expired_at(start));
        assert_eq!(
            state.snapshot_at(start).pending.unwrap().remaining_seconds,
            0
        );
    }
}

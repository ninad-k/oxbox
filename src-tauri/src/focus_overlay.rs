//! Click-through dimming windows that track a selected application window.
use crate::error::ToolError;
use oxbox_core::{
    CoreError,
    tools::{
        focus_window::{self, Cutout},
        window_manager::Rect,
        window_top,
    },
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};

#[derive(Debug, Clone, Serialize)]
pub struct FocusFrame {
    pub cutout: Option<Cutout>,
    pub opacity: f64,
}
struct MonitorFrame {
    bounds: Rect,
    scale: f64,
}
struct Session {
    generation: u64,
    id: u64,
    pid: u32,
    opacity: f64,
    frames: HashMap<String, MonitorFrame>,
}
#[derive(Default)]
pub struct FocusState {
    session: Mutex<Option<Session>>,
    starting: Mutex<bool>,
    quitting: AtomicBool,
    generation: AtomicU64,
    workers: Mutex<Vec<JoinHandle<()>>>,
}
fn state_error() -> ToolError {
    ToolError::internal("focus overlay state is unavailable".into())
}
fn close(app: &AppHandle) {
    close_generation(app, None);
}
fn close_generation(app: &AppHandle, generation: Option<u64>) {
    let session = app.try_state::<FocusState>().and_then(|state| {
        state.session.lock().ok().and_then(|mut s| {
            if generation.is_none_or(|g| s.as_ref().is_some_and(|session| session.generation == g))
            {
                s.take()
            } else {
                None
            }
        })
    });
    if let Some(session) = session {
        for label in session.frames.keys() {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.close();
            }
        }
    }
}
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<FocusState>() {
        state.quitting.store(true, Ordering::Release);
    }
    close(app);
    if let Some(state) = app.try_state::<FocusState>() {
        reap_workers(&state, true);
    }
}
fn reap_workers(state: &FocusState, wait: bool) {
    let ready = if let Ok(mut workers) = state.workers.lock() {
        let (ready, pending): (Vec<_>, Vec<_>) = std::mem::take(&mut *workers)
            .into_iter()
            .partition(|worker| wait || worker.is_finished());
        *workers = pending;
        ready
    } else {
        Vec::new()
    };
    for worker in ready {
        let _ = worker.join();
    }
}
#[tauri::command]
pub fn roadmap_focus_status(state: State<'_, FocusState>) -> Result<bool, ToolError> {
    Ok(state.session.lock().map_err(|_| state_error())?.is_some())
}
#[tauri::command]
pub fn roadmap_focus_stop(app: AppHandle) {
    close(&app);
}
#[tauri::command]
pub fn roadmap_focus_frame(app: AppHandle, window: WebviewWindow) -> Result<FocusFrame, ToolError> {
    let state = app.state::<FocusState>();
    let guard = state.session.lock().map_err(|_| state_error())?;
    let session = guard
        .as_ref()
        .ok_or_else(|| CoreError::InvalidInput("focus dimming has ended".into()))?;
    let monitor = session
        .frames
        .get(window.label())
        .ok_or_else(|| CoreError::InvalidInput("this is not a focus overlay window".into()))?;
    let target = window_top::window(session.id, session.pid)?;
    Ok(FocusFrame {
        cutout: focus_window::cutout(monitor.bounds, target.bounds, monitor.scale)?,
        opacity: session.opacity,
    })
}
#[tauri::command]
pub async fn roadmap_focus_start(
    app: AppHandle,
    id: u64,
    pid: u32,
    opacity: f64,
) -> Result<(), ToolError> {
    if !opacity.is_finite() || !(0.1..=0.85).contains(&opacity) {
        return Err(CoreError::InvalidInput("choose dimming from 10% to 85%".into()).into());
    }
    window_top::window(id, pid)?;
    let state = app.state::<FocusState>();
    if state.quitting.load(Ordering::Acquire) {
        return Err(CoreError::Cancelled.into());
    }
    reap_workers(&state, false);
    let generation = state.generation.fetch_add(1, Ordering::Relaxed);
    {
        let mut starting = state.starting.lock().map_err(|_| state_error())?;
        if *starting || state.session.lock().map_err(|_| state_error())?.is_some() {
            return Err(
                CoreError::InvalidInput("stop the current focus session first".into()).into(),
            );
        }
        *starting = true;
    }
    let result = (|| {
        let main = app
            .get_webview_window("main")
            .ok_or_else(|| ToolError::internal("launcher window is unavailable".into()))?;
        let monitors = main
            .available_monitors()
            .map_err(|e| ToolError::internal(e.to_string()))?;
        let mut frames = HashMap::new();
        for (index, monitor) in monitors.iter().enumerate() {
            let position = monitor.position();
            let size = monitor.size();
            frames.insert(
                format!("focus-dim-{generation}-{index}"),
                MonitorFrame {
                    bounds: Rect {
                        x: position.x,
                        y: position.y,
                        width: size.width,
                        height: size.height,
                    },
                    scale: monitor.scale_factor(),
                },
            );
        }
        if frames.is_empty() {
            return Err(CoreError::InvalidInput("no displays are available".into()).into());
        }
        let labels: Vec<_> = frames.keys().cloned().collect();
        *state.session.lock().map_err(|_| state_error())? = Some(Session {
            generation,
            id,
            pid,
            opacity,
            frames,
        });
        for label in labels {
            if state.quitting.load(Ordering::Acquire) {
                return Err(CoreError::Cancelled.into());
            }
            let (bounds, scale) = {
                let guard = state.session.lock().map_err(|_| state_error())?;
                let monitor = guard
                    .as_ref()
                    .and_then(|s| s.frames.get(&label))
                    .ok_or_else(state_error)?;
                (monitor.bounds, monitor.scale)
            };
            let window = crate::runtime_privacy::create_window(
                &app,
                label.clone(),
                "focus-overlay".into(),
                move |builder| {
                    let builder = builder
                        .title("Oxbox Focus Shade")
                        .decorations(false)
                        .always_on_top(true)
                        .skip_taskbar(true)
                        .focused(false)
                        .focusable(false)
                        .resizable(false)
                        .visible(false)
                        .inner_size(bounds.width as f64 / scale, bounds.height as f64 / scale);
                    #[cfg(not(target_os = "macos"))]
                    let builder = builder.transparent(true);
                    builder
                },
            )?;
            // Stop can race native construction. Never leave a newly-created
            // window behind after ownership of its session has been removed.
            let still_current = state
                .session
                .lock()
                .map_err(|_| state_error())?
                .as_ref()
                .is_some_and(|session| session.generation == generation);
            if !still_current || state.quitting.load(Ordering::Acquire) {
                let _ = window.close();
                return Err(CoreError::Cancelled.into());
            }
            window
                .set_position(PhysicalPosition::new(bounds.x, bounds.y))
                .map_err(|e| ToolError::internal(e.to_string()))?;
            window
                .set_size(PhysicalSize::new(bounds.width, bounds.height))
                .map_err(|e| ToolError::internal(e.to_string()))?;
            window
                .set_ignore_cursor_events(true)
                .map_err(|e| ToolError::internal(e.to_string()))?;
            window
                .show()
                .map_err(|e| ToolError::internal(e.to_string()))?;
        }
        Ok(())
    })();
    if let Ok(mut starting) = state.starting.lock() {
        *starting = false;
    }
    if result.is_err() {
        close_generation(&app, Some(generation));
        return result;
    }
    let tracker = app.clone();
    let worker = std::thread::Builder::new()
        .name("oxbox-focus-overlay".into())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let Some(state) = tracker.try_state::<FocusState>() else {
                    break;
                };
                if state.quitting.load(Ordering::Acquire) {
                    break;
                }
                let updates = {
                    let Ok(guard) = state.session.lock() else {
                        break;
                    };
                    let Some(session) = guard.as_ref() else { break };
                    if session.generation != generation {
                        break;
                    }
                    let target = window_top::window(id, pid);
                    match target {
                        Ok(target) if !target.minimized => Some(
                            session
                                .frames
                                .iter()
                                .filter_map(|(label, monitor)| {
                                    focus_window::cutout(
                                        monitor.bounds,
                                        target.bounds,
                                        monitor.scale,
                                    )
                                    .ok()
                                    .map(|cutout| {
                                        (
                                            label.clone(),
                                            FocusFrame {
                                                cutout,
                                                opacity: session.opacity,
                                            },
                                        )
                                    })
                                })
                                .collect::<Vec<_>>(),
                        ),
                        _ => None,
                    }
                };
                let Some(updates) = updates else {
                    close_generation(&tracker, Some(generation));
                    break;
                };
                for (label, frame) in updates {
                    let _ = tracker.emit_to(&label, "focus://frame", frame);
                }
            }
        });
    match worker {
        Ok(worker) => {
            let mut workers = match state.workers.lock() {
                Ok(workers) => workers,
                Err(_) => {
                    close_generation(&app, Some(generation));
                    let _ = worker.join();
                    return Err(state_error());
                }
            };
            if state.quitting.load(Ordering::Acquire) {
                drop(workers);
                let _ = worker.join();
                close_generation(&app, Some(generation));
                return Err(CoreError::Cancelled.into());
            }
            workers.push(worker);
        }
        Err(error) => {
            close_generation(&app, Some(generation));
            return Err(ToolError::internal(error.to_string()));
        }
    }
    Ok(())
}

//! Tauri adapters and lifecycle for local system/power tools.
use crate::error::ToolError;
use oxbox_core::{
    CoreError, CoreResult,
    tools::{
        break_time::{BreakScheduler, BreakSettings, BreakStatus},
        hide_desktop::DesktopState,
        keep_awake::{AwakeController, AwakeStatus},
        lock,
        network::{InterfaceUsage, NetworkMonitor},
        sleep_timer::{self, PowerAction, SleepTimer, TimerEvent, TimerStatus},
        system_monitor::{Monitor, Snapshot},
    },
};
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

struct PowerState {
    awake: Option<AwakeController>,
    timer: SleepTimer,
    breaks: BreakScheduler,
    break_settings: BreakSettings,
    desktop: DesktopState,
    anchor: Instant,
    epoch: u64,
    last_error: Option<String>,
    clock_notification: Option<Instant>,
}
impl Default for PowerState {
    fn default() -> Self {
        Self {
            awake: None,
            timer: SleepTimer::default(),
            breaks: BreakScheduler::default(),
            break_settings: BreakSettings::default(),
            desktop: DesktopState::default(),
            anchor: Instant::now(),
            epoch: unix_now(),
            last_error: None,
            clock_notification: None,
        }
    }
}
impl PowerState {
    fn now(&self) -> u64 {
        self.epoch.saturating_add(self.anchor.elapsed().as_secs())
    }
    fn awake_status(&self) -> CoreResult<AwakeStatus> {
        match &self.awake {
            Some(a) => a.status(),
            None => Ok(AwakeStatus::default()),
        }
    }
}
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
fn mutex<T>(m: &Mutex<T>) -> CoreResult<MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| CoreError::InvalidInput("Tool state is unavailable. Restart Oxbox.".into()))
}

pub struct SystemTools {
    monitor: Mutex<Monitor>,
    network: Mutex<Option<NetworkMonitor>>,
    power: Arc<Mutex<PowerState>>,
    stopping: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Default for SystemTools {
    fn default() -> Self {
        Self {
            monitor: Mutex::new(Monitor::default()),
            network: Mutex::new(None),
            power: Arc::new(Mutex::new(PowerState::default())),
            stopping: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        }
    }
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(SystemTools::default());
    let tools = app.state::<SystemTools>();
    if let Ok(store) = app.store("settings.json")
        && let Some(value) = store.get("breakTime")
        && let Ok(mut settings) = serde_json::from_value::<BreakSettings>(value)
    {
        let mut state = mutex(&tools.power)?;
        if settings.enabled {
            let now = state.now();
            if let Err(error) = state.breaks.start(now, settings.minutes, settings.eye_rest) {
                settings.enabled = false;
                let _ = app
                    .notification()
                    .builder()
                    .title("Break reminders are off")
                    .body(error.to_string())
                    .show();
            }
        }
        state.break_settings = settings;
    }
    let state = tools.power.clone();
    let stopping = tools.stopping.clone();
    let handle = app.clone();
    let worker = std::thread::Builder::new().name("oxbox-schedulers".into()).spawn(move || {
        let mut previous_second = 0;
        let mut previous_awake = false;
        while !stopping.load(Ordering::Acquire) {
            if let Ok(mut state) = state.lock() {
                // Notification-only fallback: the UI's performance.now() clock remains authoritative.
                // This deadline keeps Windows notifications working even if its WebView is suspended.
                if state.clock_notification.is_some_and(|deadline| Instant::now() >= deadline) {
                    state.clock_notification = None;
                    if !stopping.load(Ordering::Acquire) {
                        let _ = handle.notification().builder().title("Oxbox timer").body("Your countdown is complete.").show();
                    }
                }
                let now = state.now();
                if now != previous_second {
                    previous_second = now;
                    match state.timer.tick(now) {
                        TimerEvent::Warning => { let _ = handle.notification().builder().title("Sleep Timer").body("The scheduled power action is due within 60 seconds. Open Oxbox to cancel.").show(); }
                        TimerEvent::Execute(action) => {
                            // Cancellation wins until the deadline; never queue a detached OS shutdown.
                            if !stopping.load(Ordering::Acquire) && let Err(error) = sleep_timer::execute(action) {
                                state.last_error = Some(error.to_string());
                                let _ = handle.notification().builder().title("Sleep Timer could not run").body(error.to_string()).show();
                            }
                            let _ = handle.emit("tool://sleep-timer/status", state.timer.status());
                        }
                        TimerEvent::None => {}
                    }
                    let inactive = state.break_settings.enabled && oxbox_core::platform::activity_paused(state.break_settings.pause_fullscreen);
                    if state.breaks.tick(now, inactive) {
                        let body = if state.break_settings.eye_rest { "Look at something 20 feet away for 20 seconds. Open Break Time to continue or snooze." } else { "Stand, stretch, and rest your eyes. Open Break Time to continue or snooze." };
                        let _ = handle.notification().builder().title("Time for a break").body(body).show();
                        let _ = handle.emit("tool://break-time/status", state.breaks.status(now));
                    }
                    let active = state.awake_status().is_ok_and(|s| s.active);
                    if active != previous_awake {
                        previous_awake = active;
                        if let Some(tray) = handle.tray_by_id("oxbox-tray") { let _ = tray.set_tooltip(Some(if active { "Oxbox — Do Not Sleep is on" } else { "Oxbox" })); }
                        let _ = handle.emit("tool://do-not-sleep/status", state.awake_status().ok());
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    })?;
    *mutex(&tools.worker)? = Some(worker);
    Ok(())
}

pub fn shutdown(app: &AppHandle) {
    let Some(tools) = app.try_state::<SystemTools>() else {
        return;
    };
    tools.stopping.store(true, Ordering::Release);
    if let Ok(mut worker) = tools.worker.lock()
        && let Some(worker) = worker.take()
    {
        let _ = worker.join();
    }
    if let Ok(mut state) = tools.power.lock() {
        state.timer.cancel();
        state.clock_notification = None;
        state.breaks.stop();
        if let Some(mut awake) = state.awake.take() {
            awake.shutdown();
        }
        let _ = state.desktop.restore();
    }
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> Result<T, ToolError> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| ToolError::internal(e.to_string()))?
        .map_err(Into::into)
}
#[tauri::command]
pub async fn system_snapshot(app: AppHandle) -> Result<Snapshot, ToolError> {
    blocking(move || Ok(mutex(&app.state::<SystemTools>().monitor)?.snapshot())).await
}
#[tauri::command]
pub async fn network_sample(app: AppHandle) -> Result<Vec<InterfaceUsage>, ToolError> {
    blocking(move || {
        let tools = app.state::<SystemTools>();
        let mut network = mutex(&tools.network)?;
        Ok(network.get_or_insert_with(NetworkMonitor::default).sample())
    })
    .await
}
#[tauri::command]
pub async fn awake_start(
    app: AppHandle,
    display: bool,
    minutes: Option<u64>,
    until: Option<u64>,
) -> Result<AwakeStatus, ToolError> {
    blocking(move || {
        let tools = app.state::<SystemTools>();
        let mut state = mutex(&tools.power)?;
        // A queued blocking task may outlive Quit's cleanup; validate while owning the state lock.
        if tools.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        if state.awake.is_none() {
            state.awake = Some(AwakeController::new()?);
        }
        let now = unix_now();
        state
            .awake
            .as_ref()
            .ok_or_else(|| CoreError::InvalidInput("Keep-awake initialization failed.".into()))?
            .start(now, display, minutes, until)
    })
    .await
}
#[tauri::command]
pub async fn awake_stop(app: AppHandle) -> Result<(), ToolError> {
    blocking(move || {
        let tools = app.state::<SystemTools>();
        let state = mutex(&tools.power)?;
        if let Some(awake) = &state.awake {
            awake.stop()?;
        }
        Ok(())
    })
    .await
}
#[tauri::command]
pub fn awake_status(tools: State<'_, SystemTools>) -> Result<AwakeStatus, ToolError> {
    Ok(mutex(&tools.power)?.awake_status()?)
}
/// Arm or cancel only the countdown's native notification. No elapsed time is polled by the UI.
#[tauri::command]
pub fn timer_notification_set(
    tools: State<'_, SystemTools>,
    milliseconds: Option<u64>,
) -> Result<(), ToolError> {
    let deadline = milliseconds
        .map(|milliseconds| {
            if milliseconds > 7 * 24 * 3600 * 1000 {
                return Err(CoreError::InvalidInput(
                    "Timer duration exceeds seven days.".into(),
                ));
            }
            Instant::now()
                .checked_add(Duration::from_millis(milliseconds))
                .ok_or_else(|| {
                    CoreError::InvalidInput("Timer duration exceeds the clock range.".into())
                })
        })
        .transpose()?;
    let mut state = mutex(&tools.power)?;
    if deadline.is_some() && tools.stopping.load(Ordering::Acquire) {
        return Err(CoreError::Cancelled.into());
    }
    state.clock_notification = deadline;
    Ok(())
}
#[tauri::command]
pub async fn lock_screen(app: AppHandle) -> Result<(), ToolError> {
    blocking(move || {
        let tools = app.state::<SystemTools>();
        let _state = mutex(&tools.power)?;
        if tools.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        lock::lock()
    })
    .await
}

#[tauri::command]
pub fn sleep_arm(
    app: AppHandle,
    seconds: u64,
    action: PowerAction,
    confirmed: bool,
) -> Result<TimerStatus, ToolError> {
    if !confirmed {
        return Err(CoreError::InvalidInput(
            "Confirm the power action before arming the timer.".into(),
        )
        .into());
    }
    let tools = app.state::<SystemTools>();
    let mut state = mutex(&tools.power)?;
    if tools.stopping.load(Ordering::Acquire) {
        return Err(CoreError::Cancelled.into());
    }
    // Round the arming instant up so sub-second clock precision never runs a power action early.
    let now = state.now().saturating_add(1);
    state.timer.arm(now, seconds, action)?;
    state.last_error = None;
    let status = state
        .timer
        .status()
        .ok_or_else(|| CoreError::InvalidInput("Timer was not armed.".into()))?;
    let _ = app.emit("tool://sleep-timer/status", Some(status));
    Ok(status)
}
#[tauri::command]
pub fn sleep_cancel(app: AppHandle) -> Result<(), ToolError> {
    let tools = app.state::<SystemTools>();
    mutex(&tools.power)?.timer.cancel();
    let _ = app.emit("tool://sleep-timer/status", Option::<TimerStatus>::None);
    Ok(())
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepStatus {
    pub timer: Option<TimerStatus>,
    pub error: Option<String>,
    pub remaining_seconds: u64,
}
#[tauri::command]
pub fn sleep_status(tools: State<'_, SystemTools>) -> Result<SleepStatus, ToolError> {
    let state = mutex(&tools.power)?;
    Ok(SleepStatus {
        timer: state.timer.status(),
        error: state.last_error.clone(),
        remaining_seconds: state.timer.remaining(state.now()),
    })
}

#[tauri::command]
pub fn break_configure(app: AppHandle, settings: BreakSettings) -> Result<BreakStatus, ToolError> {
    let tools = app.state::<SystemTools>();
    let mut state = mutex(&tools.power)?;
    if settings.enabled && tools.stopping.load(Ordering::Acquire) {
        return Err(CoreError::Cancelled.into());
    }
    let now = state.now();
    // Validate before writing or changing a running schedule.
    let mut candidate = BreakScheduler::default();
    candidate.start(now, settings.minutes, settings.eye_rest)?;
    if !settings.enabled {
        candidate.stop();
    }
    let store = app
        .store("settings.json")
        .map_err(|e| ToolError::internal(e.to_string()))?;
    let previous = store.get("breakTime");
    store.set(
        "breakTime",
        serde_json::to_value(&settings).map_err(|e| ToolError::internal(e.to_string()))?,
    );
    if let Err(error) = store.save() {
        if let Some(value) = previous {
            store.set("breakTime", value);
        } else {
            store.delete("breakTime");
        }
        return Err(ToolError::internal(error.to_string()));
    }
    state.breaks = candidate;
    state.break_settings = settings;
    Ok(state.breaks.status(now))
}
#[derive(serde::Serialize)]
pub struct BreakInfo {
    pub settings: BreakSettings,
    pub status: BreakStatus,
}
#[tauri::command]
pub fn break_status(tools: State<'_, SystemTools>) -> Result<BreakInfo, ToolError> {
    let state = mutex(&tools.power)?;
    Ok(BreakInfo {
        settings: state.break_settings.clone(),
        status: state.breaks.status(state.now()),
    })
}
#[tauri::command]
pub fn break_snooze(tools: State<'_, SystemTools>) -> Result<(), ToolError> {
    let mut state = mutex(&tools.power)?;
    let now = state.now();
    state.breaks.snooze(now);
    Ok(())
}
#[tauri::command]
pub fn break_skip(tools: State<'_, SystemTools>) -> Result<(), ToolError> {
    let mut state = mutex(&tools.power)?;
    let now = state.now();
    state.breaks.skip(now);
    Ok(())
}

#[tauri::command]
pub async fn desktop_set_hidden(app: AppHandle, hidden: bool) -> Result<bool, ToolError> {
    blocking(move || {
        let tools = app.state::<SystemTools>();
        let mut state = mutex(&tools.power)?;
        if hidden && tools.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        if hidden {
            state.desktop.hide()?;
        } else {
            state.desktop.restore()?;
        }
        Ok(state.desktop.is_hidden())
    })
    .await
}
#[tauri::command]
pub fn desktop_status(tools: State<'_, SystemTools>) -> Result<bool, ToolError> {
    Ok(mutex(&tools.power)?.desktop.is_hidden())
}

pub fn toggle_awake(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let active = {
            let tools = app.state::<SystemTools>();
            mutex(&tools.power)
                .and_then(|s| s.awake_status())
                .is_ok_and(|s| s.active)
        };
        let result = if active {
            awake_stop(app.clone()).await.map(|_| ())
        } else {
            awake_start(app.clone(), false, None, None)
                .await
                .map(|_| ())
        };
        if let Err(error) = result {
            let _ = app.emit("tool://system/error", error);
        }
    });
}
pub fn lock_now(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = lock_screen(app.clone()).await {
            let _ = app.emit("tool://system/error", error);
        }
    });
}
pub fn toggle_desktop(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let hidden = {
            let tools = app.state::<SystemTools>();
            mutex(&tools.power)
                .map(|s| s.desktop.is_hidden())
                .unwrap_or(false)
        };
        if let Err(error) = desktop_set_hidden(app.clone(), !hidden).await {
            let _ = app.emit("tool://system/error", error);
        }
    });
}

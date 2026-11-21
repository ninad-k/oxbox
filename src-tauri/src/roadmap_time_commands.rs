//! Phase 5 adapters for presentation sessions, user-owned system controls, and durable native reminders.
use crate::error::ToolError;
use oxbox_core::{
    CoreError, CoreResult,
    platform::Os,
    tools::{
        airplane_mode,
        alarm::{Alarm, AlarmBook, AlarmInput},
        date_countdown::{Countdown, CountdownBook, CountdownInput, CountdownSnapshot},
        do_not_disturb::{self, ControlCapability},
        presentation_mode::{self, PresentationOptions, PresentationSession, PresentationStatus},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Stored {
    alarms: Vec<Alarm>,
    countdowns: Vec<Countdown>,
    presentation: PresentationOptions,
}
struct TimeState {
    alarms: AlarmBook,
    countdowns: CountdownBook,
    presentation: PresentationSession,
    error: Option<String>,
    presentation_error: Option<String>,
    storage_ready: bool,
}
impl Default for TimeState {
    fn default() -> Self {
        let mut presentation = PresentationSession::default();
        if Os::current() == Os::Linux {
            let _ = presentation.configure(PresentationOptions {
                hide_desktop: false,
                ..PresentationOptions::default()
            });
        }
        Self {
            alarms: AlarmBook::default(),
            countdowns: CountdownBook::default(),
            presentation,
            error: None,
            presentation_error: None,
            storage_ready: false,
        }
    }
}
impl TimeState {
    fn stored(&self) -> Stored {
        Stored {
            alarms: self.alarms.items.clone(),
            countdowns: self.countdowns.items.clone(),
            presentation: self.presentation.status().options,
        }
    }
    fn ready(&self) -> CoreResult<()> {
        if self.storage_ready {
            Ok(())
        } else {
            Err(CoreError::InvalidInput("Reminder storage could not be loaded. Back up and repair settings.json, then restart Oxbox before editing reminders.".into()))
        }
    }
    fn presentation_info(&self) -> PresentationInfo {
        PresentationInfo {
            status: self.presentation.status(),
            error: self.presentation_error.clone().or_else(|| {
                if self.storage_ready {
                    None
                } else {
                    self.error.clone()
                }
            }),
        }
    }
}
#[derive(Serialize)]
pub struct PresentationInfo {
    #[serde(flatten)]
    status: PresentationStatus,
    error: Option<String>,
}
pub struct RoadmapTimeState {
    inner: Arc<Mutex<TimeState>>,
    stopping: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Default for RoadmapTimeState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TimeState::default())),
            stopping: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        }
    }
}
fn lock<T>(mutex: &Mutex<T>) -> CoreResult<MutexGuard<'_, T>> {
    mutex.lock().map_err(|_| {
        CoreError::InvalidInput("Reminder state is unavailable. Restart Oxbox.".into())
    })
}
fn now() -> i64 {
    chrono_now()
}
fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs().min(i64::MAX as u64) as i64)
}
fn persist(app: &AppHandle, data: &Stored) -> CoreResult<()> {
    let store = app.store("settings.json").map_err(|error| {
        CoreError::InvalidInput(format!("Could not open reminder storage: {error}"))
    })?;
    let previous = store.get("roadmapTime");
    let value =
        serde_json::to_value(data).map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    store.set("roadmapTime", value);
    if let Err(error) = store.save() {
        if let Some(previous) = previous {
            store.set("roadmapTime", previous);
        } else {
            store.delete("roadmapTime");
        }
        return Err(CoreError::InvalidInput(format!(
            "Could not save reminders: {error}"
        )));
    }
    Ok(())
}
pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    app.manage(RoadmapTimeState::default());
    let tools = app.state::<RoadmapTimeState>();
    {
        let mut state = lock(&tools.inner)?;
        let loaded = (|| -> CoreResult<Option<Stored>> {
            let store = app
                .store("settings.json")
                .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
            store
                .get("roadmapTime")
                .map(serde_json::from_value::<Stored>)
                .transpose()
                .map_err(|error| {
                    CoreError::InvalidInput(format!("Could not parse reminder storage: {error}"))
                })
        })();
        match loaded {
            Ok(Some(stored)) => {
                match (
                    AlarmBook::restore(stored.alarms),
                    CountdownBook::restore(stored.countdowns),
                ) {
                    (Ok(alarms), Ok(countdowns)) => {
                        state.alarms = alarms;
                        state.countdowns = countdowns;
                        state.presentation.configure(stored.presentation)?;
                        state.storage_ready = true;
                    }
                    (alarms, countdowns) => {
                        state.error = Some(format!(
                            "Invalid reminder storage: {}",
                            alarms
                                .err()
                                .or(countdowns.err())
                                .map_or_else(|| "unknown error".into(), |error| error.to_string())
                        ));
                    }
                }
            }
            Ok(None) => state.storage_ready = true,
            Err(error) => state.error = Some(error.to_string()),
        }
    }
    let inner = tools.inner.clone();
    let stopping = tools.stopping.clone();
    let handle = app.clone();
    let worker=std::thread::Builder::new().name("oxbox-roadmap-reminders".into()).spawn(move||{
        let mut last_second=i64::MIN;let mut next_display_check=Instant::now();
        while !stopping.load(Ordering::Acquire) {
            if let Ok(mut state)=inner.lock(){let timestamp=now();if timestamp!=last_second&&state.storage_ready {last_second=timestamp;
                let old_alarms=state.alarms.clone();let old_countdowns=state.countdowns.clone();let alarms=state.alarms.tick(timestamp);let countdowns=state.countdowns.tick(timestamp);
                if !alarms.is_empty()||!countdowns.is_empty(){match persist(&handle,&state.stored()) {
                    Ok(())=>{state.error=None;if !stopping.load(Ordering::Acquire){for alarm in alarms {if let Err(error)=handle.notification().builder().title("Oxbox alarm").body(&alarm.input.label).show(){state.error=Some(format!("An alarm is due, but its system notification failed: {error}"));}}for countdown in countdowns{if let Err(error)=handle.notification().builder().title("Date reached").body(&countdown.input.label).show(){state.error=Some(format!("A date was reached, but its system notification failed: {error}"));}}let _=handle.emit("tool://roadmap-time/updated",());}},
                    Err(error)=>{state.alarms=old_alarms;state.countdowns=old_countdowns;state.error=Some(error.to_string());},
                }}
                if state.presentation.status().options.auto_external&&Instant::now()>=next_display_check {
                    next_display_check=Instant::now()+Duration::from_secs(5);match presentation_mode::connected_displays().and_then(|count|state.presentation.monitor_count(count)){Ok(())=>state.presentation_error=None,Err(error)=>state.presentation_error=Some(error.to_string())}
                }
            }}
            std::thread::sleep(Duration::from_millis(250));
        }
    })?;
    *lock(&tools.worker)? = Some(worker);
    Ok(())
}
pub fn shutdown(app: &AppHandle) {
    if let Some(tools) = app.try_state::<RoadmapTimeState>() {
        tools.stopping.store(true, Ordering::Release);
        if let Ok(mut worker) = tools.worker.lock()
            && let Some(worker) = worker.take()
        {
            let _ = worker.join();
        }
        if let Ok(mut state) = tools.inner.lock() {
            let _ = state.presentation.stop(false);
        }
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
#[tauri::command]
pub fn dnd_capability() -> ControlCapability {
    do_not_disturb::capability(Os::current())
}
#[tauri::command]
pub async fn dnd_open_settings() -> Result<(), ToolError> {
    blocking(do_not_disturb::open_settings).await
}
#[tauri::command]
pub fn airplane_capability() -> ControlCapability {
    airplane_mode::capability(Os::current())
}
#[tauri::command]
pub async fn airplane_open_settings() -> Result<(), ToolError> {
    blocking(airplane_mode::open_settings).await
}
#[tauri::command]
pub fn presentation_status(
    tools: State<'_, RoadmapTimeState>,
) -> Result<PresentationInfo, ToolError> {
    Ok(lock(&tools.inner)?.presentation_info())
}
#[tauri::command]
pub async fn presentation_configure(
    app: AppHandle,
    options: PresentationOptions,
) -> Result<PresentationInfo, ToolError> {
    blocking(move || {
        let tools = app.state::<RoadmapTimeState>();
        let mut state = lock(&tools.inner)?;
        if tools.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        state.ready()?;
        let old = state.presentation.status().options;
        state.presentation.configure(options)?;
        if let Err(error) = persist(&app, &state.stored()) {
            let _ = state.presentation.configure(old);
            return Err(error);
        }
        state.presentation_error = None;
        Ok(state.presentation_info())
    })
    .await
}
#[tauri::command]
pub async fn presentation_start(
    app: AppHandle,
    options: PresentationOptions,
) -> Result<PresentationInfo, ToolError> {
    blocking(move || {
        let tools = app.state::<RoadmapTimeState>();
        let mut state = lock(&tools.inner)?;
        if tools.stopping.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        state.ready()?;
        if state.presentation.status().active {
            return Ok(state.presentation_info());
        }
        let old = state.presentation.status().options;
        state.presentation.start(options, false)?;
        if let Err(error) = persist(&app, &state.stored()) {
            let _ = state.presentation.stop(false);
            let _ = state.presentation.configure(old);
            return Err(error);
        }
        state.presentation_error = None;
        Ok(state.presentation_info())
    })
    .await
}
#[tauri::command]
pub async fn presentation_stop(app: AppHandle) -> Result<PresentationInfo, ToolError> {
    blocking(move || {
        let tools = app.state::<RoadmapTimeState>();
        let mut state = lock(&tools.inner)?;
        state.presentation.stop(true)?;
        state.presentation_error = None;
        Ok(state.presentation_info())
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSnapshot {
    alarms: Vec<Alarm>,
    countdowns: Vec<CountdownSnapshot>,
    error: Option<String>,
}
#[tauri::command]
pub fn roadmap_time_snapshot(
    tools: State<'_, RoadmapTimeState>,
) -> Result<TimeSnapshot, ToolError> {
    let state = lock(&tools.inner)?;
    Ok(TimeSnapshot {
        alarms: state.alarms.items.clone(),
        countdowns: state.countdowns.snapshot(now()),
        error: state.error.clone(),
    })
}
fn edit<T>(app: &AppHandle, change: impl FnOnce(&mut TimeState) -> CoreResult<T>) -> CoreResult<T> {
    let tools = app.state::<RoadmapTimeState>();
    let mut state = lock(&tools.inner)?;
    state.ready()?;
    let old_alarms = state.alarms.clone();
    let old_countdowns = state.countdowns.clone();
    let result = change(&mut state)?;
    if let Err(error) = persist(app, &state.stored()) {
        state.alarms = old_alarms;
        state.countdowns = old_countdowns;
        return Err(error);
    }
    state.error = None;
    Ok(result)
}
#[tauri::command]
pub async fn alarm_add(app: AppHandle, input: AlarmInput) -> Result<u64, ToolError> {
    blocking(move || edit(&app, |state| state.alarms.add(input, now()))).await
}
#[tauri::command]
pub async fn alarm_remove(app: AppHandle, id: u64) -> Result<(), ToolError> {
    blocking(move || edit(&app, |state| state.alarms.remove(id))).await
}
#[tauri::command]
pub async fn alarm_snooze(app: AppHandle, id: u64, minutes: u64) -> Result<(), ToolError> {
    blocking(move || edit(&app, |state| state.alarms.snooze(id, now(), minutes))).await
}
#[tauri::command]
pub async fn alarm_dismiss(app: AppHandle, id: u64) -> Result<(), ToolError> {
    blocking(move || edit(&app, |state| state.alarms.dismiss(id))).await
}
#[tauri::command]
pub async fn alarm_set_enabled(app: AppHandle, id: u64, enabled: bool) -> Result<(), ToolError> {
    blocking(move || edit(&app, |state| state.alarms.set_enabled(id, enabled, now()))).await
}
#[tauri::command]
pub async fn countdown_add(app: AppHandle, input: CountdownInput) -> Result<u64, ToolError> {
    blocking(move || edit(&app, |state| state.countdowns.add(input, now()))).await
}
#[tauri::command]
pub async fn countdown_remove(app: AppHandle, id: u64) -> Result<(), ToolError> {
    blocking(move || edit(&app, |state| state.countdowns.remove(id))).await
}

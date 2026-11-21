//! Native host ownership for core system controllers. No effects occur on construction.
use eframe::egui;
use oxbox_core::{
    CoreError, CoreResult,
    tools::{
        hide_desktop::DesktopState,
        keep_awake::{AwakeController, AwakeStatus},
        mouse_activity,
        presentation_mode::{self, PresentationOptions, PresentationSession},
        resolution::{self, DisplayMode},
        show_desktop,
        sleep_timer::{self, PowerAction, SleepTimer, TimerEvent},
        window_manager::{self, Snap},
        window_top,
    },
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub fn error(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
pub fn lock<T>(value: &Mutex<T>) -> CoreResult<MutexGuard<'_, T>> {
    value
        .lock()
        .map_err(|_| error("System state is unavailable. Restart Oxbox."))
}
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

struct Topmost {
    pid: u32,
    original: bool,
    last: bool,
}
pub struct PendingMode {
    pub original: DisplayMode,
    pub expires: Option<Instant>,
}
impl PendingMode {
    pub fn remaining(&self) -> u64 {
        let remaining = self.expires.map_or(Duration::ZERO, |d| {
            d.saturating_duration_since(Instant::now())
        });
        remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0)
    }
}
pub struct State {
    pub awake: Option<AwakeController>,
    pub desktop: DesktopState,
    pub timer: SleepTimer,
    pub anchor: Instant,
    pub epoch: u64,
    pub presentation: PresentationSession,
    pub pending: Option<PendingMode>,
    pub focus: Option<super::focus::Focus>,
    pub error: Option<String>,
    pub resolution_outcome: Option<String>,
    topmost: HashMap<u64, Topmost>,
    minimized: Vec<(u64, u32)>,
}
impl Default for State {
    fn default() -> Self {
        let mut presentation = PresentationSession::default();
        if cfg!(target_os = "linux") {
            let _ = presentation.configure(PresentationOptions {
                hide_desktop: false,
                ..Default::default()
            });
        }
        Self {
            awake: None,
            desktop: DesktopState::default(),
            timer: SleepTimer::default(),
            anchor: Instant::now(),
            epoch: unix_now(),
            presentation,
            pending: None,
            focus: None,
            error: None,
            resolution_outcome: None,
            topmost: HashMap::new(),
            minimized: vec![],
        }
    }
}
impl State {
    fn start_awake(
        &mut self,
        display: bool,
        minutes: Option<u64>,
        until: Option<u64>,
    ) -> CoreResult<()> {
        if self.awake.is_none() {
            self.awake = Some(AwakeController::new()?);
        }
        self.awake
            .as_ref()
            .ok_or_else(|| error("Awake worker unavailable"))?
            .start(unix_now(), display, minutes, until)
            .map(|_| ())
    }
    fn set_desktop(&mut self, hidden: bool) -> CoreResult<String> {
        let presentation = self.presentation.status();
        if presentation.active && presentation.options.hide_desktop {
            return Err(error(
                "Stop Presentation Mode before changing desktop visibility.",
            ));
        }
        if hidden {
            self.desktop.hide()?;
        } else {
            self.desktop.restore()?;
        }
        Ok(if hidden {
            "Enabled Hide Desktop."
        } else {
            "Restored previous desktop visibility."
        }
        .into())
    }
    pub fn now(&self) -> u64 {
        self.epoch.saturating_add(self.anchor.elapsed().as_secs())
    }
    pub fn awake_status(&self) -> CoreResult<AwakeStatus> {
        self.awake
            .as_ref()
            .map_or(Ok(AwakeStatus::default()), AwakeController::status)
    }
    fn restore_mode(&mut self) -> CoreResult<()> {
        if let Some(pending) = &self.pending {
            resolution::restore(&pending.original)?;
        }
        self.pending = None;
        self.resolution_outcome = Some("Previous display mode restored.".into());
        Ok(())
    }
    pub fn cleanup(&mut self) -> CoreResult<()> {
        let mut failures = vec![];
        if let Some(mut focus) = self.focus.take() {
            focus.stop();
        }
        self.timer.cancel();
        if let Err(e) = self.restore_mode() {
            failures.push(e.to_string());
        }
        if let Err(e) = self.presentation.stop(false) {
            failures.push(e.to_string());
        }
        if let Err(e) = self.desktop.restore() {
            failures.push(e.to_string());
        }
        if let Some(mut awake) = self.awake.take() {
            awake.shutdown();
        }
        let restored = show_desktop::restore_tracked(&self.minimized);
        self.minimized = restored.remaining;
        if !self.minimized.is_empty() {
            failures.push(format!(
                "{} minimized windows still need restoration",
                self.minimized.len()
            ));
        }
        for (id, change) in std::mem::take(&mut self.topmost) {
            if window_top::window(id, change.pid).is_ok_and(|w| w.topmost == change.last)
                && let Err(e) = window_top::set(id, change.pid, change.original)
            {
                failures.push(e.to_string());
                self.topmost.insert(id, change);
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(error(format!(
                "Some system state could not be restored: {}",
                failures.join("; ")
            )))
        }
    }
}
pub struct Shared {
    pub state: Mutex<State>,
    pub stopping: AtomicBool,
    pub mouse: mouse_activity::Service,
    generation: AtomicU64,
    sleep_cancel: AtomicU64,
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            stopping: AtomicBool::new(false),
            mouse: Default::default(),
            generation: AtomicU64::new(1),
            sleep_cancel: AtomicU64::new(0),
        }
    }
}
impl Shared {
    pub fn reserve(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
    pub fn invalidate_pending(&self) -> u64 {
        self.generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1)
    }
    pub fn reserve_action(&self, action: &Action) -> u64 {
        if matches!(action, Action::SleepCancel) {
            self.prepare_recovery(crate::recovery::Action::CancelSleep)
        } else if action.recovery() {
            self.invalidate_pending()
        } else {
            self.reserve()
        }
    }
    pub fn prepare_recovery(&self, action: crate::recovery::Action) -> u64 {
        if action == crate::recovery::Action::Quit {
            return self.request_shutdown();
        }
        if matches!(
            action,
            crate::recovery::Action::ToggleAwake
                | crate::recovery::Action::ToggleDesktop
                | crate::recovery::Action::Lock
        ) {
            return self.reserve();
        }
        let generation = self.invalidate_pending();
        if action == crate::recovery::Action::CancelSleep {
            self.sleep_cancel.store(generation, Ordering::Release);
        }
        if action == crate::recovery::Action::StopMouse {
            let _ = self.mouse.request_stop();
        }
        generation
    }
    pub fn request_shutdown(&self) -> u64 {
        self.stopping.store(true, Ordering::Release);
        let generation = self.invalidate_pending();
        self.sleep_cancel.store(generation, Ordering::Release);
        self.mouse.request_shutdown();
        generation
    }
    pub fn apply_sleep_cancel(&self, state: &mut State) -> bool {
        let ticket = self.sleep_cancel.load(Ordering::Acquire);
        if ticket == 0 {
            return false;
        }
        state.timer.cancel();
        // A newer cancellation arriving during this acknowledgement remains pending.
        let _ = self
            .sleep_cancel
            .compare_exchange(ticket, 0, Ordering::AcqRel, Ordering::Acquire);
        true
    }
    fn timer_event(&self, state: &mut State, now: u64) -> TimerEvent {
        if self.apply_sleep_cancel(state) {
            TimerEvent::None
        } else {
            state.timer.tick(now)
        }
    }
    #[cfg(test)]
    pub fn presentation_change(
        &self,
        directory: &std::path::Path,
        options: PresentationOptions,
        start: bool,
    ) -> CoreResult<String> {
        self.presentation_change_reserved(directory, options, start, self.reserve())
    }
    pub fn presentation_change_reserved(
        &self,
        directory: &std::path::Path,
        options: PresentationOptions,
        start: bool,
        permit: u64,
    ) -> CoreResult<String> {
        self.with_reserved(permit, |state| {
            if state.presentation.status().active {
                return Err(error(
                    "Stop the current presentation before changing preferences.",
                ));
            }
            let old = state.presentation.status().options;
            if start {
                state.presentation.start(options.clone(), false)?;
            } else {
                state.presentation.configure(options.clone())?;
            }
            let result = (|| {
                let mut stored = oxbox_core::preferences::load::<super::Stored>(
                    directory,
                    "native-system.json",
                )?
                .ok_or_else(|| error("System preference storage is missing; restart Oxbox."))?;
                stored.presentation = options;
                crate::host::save_at(directory, "native-system.json", &stored)
            })();
            if let Err(e) = result {
                let mut rollback_errors = vec![];
                if start && let Err(restore) = state.presentation.stop(false) {
                    rollback_errors.push(format!("Presentation cleanup failed: {restore}. The remaining desktop lease is retained; use Stop presentation to retry"));
                }
                if let Err(restore) = state.presentation.configure(old) {
                    rollback_errors.push(format!("Previous presentation preferences could not be restored: {restore}"));
                }
                if !rollback_errors.is_empty() {
                    let message = format!("{e}. {}", rollback_errors.join(". "));
                    state.error = Some(message.clone());
                    return Err(error(message));
                }
                return Err(e);
            }
            Ok(if start {
                "Presentation started and preferences saved."
            } else {
                "Presentation preferences saved."
            }
            .into())
        })
    }
    pub fn with_live<T>(&self, work: impl FnOnce(&mut State) -> CoreResult<T>) -> CoreResult<T> {
        self.with_reserved(self.reserve(), work)
    }
    pub fn with_reserved<T>(
        &self,
        permit: u64,
        work: impl FnOnce(&mut State) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut state = lock(&self.state)?;
        if self.stopping.load(Ordering::Acquire) || self.reserve() != permit {
            return Err(CoreError::Cancelled);
        }
        work(&mut state)
    }
    pub fn perform(&self, action: Action) -> CoreResult<String> {
        self.perform_reserved(action, self.reserve())
    }
    pub fn perform_reserved(&self, action: Action, permit: u64) -> CoreResult<String> {
        let mut state = lock(&self.state)?;
        if (self.stopping.load(Ordering::Acquire) || self.reserve() != permit) && !action.recovery()
        {
            return Err(CoreError::Cancelled);
        }
        match action {
            Action::Awake {
                display,
                minutes,
                until,
            } => {
                state.start_awake(display, minutes, until)?;
                Ok("Started a keep-awake request.".into())
            }
            Action::AwakeToggle => {
                if state.awake_status()?.active {
                    self.invalidate_pending();
                    if let Some(awake) = &state.awake {
                        awake.stop()?;
                    }
                    Ok("Released the keep-awake request.".into())
                } else {
                    state.start_awake(false, None, None)?;
                    Ok("Started a keep-awake request until stopped.".into())
                }
            }
            Action::AwakeStop => {
                if let Some(awake) = &state.awake {
                    awake.stop()?;
                }
                Ok("Keep-awake request released.".into())
            }
            Action::Sleep { seconds, action } => {
                let now = state.now();
                state.timer.arm(now, seconds, action)?;
                state.error = None;
                Ok("Sleep Timer armed. Cancel from the persistent banner or tray.".into())
            }
            Action::SleepCancel => {
                state.timer.cancel();
                self.apply_sleep_cancel(&mut state);
                Ok("Sleep Timer cancelled.".into())
            }
            Action::Lock => {
                oxbox_core::tools::lock::lock()?;
                Ok("Lock request sent to the operating system.".into())
            }
            Action::Desktop(hidden) => state.set_desktop(hidden),
            Action::DesktopToggle => {
                let hidden = !state.desktop.is_hidden();
                if !hidden {
                    self.invalidate_pending();
                }
                state.set_desktop(hidden)
            }
            Action::PresentationStop => {
                state.presentation.stop(true)?;
                Ok("Presentation mode stopped and owned state restored.".into())
            }
            Action::Top { id, pid, enabled } => {
                let current = window_top::window(id, pid)?;
                let original = state
                    .topmost
                    .get(&id)
                    .filter(|v| v.pid == pid)
                    .map_or(current.topmost, |v| v.original);
                window_top::set(id, pid, enabled)?;
                if enabled == original {
                    state.topmost.remove(&id);
                } else {
                    state.topmost.insert(
                        id,
                        Topmost {
                            pid,
                            original,
                            last: enabled,
                        },
                    );
                }
                Ok("Window topmost state updated.".into())
            }
            Action::Snap { id, pid, placement } => {
                window_manager::snap(id, pid, placement)?;
                Ok("Window placement updated.".into())
            }
            Action::Minimize => {
                if !state.minimized.is_empty() {
                    return Err(error(
                        "Restore the windows from the preceding Show Desktop action first.",
                    ));
                }
                state.minimized = show_desktop::minimize()?;
                Ok(format!(
                    "Minimize requested for {} windows.",
                    state.minimized.len()
                ))
            }
            Action::RestoreWindows => {
                let outcome = show_desktop::restore_tracked(&state.minimized);
                state.minimized = outcome.remaining;
                if !state.minimized.is_empty() {
                    return Err(error(format!(
                        "{} window restore requests were refused. Retry Restore.",
                        state.minimized.len()
                    )));
                }
                Ok(format!("Restored {} tracked windows.", outcome.restored))
            }
            Action::TestMode(mode) => {
                if state.pending.is_some() {
                    return Err(error("Keep or restore the pending display test first."));
                }
                let original = resolution::displays()?
                    .into_iter()
                    .find(|d| d.device == mode.device)
                    .ok_or_else(|| error("Display disconnected; refresh the list."))?
                    .current;
                resolution::apply(&mode)?;
                state.pending = Some(PendingMode {
                    original,
                    expires: Some(Instant::now() + Duration::from_secs(15)),
                });
                state.error = None;
                state.resolution_outcome = None;
                Ok(
                    "Display mode applied for 15 seconds. Choose Keep or it will be restored."
                        .into(),
                )
            }
            Action::KeepMode => {
                let pending = state
                    .pending
                    .as_ref()
                    .ok_or_else(|| error("No display test is pending."))?;
                if pending
                    .expires
                    .is_none_or(|deadline| Instant::now() >= deadline)
                {
                    state.restore_mode()?;
                    return Err(error(
                        "The confirmation deadline expired; the previous mode was restored.",
                    ));
                }
                state.pending = None;
                state.resolution_outcome = Some("Display mode kept.".into());
                Ok("Display mode kept.".into())
            }
            Action::RestoreMode => {
                if state.pending.is_none() {
                    return Err(error("No display test is pending."));
                }
                state.restore_mode()?;
                Ok("Previous display mode restored.".into())
            }
            Action::Focus { id, pid, opacity } => {
                if state.focus.as_ref().is_some_and(|f| f.status().active) {
                    return Err(error("Stop the preceding focus session first."));
                }
                state.focus = Some(super::focus::Focus::start(id, pid, opacity)?);
                Ok("Focus dimming started. Stop from this tool or the persistent banner.".into())
            }
            Action::FocusStop => {
                if let Some(mut focus) = state.focus.take() {
                    focus.stop();
                }
                Ok("Focus dimming stopped.".into())
            }
        }
    }
}
#[derive(Clone)]
pub enum Action {
    Awake {
        display: bool,
        minutes: Option<u64>,
        until: Option<u64>,
    },
    AwakeStop,
    AwakeToggle,
    Sleep {
        seconds: u64,
        action: PowerAction,
    },
    SleepCancel,
    Lock,
    Desktop(bool),
    DesktopToggle,
    PresentationStop,
    Top {
        id: u64,
        pid: u32,
        enabled: bool,
    },
    Snap {
        id: u64,
        pid: u32,
        placement: Snap,
    },
    Minimize,
    RestoreWindows,
    TestMode(DisplayMode),
    KeepMode,
    RestoreMode,
    Focus {
        id: u64,
        pid: u32,
        opacity: f64,
    },
    FocusStop,
}
impl Action {
    fn recovery(&self) -> bool {
        matches!(
            self,
            Self::AwakeStop
                | Self::SleepCancel
                | Self::Desktop(false)
                | Self::PresentationStop
                | Self::RestoreWindows
                | Self::RestoreMode
                | Self::FocusStop
        )
    }
}

pub struct Runtime {
    pub shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    initialized: bool,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            shared: Arc::new(Shared::default()),
            worker: None,
            initialized: false,
        }
    }
}
impl Runtime {
    pub fn initialize(&mut self, host: &crate::host::Host, ctx: &egui::Context) -> CoreResult<()> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        let shared = self.shared.clone();
        let notifier = host.notifier.clone();
        let ctx = ctx.clone();
        self.worker = Some(
            std::thread::Builder::new()
                .name("oxbox-native-system".into())
                .spawn(move || {
                    let mut display_check = Instant::now();
                    let mut repaint = Instant::now();
                    while !shared.stopping.load(Ordering::Acquire) {
                        if let Ok(mut state) = shared.state.lock() {
                            // Quit can start while this worker is waiting for an action's lock.
                            if shared.stopping.load(Ordering::Acquire) {
                                break;
                            }
                            let now = state.now();
                            match shared.timer_event(&mut state, now) {
                                TimerEvent::Warning => notifier.notify(
                                    "Sleep Timer",
                                    "The scheduled power action is due within 60 seconds. Open Oxbox to cancel.",
                                ),
                                TimerEvent::Execute(action) => {
                                    if !shared.stopping.load(Ordering::Acquire)
                                        && !shared.apply_sleep_cancel(&mut state)
                                        && let Err(e) = sleep_timer::execute(action)
                                    {
                                        state.error = Some(e.to_string());
                                        notifier.notify("Sleep Timer could not run", e.to_string());
                                    }
                                }
                                TimerEvent::None => {}
                            }
                            if state
                                .pending
                                .as_ref()
                                .and_then(|p| p.expires)
                                .is_some_and(|deadline| Instant::now() >= deadline)
                                && let Err(e) = state.restore_mode()
                            {
                                if let Some(p) = &mut state.pending {
                                    p.expires = None;
                                }
                                state.error = Some(format!(
                                    "Automatic display restore failed: {e}. Use Restore to retry."
                                ));
                                notifier.notify("Display restore needs attention", e.to_string());
                            }
                            if state.presentation.status().options.auto_external
                                && display_check.elapsed() >= Duration::from_secs(2)
                            {
                                display_check = Instant::now();
                                let result = presentation_mode::connected_displays().and_then(|count| {
                                    if shared.stopping.load(Ordering::Acquire) {
                                        return Err(CoreError::Cancelled);
                                    }
                                    state.presentation.monitor_count(count)
                                });
                                if let Err(e) = result
                                    && !matches!(e, CoreError::Cancelled)
                                {
                                    state.error = Some(e.to_string());
                                }
                            }
                        }
                        if repaint.elapsed() >= Duration::from_secs(1) {
                            ctx.request_repaint();
                            repaint = Instant::now();
                        }
                        std::thread::park_timeout(Duration::from_millis(150));
                    }
                })?,
        );
        Ok(())
    }
    pub fn shutdown(&mut self) {
        self.shared.stopping.store(true, Ordering::Release);
        self.shared.mouse.shutdown();
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
        let mut state = self.shared.state.lock().unwrap_or_else(|p| p.into_inner());
        let _ = state.cleanup();
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preparing_consecutive_tray_toggles_preserves_both_accepted_commands() {
        let shared = Shared::default();
        let first = shared.prepare_recovery(crate::recovery::Action::ToggleAwake);
        let second = shared.prepare_recovery(crate::recovery::Action::ToggleAwake);
        assert_eq!(first, second);
        assert_eq!(shared.reserve(), first);
    }
    #[test]
    fn delayed_ui_start_captured_before_stop_cannot_rearm_after_acknowledgement() {
        let shared = Arc::new(Shared::default());
        let old_permit = shared.reserve();
        let controls = super::super::SystemRecovery {
            shared: shared.clone(),
        };
        let stop = controls.prepare(crate::recovery::Action::CancelSleep);
        controls
            .execute(crate::recovery::Action::CancelSleep, stop)
            .unwrap();
        assert_eq!(
            shared
                .perform_reserved(
                    Action::Sleep {
                        seconds: 60,
                        action: PowerAction::Shutdown
                    },
                    old_permit
                )
                .unwrap_err()
                .code(),
            "cancelled"
        );
        assert!(lock(&shared.state).unwrap().timer.status().is_none());
        let fresh = shared.reserve();
        shared
            .perform_reserved(
                Action::Sleep {
                    seconds: 60,
                    action: PowerAction::Shutdown,
                },
                fresh,
            )
            .unwrap();
        assert!(lock(&shared.state).unwrap().timer.status().is_some());
        assert!(!shared.apply_sleep_cancel(&mut lock(&shared.state).unwrap()));
        shared.perform(Action::SleepCancel).unwrap();
    }
    #[test]
    fn immediate_sleep_cancel_prevents_due_event_without_polling_ui_or_recovery_queue() {
        let shared = Shared::default();
        let mut state = lock(&shared.state).unwrap();
        state.timer.arm(100, 60, PowerAction::Shutdown).unwrap();
        shared.prepare_recovery(crate::recovery::Action::CancelSleep);
        assert!(matches!(
            shared.timer_event(&mut state, 160),
            TimerEvent::None
        ));
        assert!(state.timer.status().is_none());
        state.timer.arm(100, 60, PowerAction::Shutdown).unwrap();
        assert!(matches!(state.timer.tick(160), TimerEvent::Execute(_)));
        shared.prepare_recovery(crate::recovery::Action::CancelSleep);
        assert!(
            shared.apply_sleep_cancel(&mut state),
            "late intent must still guard the native execute call"
        );
    }
    #[test]
    fn permit_is_rechecked_under_gate_before_delayed_mouse_or_presentation_start() {
        let shared = Shared::default();
        let permit = shared.reserve();
        let invoked = AtomicBool::new(false);
        shared.invalidate_pending();
        assert_eq!(
            shared
                .with_reserved(permit, |_| {
                    invoked.store(true, Ordering::Release);
                    Ok(())
                })
                .unwrap_err()
                .code(),
            "cancelled"
        );
        assert!(!invoked.load(Ordering::Acquire));
        assert!(lock(&shared.state).unwrap().awake.is_none());
    }
    #[test]
    fn quitting_rejects_effects_before_any_native_backend_is_created() {
        let shared = Shared::default();
        shared.stopping.store(true, Ordering::Release);
        for action in [
            Action::Lock,
            Action::Desktop(true),
            Action::DesktopToggle,
            Action::AwakeToggle,
            Action::Awake {
                display: true,
                minutes: None,
                until: None,
            },
            Action::Minimize,
            Action::Focus {
                id: 1,
                pid: 1,
                opacity: 0.5,
            },
            Action::Top {
                id: 1,
                pid: 1,
                enabled: true,
            },
        ] {
            assert_eq!(shared.perform(action).unwrap_err().code(), "cancelled");
        }
        assert!(lock(&shared.state).unwrap().awake.is_none());
        assert!(lock(&shared.state).unwrap().focus.is_none());
    }
    #[test]
    fn queued_work_rechecks_quit_after_acquiring_the_managed_lock() {
        let shared = Arc::new(Shared::default());
        let invoked = Arc::new(AtomicBool::new(false));
        let held = shared.state.lock().unwrap();
        let worker_shared = shared.clone();
        let called = invoked.clone();
        let worker = std::thread::spawn(move || {
            worker_shared.with_live(|_| {
                called.store(true, Ordering::Release);
                Ok(())
            })
        });
        shared.stopping.store(true, Ordering::Release);
        drop(held);
        assert_eq!(worker.join().unwrap().unwrap_err().code(), "cancelled");
        assert!(!invoked.load(Ordering::Acquire));
    }
    #[test]
    fn recovery_can_cancel_a_fake_timer_after_quit_without_executing_power() {
        let shared = Shared::default();
        shared
            .perform(Action::Sleep {
                seconds: 60,
                action: PowerAction::Shutdown,
            })
            .unwrap();
        shared.stopping.store(true, Ordering::Release);
        shared.perform(Action::SleepCancel).unwrap();
        assert!(lock(&shared.state).unwrap().timer.status().is_none());
    }
    #[test]
    fn presentation_preferences_merge_the_latest_saved_app_sets() {
        let folder = tempfile::tempdir().unwrap();
        let mut stored = super::super::Stored::default();
        stored.sets.push(oxbox_core::tools::launch_apps::AppSet {
            id: "saved-work".into(),
            name: "Saved work".into(),
            entries: vec![oxbox_core::tools::launch_apps::LaunchEntry {
                kind: oxbox_core::tools::launch_apps::LaunchKind::File,
                target: folder.path().join("notes.txt").to_string_lossy().into(),
                args: vec![],
            }],
        });
        oxbox_core::preferences::save(folder.path(), "native-system.json", &stored).unwrap();
        let shared = Shared::default();
        let options = PresentationOptions {
            display: false,
            hide_desktop: false,
            auto_external: true,
        };
        shared
            .presentation_change(folder.path(), options, false)
            .unwrap();
        let saved: super::super::Stored =
            oxbox_core::preferences::load(folder.path(), "native-system.json")
                .unwrap()
                .unwrap();
        assert_eq!(saved.sets[0].id, "saved-work");
        assert!(saved.presentation.auto_external);
        assert!(!lock(&shared.state).unwrap().presentation.status().active);
    }
    #[test]
    fn failed_presentation_preference_write_restores_the_preceding_configuration() {
        let folder = tempfile::tempdir().unwrap();
        let shared = Shared::default();
        let old = lock(&shared.state).unwrap().presentation.status().options;
        // No stored file: the host must not invent/rewrite one after a failed load.
        assert!(
            shared
                .presentation_change(
                    folder.path(),
                    PresentationOptions {
                        display: !old.display,
                        hide_desktop: false,
                        auto_external: true
                    },
                    false
                )
                .is_err()
        );
        let current = lock(&shared.state).unwrap().presentation.status();
        assert_eq!(current.options.display, old.display);
        assert_eq!(current.options.auto_external, old.auto_external);
        assert!(!current.active);
        assert!(!folder.path().join("native-system.json").exists());
    }
}

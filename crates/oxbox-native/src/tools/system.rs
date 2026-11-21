//! Native forms for system, power, window and application tools.
mod apps;
mod focus;
mod monitor;
mod power;
mod runtime;
mod windows;
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::{
    CoreResult,
    tools::{launch_apps::AppSet, presentation_mode::PresentationOptions},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct SystemRecovery {
    shared: Arc<runtime::Shared>,
}
impl SystemRecovery {
    pub fn invalidate_pending(&self) -> u64 {
        self.shared.invalidate_pending()
    }
    pub fn prepare(&self, action: crate::recovery::Action) -> u64 {
        self.shared.prepare_recovery(action)
    }
    pub fn request_shutdown(&self) -> u64 {
        self.shared.request_shutdown()
    }
    pub fn shutdown(&self) -> CoreResult<()> {
        self.request_shutdown();
        self.shared.mouse.shutdown();
        runtime::lock(&self.shared.state)?.cleanup()
    }
    pub fn execute(&self, action: crate::recovery::Action, _generation: u64) -> CoreResult<()> {
        use crate::recovery::Action as Recovery;
        let action = match action {
            Recovery::StopMouse => {
                self.invalidate_pending();
                return power::stop_mouse(&self.shared);
            }
            Recovery::CancelSleep => runtime::Action::SleepCancel,
            Recovery::StopAwake => runtime::Action::AwakeStop,
            Recovery::StopPresentation => runtime::Action::PresentationStop,
            Recovery::ToggleAwake => runtime::Action::AwakeToggle,
            Recovery::Lock => runtime::Action::Lock,
            Recovery::ToggleDesktop => runtime::Action::DesktopToggle,
            Recovery::Quit => return self.shutdown(),
            Recovery::QuitMedia => {
                return Err(runtime::error(
                    "Media Quit must be routed to the media service.",
                ));
            }
            Recovery::StopRecording => {
                return Err(runtime::error(
                    "Recording recovery must be routed to the media service.",
                ));
            }
        };
        if matches!(
            action,
            runtime::Action::SleepCancel
                | runtime::Action::AwakeStop
                | runtime::Action::PresentationStop
        ) {
            self.invalidate_pending();
        }
        // Accepted tray commands are serialized; pending UI jobs carry admission permits.
        self.shared.perform(action).map(|_| ())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    sets: Vec<AppSet>,
    presentation: PresentationOptions,
}
impl Default for Stored {
    fn default() -> Self {
        let mut presentation = PresentationOptions::default();
        if cfg!(target_os = "linux") {
            presentation.hide_desktop = false;
        }
        Self {
            sets: vec![],
            presentation,
        }
    }
}
#[derive(Deserialize)]
#[serde(default)]
struct LegacyTime {
    presentation: PresentationOptions,
}
impl Default for LegacyTime {
    fn default() -> Self {
        Self {
            presentation: Stored::default().presentation,
        }
    }
}
#[derive(Default)]
pub struct SystemTools {
    runtime: runtime::Runtime,
    power: power::PowerView,
    monitor: monitor::MonitorViews,
    windows: windows::WindowsViews,
    apps: apps::AppSetsView,
    stored: Stored,
    storage_ready: bool,
    loaded: bool,
    error: Option<String>,
    tray_error: Option<String>,
    banner_task: Task<String>,
    ctx: Option<egui::Context>,
}
impl SystemTools {
    pub fn recovery_handle(&self) -> SystemRecovery {
        SystemRecovery {
            shared: self.runtime.shared.clone(),
        }
    }
    pub fn initialize(&mut self, host: &mut Host, ctx: &egui::Context) {
        self.ctx = Some(ctx.clone());
        self.tick(host);
    }

    fn load(&mut self, host: &Host) -> CoreResult<()> {
        let stored = match host.load::<Stored>("native-system.json")? {
            Some(stored) => stored,
            None => {
                let mut stored = Stored::default();
                if let Some(sets) = host.load_key::<Vec<AppSet>>("app-sets.json", "sets")? {
                    stored.sets = sets;
                }
                if let Some(time) = host.load_key::<LegacyTime>("settings.json", "roadmapTime")? {
                    stored.presentation = time.presentation;
                }
                apps::validate(&stored.sets)?;
                host.save("native-system.json", &stored)?;
                stored
            }
        };
        apps::validate(&stored.sets)?;
        runtime::lock(&self.runtime.shared.state)?
            .presentation
            .configure(stored.presentation.clone())?;
        self.stored = stored;
        self.storage_ready = true;
        Ok(())
    }
    pub fn tick(&mut self, host: &mut Host) {
        if !self.loaded {
            self.loaded = true;
            if let Err(e) = self.load(host) {
                self.error = Some(format!(
                    "System preferences could not load: {e}. Existing files were preserved; repair them before saving."
                ));
            }
        }
        if let Some(ctx) = &self.ctx
            && let Err(e) = self.runtime.initialize(host, ctx)
        {
            self.runtime
                .shared
                .stopping
                .store(true, std::sync::atomic::Ordering::Release);
            self.error = Some(format!(
                "The background system scheduler could not start: {e}. Effect controls are unavailable; restart Oxbox."
            ));
        }
        self.power.poll();
        self.monitor.poll();
        self.windows.poll();
        self.apps.poll();
        self.banner_task.poll();
        if let Ok(state) = self.runtime.shared.state.try_lock() {
            self.stored.presentation = state.presentation.status().options;
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, host: &mut Host) -> bool {
        self.ctx = Some(ui.ctx().clone());
        self.tick(host);
        let shared = self.runtime.shared.clone();
        let handled = self.monitor.show(ui, id)
            || self
                .power
                .show(ui, id, &shared, &mut self.stored, self.storage_ready, host)
            || self.windows.show(ui, id, &shared)
            || if id == "launch-app-sets" {
                self.apps
                    .show(ui, &mut self.stored, self.storage_ready, host, &shared);
                true
            } else {
                false
            };
        if handled && let Some(error) = &self.error {
            ui.label(format!("Error: {error}"));
        }
        handled
    }
    fn banner_action(&mut self, ctx: &egui::Context, action: runtime::Action) {
        let shared = self.runtime.shared.clone();
        let permit = shared.reserve_action(&action);
        if self.banner_task.start(ctx, move |cancel, _| {
            cancel.check()?;
            shared.perform_reserved(action, permit)
        }) {
            self.tray_error = None;
        } else {
            self.tray_error = Some(
                "Another system operation is still running. Retry this action when it finishes."
                    .into(),
            );
        }
    }
    fn tray_action(&mut self, action: runtime::Action) {
        let Some(ctx) = self.ctx.clone() else {
            self.tray_error =
                Some("System tools are still initializing. Retry from the tray.".into());
            return;
        };
        self.banner_action(&ctx, action);
    }
    pub fn banners(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        self.ctx = Some(ui.ctx().clone());
        self.tick(host);
        let shared = self.runtime.shared.clone();
        let summary = shared
            .state
            .try_lock()
            .map(|s| {
                (
                    s.timer
                        .status()
                        .map(|t| (t.action, s.timer.remaining(s.now()))),
                    s.awake_status().unwrap_or_default(),
                    s.presentation.status().active,
                    s.pending.as_ref().map(|p| p.remaining()),
                    s.focus.as_ref().is_some_and(|f| f.status().active),
                    s.error.clone(),
                )
            })
            .ok();
        if let Some((sleep, awake, presentation, resolution, focus, error)) = summary {
            if let Some((action, remaining)) = sleep {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(format!("Sleep Timer: {action:?} in {remaining} s"));
                    if ui.button("Cancel Sleep Timer").clicked() {
                        self.banner_action(ui.ctx(), runtime::Action::SleepCancel);
                    }
                });
            }
            if awake.active {
                ui.horizontal(|ui| {
                    ui.label("Do Not Sleep is active");
                    if ui.button("Stop keeping awake").clicked() {
                        self.banner_action(ui.ctx(), runtime::Action::AwakeStop);
                    }
                });
            }
            if presentation {
                ui.horizontal(|ui| {
                    ui.label("Presentation mode is active");
                    if ui.button("Stop presentation").clicked() {
                        self.banner_action(ui.ctx(), runtime::Action::PresentationStop);
                    }
                });
            }
            if let Some(seconds) = resolution {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(format!("Display test: {seconds} seconds left"));
                    if ui.button("Keep mode").clicked() {
                        self.banner_action(ui.ctx(), runtime::Action::KeepMode);
                    }
                    if ui.button("Restore mode").clicked() {
                        self.banner_action(ui.ctx(), runtime::Action::RestoreMode);
                    }
                });
            }
            if focus {
                ui.horizontal(|ui| {
                    ui.label("Focus shades are active");
                    if ui.button("Stop focus dimming").clicked() {
                        self.banner_action(ui.ctx(), runtime::Action::FocusStop);
                    }
                });
            }
            if let Some(error) = error {
                ui.label(format!("System error: {error}"));
            }
        }
        if let Ok(mouse) = shared.mouse.status()
            && mouse.active
        {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Mouse Activity: {:?}", mouse.phase));
                if ui.button("Stop Mouse Activity").clicked()
                    && let Err(e) = power::stop_mouse(&shared)
                {
                    self.error = Some(e.to_string());
                }
            });
        }
        self.banner_task.show(ui);
        if let Some(error) = &self.tray_error {
            ui.label(format!("Tray action: {error}"));
        }
        if let Some(e) = &self.error {
            ui.label(e);
        }
    }
    pub fn stop_mouse(&mut self) {
        if let Err(e) = power::stop_mouse(&self.runtime.shared) {
            self.error = Some(e.to_string());
        }
    }
    pub fn toggle_awake(&mut self) {
        self.tray_action(runtime::Action::AwakeToggle);
    }
    pub fn lock_screen(&mut self) {
        self.tray_action(runtime::Action::Lock);
    }
    pub fn toggle_desktop(&mut self) {
        self.tray_action(runtime::Action::DesktopToggle);
    }
    pub fn cancel_sleep(&mut self) {
        if let Err(e) = self.runtime.shared.perform(runtime::Action::SleepCancel) {
            self.error = Some(e.to_string());
        }
    }
    pub fn stop_awake(&mut self) {
        if let Err(e) = self.runtime.shared.perform(runtime::Action::AwakeStop) {
            self.error = Some(e.to_string());
        }
    }
    pub fn stop_presentation(&mut self) {
        if let Err(e) = self
            .runtime
            .shared
            .perform(runtime::Action::PresentationStop)
        {
            self.error = Some(e.to_string());
        }
    }
    pub fn shutdown(&mut self) {
        self.runtime
            .shared
            .stopping
            .store(true, std::sync::atomic::Ordering::Release);
        self.runtime.shared.mouse.shutdown();
        self.power.shutdown();
        self.monitor.shutdown();
        self.windows.shutdown();
        self.apps.shutdown();
        self.banner_task.shutdown();
        self.runtime.shutdown();
    }
}

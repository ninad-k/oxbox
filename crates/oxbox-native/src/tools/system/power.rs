use super::{
    Stored,
    runtime::{self, Action, Shared},
};
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::{
    CoreResult,
    jobs::CancelToken,
    platform::Os,
    tools::{
        airplane_mode,
        alarm::{self, Ambiguity},
        do_not_disturb,
        mouse_activity::{self, Config},
        sleep_timer::PowerAction,
    },
};
use std::sync::Arc;

pub struct PowerView {
    task: Task<String>,
    pub error: Option<String>,
    awake_mode: u8,
    display: bool,
    minutes: u64,
    until: String,
    zone: String,
    sleep_action: PowerAction,
    sleep_seconds: u64,
    mouse: Config,
    capped: bool,
    cap_minutes: u64,
    pending: Option<(Action, String)>,
    presentation: Option<oxbox_core::tools::presentation_mode::PresentationOptions>,
}
impl Default for PowerView {
    fn default() -> Self {
        Self {
            task: Default::default(),
            error: None,
            awake_mode: 0,
            display: false,
            minutes: 25,
            until: String::new(),
            zone: "UTC".into(),
            sleep_action: PowerAction::Sleep,
            sleep_seconds: 1500,
            mouse: Config {
                idle_seconds: 60,
                interval_seconds: 30,
                duration_minutes: None,
            },
            capped: false,
            cap_minutes: 60,
            pending: None,
            presentation: None,
        }
    }
}
impl PowerView {
    pub fn poll(&mut self) {
        self.task.poll();
    }
    pub fn run(&mut self, ctx: &egui::Context, shared: Arc<Shared>, action: Action) {
        self.error = None;
        let permit = shared.reserve_action(&action);
        self.task.start(ctx, move |cancel, _| {
            cancel.check()?;
            shared.perform_reserved_cancellable(action, permit, &cancel)
        });
    }
    fn settings(&mut self, ctx: &egui::Context, shared: Arc<Shared>, airplane: bool) {
        let permit = shared.reserve();
        self.task.start(ctx, move |cancel, _| {
            open_settings_reserved(&shared, permit, &cancel, airplane, |airplane| {
                if airplane {
                    airplane_mode::open_settings()
                } else {
                    do_not_disturb::open_settings()
                }
            })
        });
    }
    fn result(&mut self, ui: &mut egui::Ui) {
        self.task.show(ui);
        if let Some(text) = &self.task.result {
            ui.label(text);
        }
        if let Some(text) = &self.error {
            crate::accessibility::alert(ui, text);
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        shared: &Arc<Shared>,
        stored: &mut Stored,
        storage_ready: bool,
        host: &mut Host,
    ) -> bool {
        if !matches!(
            id,
            "do-not-sleep"
                | "sleep-timer"
                | "mouse-activity"
                | "lock-screen"
                | "hide-desktop"
                | "presentation-mode"
                | "do-not-disturb"
                | "airplane-mode"
        ) {
            return false;
        }
        let snapshot = match shared.state.try_lock() {
            Ok(state) => (
                state.awake_status().unwrap_or_default(),
                state
                    .timer
                    .status()
                    .map(|t| (t.action, state.timer.remaining(state.now()))),
                state.desktop.is_hidden(),
                state.presentation.status(),
            ),
            Err(_) => {
                ui.spinner();
                ui.label("System state is updating. Stop and recovery controls remain available from the tray.");
                self.result(ui);
                return true;
            }
        };
        match id {
            "do-not-sleep" => {
                let status = snapshot.0;
                ui.label(if status.active {
                    format!(
                        "Keeping the system awake{}{}.",
                        if status.display {
                            " and the display on"
                        } else {
                            ""
                        },
                        status
                            .until
                            .map_or(String::new(), |t| format!(" until Unix time {t}"))
                    )
                } else {
                    "Do Not Sleep is off.".into()
                });
                ui.add_enabled_ui(!status.active && !self.task.running(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Keep awake");
                        let choice = egui::ComboBox::from_label("Keep awake")
                            .selected_text(
                                ["Until stopped", "For a duration", "Until a dated time"]
                                    [self.awake_mode as usize],
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.awake_mode, 0, "Until stopped");
                                ui.selectable_value(&mut self.awake_mode, 1, "For a duration");
                                ui.selectable_value(&mut self.awake_mode, 2, "Until a dated time");
                            });
                        crate::accessibility::combo_expansion(&choice.response);
                    });
                    if self.awake_mode == 1 {
                        number(ui, "Minutes", &mut self.minutes, 1..=44640);
                    }
                    if self.awake_mode == 2 {
                        let label = ui.label("End date/time (YYYY-MM-DDTHH:MM)");
                        ui.text_edit_singleline(&mut self.until)
                            .labelled_by(label.id);
                        let label = ui.label("IANA time zone");
                        ui.text_edit_singleline(&mut self.zone)
                            .labelled_by(label.id);
                        ui.small("For a repeated DST time, the earlier occurrence is used.");
                    }
                    ui.checkbox(&mut self.display, "Keep the display on too");
                });
                if status.active {
                    if ui.button("Stop keeping awake").clicked() {
                        self.run(ui.ctx(), shared.clone(), Action::AwakeStop);
                    }
                } else if ui
                    .add_enabled(
                        !self.task.running(),
                        egui::Button::new("Start keeping awake"),
                    )
                    .clicked()
                {
                    let until = if self.awake_mode == 2 {
                        alarm::resolve_local(&self.until, &self.zone, Ambiguity::Earlier)
                            .and_then(|v| {
                                u64::try_from(v)
                                    .map_err(|_| runtime::error("Choose a future end time."))
                            })
                            .map(Some)
                    } else {
                        Ok(None)
                    };
                    match until {
                        Ok(until) => self.run(
                            ui.ctx(),
                            shared.clone(),
                            Action::Awake {
                                display: self.display,
                                minutes: (self.awake_mode == 1).then_some(self.minutes),
                                until,
                            },
                        ),
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
                ui.label("Stop or Quit releases the request. Explicit sleep, lid and battery policies remain under operating-system control.");
            }
            "sleep-timer" => {
                ui.label("Schedule sleep, shutdown, restart or sign out. Save your work first. A native worker runs the deadline even when Oxbox is hidden.");
                let pending = snapshot.1;
                if let Some((action, seconds)) = pending {
                    ui.strong(format!("{action:?} in {seconds} seconds"));
                    if ui.button("Cancel Sleep Timer").clicked() {
                        self.run(ui.ctx(), shared.clone(), Action::SleepCancel);
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Action");
                    let choice = egui::ComboBox::from_label("Power action")
                        .selected_text(format!("{:?}", self.sleep_action))
                        .show_ui(ui, |ui| {
                            for (action, label) in [
                                (PowerAction::Sleep, "Sleep"),
                                (PowerAction::Shutdown, "Shutdown"),
                                (PowerAction::Restart, "Restart"),
                                (PowerAction::SignOut, "Sign out"),
                            ] {
                                ui.selectable_value(&mut self.sleep_action, action, label);
                            }
                        });
                    crate::accessibility::combo_expansion(&choice.response);
                });
                number(ui, "Delay in seconds", &mut self.sleep_seconds, 1..=2678400);
                if ui
                    .add_enabled(
                        !self.task.running(),
                        egui::Button::new("Review scheduled action…"),
                    )
                    .clicked()
                {
                    self.pending = Some((
                        Action::Sleep {
                            seconds: self.sleep_seconds,
                            action: self.sleep_action,
                        },
                        format!(
                            "In {} seconds the operating system will {:?}. Unsaved work may be lost. Quit cancels the pending action.",
                            self.sleep_seconds, self.sleep_action
                        ),
                    ));
                }
            }
            "mouse-activity" => {
                ui.label("Starts stopped. Explicit sessions move the pointer gently only after the idle delay and yield when input resumes. No clicks, typing or scrolling.");
                match shared.mouse.status() {
                    Ok(s) => {
                        ui.strong(format!(
                            "{:?} · idle {} s · {} movements",
                            s.phase, s.idle_seconds, s.movements
                        ));
                        if let Some(remaining) = s.remaining_seconds {
                            ui.label(format!("Duration remaining: {remaining} seconds"));
                        }
                        if let Some(e) = s.error {
                            crate::accessibility::alert(ui, e);
                        }
                    }
                    Err(e) => {
                        ui.label(e.to_string());
                    }
                }
                if Os::current() != Os::Windows {
                    ui.label("Mouse Activity is not available on this operating system.");
                } else {
                    number(
                        ui,
                        "Idle delay (seconds)",
                        &mut self.mouse.idle_seconds,
                        30..=3600,
                    );
                    number(
                        ui,
                        "Movement interval (seconds)",
                        &mut self.mouse.interval_seconds,
                        5..=300,
                    );
                    ui.checkbox(&mut self.capped, "End after a duration");
                    if self.capped {
                        number(ui, "Maximum minutes", &mut self.cap_minutes, 1..=1440);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.task.running(),
                                egui::Button::new("Start Mouse Activity"),
                            )
                            .clicked()
                        {
                            self.mouse.duration_minutes = self.capped.then_some(self.cap_minutes);
                            let config = self.mouse;
                            let permit = shared.reserve();
                            match shared.mouse.reserve(mouse_activity::Action::Start) {
                                Ok(request) => {
                                    let shared = shared.clone();
                                    self.task.start(ui.ctx(), move |cancel, _| {
                                        cancel.check()?;
                                        shared.with_reserved_cancellable(permit, &cancel, |_| {
                                            shared.mouse.start(request, config)?;
                                            Ok("Started a Mouse Activity session.".into())
                                        })
                                    });
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            }
                        }
                        if ui.button("Stop Mouse Activity").clicked() {
                            match stop_mouse(shared) {
                                Ok(()) => {}
                                Err(e) => self.error = Some(e.to_string()),
                            }
                        }
                    });
                }
                ui.label("Held input, protected desktops or native errors stop/prevent movement. Accessibility and remote-control input may conservatively restart waiting. This is not a guarantee about sleep, lock or application presence policy.");
            }
            "lock-screen" => {
                if Os::current() == Os::Macos {
                    ui.label("macOS sleeps the display. Configure password-on-wake immediately in System Settings for actual locking.");
                } else {
                    ui.label("Lock this workstation immediately. Save any work that requires your attention first.");
                }
                if ui.button("Lock now").clicked() {
                    self.run(ui.ctx(), shared.clone(), Action::Lock);
                }
            }
            "hide-desktop" => {
                if Os::current() == Os::Linux {
                    ui.label("Desktop icon hiding is unavailable on Linux.");
                } else {
                    let hidden = snapshot.2;
                    ui.label(if hidden {
                        "Desktop icon visibility is owned by this session."
                    } else {
                        "Desktop icons have not been changed by this tool."
                    });
                    if ui
                        .button(if hidden {
                            "Restore preceding visibility"
                        } else {
                            "Hide desktop icons"
                        })
                        .clicked()
                    {
                        self.run(ui.ctx(), shared.clone(), Action::Desktop(!hidden));
                    }
                    ui.label("Oxbox restores the exact visibility it found on Stop/Quit. macOS may restart Finder when applying this setting.");
                }
            }
            "presentation-mode" => {
                let mut options = self
                    .presentation
                    .take()
                    .unwrap_or_else(|| stored.presentation.clone());
                let active = snapshot.3.active;
                ui.strong(if active {
                    "Presentation mode is active"
                } else {
                    "Presentation mode is off"
                });
                ui.add_enabled_ui(!active && !self.task.running(), |ui| {
                    ui.checkbox(&mut options.display, "Keep the display on");
                    ui.checkbox(
                        &mut options.hide_desktop,
                        "Hide desktop icons while presenting (Windows/macOS)",
                    );
                    ui.checkbox(
                        &mut options.auto_external,
                        "Start automatically when an additional display is connected",
                    );
                });
                ui.label("Automatic display inspection begins only when enabled. A manual Stop suppresses restart until the additional display disconnects. Notification mode remains manual in OS settings.");
                if active {
                    if ui.button("Stop presentation").clicked() {
                        self.run(ui.ctx(), shared.clone(), Action::PresentationStop);
                    }
                } else {
                    ui.horizontal(|ui| {
                        for (start, label) in [
                            (true, "Start presentation"),
                            (false, "Save automatic preferences"),
                        ] {
                            if ui
                                .add_enabled(
                                    storage_ready && !self.task.running(),
                                    egui::Button::new(label),
                                )
                                .clicked()
                            {
                                let options = options.clone();
                                let shared = shared.clone();
                                let directory = host.data_dir().to_path_buf();
                                let permit = shared.reserve();
                                self.task.start(ui.ctx(), move |cancel, _| {
                                    cancel.check()?;
                                    shared.presentation_change_reserved(
                                        &directory, options, start, permit, &cancel,
                                    )
                                });
                            }
                        }
                    });
                }
                self.presentation = Some(options);
                if ui.button("Open notification controls").clicked() {
                    self.settings(ui.ctx(), shared.clone(), false);
                }
            }
            "do-not-disturb" | "airplane-mode" => {
                let capability = if id == "airplane-mode" {
                    airplane_mode::capability(Os::current())
                } else {
                    do_not_disturb::capability(Os::current())
                };
                ui.label(capability.message);
                ui.label("State is controlled by you and the operating system; Oxbox does not report a successful automatic toggle.");
                if ui
                    .add_enabled(
                        capability.available && !self.task.running(),
                        egui::Button::new(capability.action_label),
                    )
                    .clicked()
                {
                    self.settings(ui.ctx(), shared.clone(), id == "airplane-mode");
                }
            }
            _ => return false,
        }
        if let Some((action, description)) = self.pending.clone() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong("Confirm this operating-system action");
                ui.label(description);
                ui.horizontal(|ui| {
                    if ui.button("Confirm and arm").clicked() {
                        self.pending = None;
                        self.run(ui.ctx(), shared.clone(), action);
                    }
                    if ui.button("Cancel review").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
        self.result(ui);
        true
    }
    pub fn shutdown(&mut self) {
        self.task.shutdown();
        self.pending = None;
    }
    pub fn leave_tool(&mut self) {
        self.task.abandon();
        self.pending = None;
    }
}
fn open_settings_reserved(
    shared: &Shared,
    permit: u64,
    cancel: &CancelToken,
    airplane: bool,
    open: impl FnOnce(bool) -> CoreResult<()>,
) -> CoreResult<String> {
    // Admission belongs to the original click, even if this task starts later.
    // Check cancellation under the same gate immediately before the OS request.
    shared.with_reserved_cancellable(permit, cancel, |_| {
        open(airplane)?;
        Ok("System settings opened; choose the setting there.".into())
    })
}
pub fn stop_mouse(shared: &Shared) -> CoreResult<()> {
    shared.invalidate_pending();
    let _ = shared.mouse.request_stop();
    let request = shared.mouse.reserve(mouse_activity::Action::Stop)?;
    shared.mouse.stop(request)?;
    Ok(())
}
pub fn number(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u64,
    range: std::ops::RangeInclusive<u64>,
) {
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(1)
                .prefix(" "),
        )
        .labelled_by(label.id);
    });
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    use oxbox_core::jobs::CancelToken;

    #[test]
    fn settings_open_captured_before_navigation_or_quit_never_calls_backend() {
        for quit in [false, true] {
            let shared = Shared::default();
            let permit = shared.reserve();
            let cancel = CancelToken::default();
            if quit {
                shared.request_shutdown();
            } else {
                shared.invalidate_pending();
            }
            let mut opened = false;
            let result = open_settings_reserved(&shared, permit, &cancel, false, |_| {
                opened = true;
                Ok(())
            });
            assert_eq!(result.unwrap_err().code(), "cancelled");
            assert!(
                !opened,
                "a delayed settings request must not renew its admission"
            );
        }

        let shared = Shared::default();
        let cancel = CancelToken::default();
        let mut opened = None;
        open_settings_reserved(&shared, shared.reserve(), &cancel, true, |airplane| {
            opened = Some(airplane);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            opened,
            Some(true),
            "a new explicit request remains available"
        );

        cancel.cancel();
        assert_eq!(
            open_settings_reserved(&shared, shared.reserve(), &cancel, false, |_| {
                panic!("a cancelled task must not open settings");
            })
            .unwrap_err()
            .code(),
            "cancelled"
        );
    }
}

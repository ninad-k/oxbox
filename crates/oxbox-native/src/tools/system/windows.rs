use super::{
    monitor::bytes,
    runtime::{self, Action, Shared},
};
use crate::jobs::Task;
use eframe::egui;
use oxbox_core::{
    CoreResult,
    platform::Os,
    tools::{
        close_apps,
        eject_volumes::{self, Volume},
        energy_saver::{self, PowerProfile},
        hidden_files,
        resolution::{self, Display},
        uninstall::{self, InstalledApp, Leftover},
        window_manager::Snap,
        window_top::{self, WindowInfo},
    },
};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

enum Read {
    Windows(Vec<WindowInfo>),
    Displays(Vec<Display>),
    Volumes(Vec<Volume>),
    Profiles(Vec<PowerProfile>),
    Hidden(bool),
    Apps(Vec<InstalledApp>),
    Leftovers(String, Vec<Leftover>),
}
#[derive(Clone)]
enum Operation {
    Core(Action),
    Close(Vec<(u64, u32)>),
    Eject(String),
    Profile(String),
    Hidden(bool),
    Uninstall(String, String),
    Trash(Vec<PathBuf>),
}
impl Operation {
    fn execute(
        self,
        shared: &Shared,
        permit: u64,
        cancel: &oxbox_core::jobs::CancelToken,
    ) -> CoreResult<String> {
        match self{
 Self::Core(action)=>shared.perform_reserved_cancellable(action, permit, cancel),
 operation=>shared.with_reserved_cancellable(permit, cancel, |_|match operation{
  Self::Close(targets)=>Ok(format!("Sent normal close requests to {} windows. Complete any save prompts in those applications.",close_apps::close(&targets,true)?)),
  Self::Eject(root)=>{eject_volumes::eject(&root,true)?;Ok("Safe eject request accepted.".into())},
  Self::Profile(id)=>{energy_saver::set(&id,true)?;Ok("Power plan changed. Refresh to inspect the active plan.".into())},
  Self::Hidden(show)=>set_hidden_visibility_with(show, hidden_files::set),
  Self::Uninstall(id,command)=>Ok(format!("Registered uninstaller started (PID {}). Complete its own prompts, then refresh. User-data folders were kept.",uninstall::uninstall(&id,&command,true)?)),
  Self::Trash(paths)=>Ok(format!("Moved {} reviewed folders to Trash.",oxbox_core::fs_safe::move_to_trash(&paths)?)),
  Self::Core(_)=>Err(runtime::error("Invalid nested native action.")),
 })
}
    }
}
fn set_hidden_visibility_with(
    requested: bool,
    set: impl FnOnce(bool) -> CoreResult<bool>,
) -> CoreResult<String> {
    let actual = set(requested)?;
    if actual != requested {
        return Err(runtime::error(format!(
            "The operating system did not apply the hidden-file visibility change. Hidden files are currently {}. Refresh and review your file-manager or organization settings.",
            if actual { "shown" } else { "hidden" }
        )));
    }
    Ok(format!("Hidden files visibility set to {actual}."))
}
pub struct WindowsViews {
    read: Task<Read>,
    task: Task<String>,
    windows: Vec<WindowInfo>,
    selected_window: Option<(u64, u32)>,
    checked: BTreeSet<u64>,
    displays: Vec<Display>,
    display_index: usize,
    mode_index: usize,
    volumes: Vec<Volume>,
    profiles: Vec<PowerProfile>,
    hidden: Option<bool>,
    apps: Vec<InstalledApp>,
    app_id: String,
    query: String,
    leftovers: Vec<Leftover>,
    leftover_checked: BTreeSet<PathBuf>,
    pending: Option<(Operation, String)>,
    opacity: f64,
    error: Option<String>,
    on_success_refresh: Option<&'static str>,
    needs_refresh: Option<&'static str>,
    on_success_trash: Vec<PathBuf>,
}
impl Default for WindowsViews {
    fn default() -> Self {
        Self {
            read: Default::default(),
            task: Default::default(),
            windows: vec![],
            selected_window: None,
            checked: BTreeSet::new(),
            displays: vec![],
            display_index: 0,
            mode_index: 0,
            volumes: vec![],
            profiles: vec![],
            hidden: None,
            apps: vec![],
            app_id: String::new(),
            query: String::new(),
            leftovers: vec![],
            leftover_checked: BTreeSet::new(),
            pending: None,
            opacity: 0.5,
            error: None,
            on_success_refresh: None,
            needs_refresh: None,
            on_success_trash: vec![],
        }
    }
}
impl WindowsViews {
    pub fn poll(&mut self) {
        self.read.poll();
        self.task.poll();
        if !self.task.running() {
            if self.task.result.is_some() {
                if let Some(refresh) = self.on_success_refresh.take() {
                    self.needs_refresh = Some(refresh);
                }
                if !self.on_success_trash.is_empty() {
                    self.leftovers
                        .retain(|item| !self.on_success_trash.contains(&item.path));
                    self.leftover_checked
                        .retain(|path| !self.on_success_trash.contains(path));
                    self.on_success_trash.clear();
                }
            } else if self.task.error.is_some() {
                self.on_success_refresh = None;
                self.on_success_trash.clear();
            }
        }
        if let Some(result) = self.read.result.take() {
            match result {
                Read::Windows(w) => {
                    self.windows = w;
                    self.checked
                        .retain(|id| self.windows.iter().any(|w| w.id == *id));
                    if self.selected_window.is_some_and(|(id, pid)| {
                        !self.windows.iter().any(|w| w.id == id && w.pid == pid)
                    }) {
                        self.selected_window = None;
                    }
                }
                Read::Displays(d) => {
                    self.displays = d;
                    self.display_index = 0;
                    self.mode_index = 0;
                }
                Read::Volumes(v) => self.volumes = v,
                Read::Profiles(p) => self.profiles = p,
                Read::Hidden(show) => self.hidden = Some(show),
                Read::Apps(a) => self.apps = a,
                Read::Leftovers(owner, items) => {
                    if owner == self.app_id {
                        self.leftovers = items;
                        self.leftover_checked.clear();
                    }
                }
            }
        }
    }
    fn action(&mut self, ui: &egui::Ui, shared: Arc<Shared>, operation: Operation) {
        let refresh = match &operation {
            Operation::Hidden(_) => Some("hidden-files"),
            Operation::Profile(_) => Some("energy-saver"),
            Operation::Eject(_) => Some("eject-volumes"),
            Operation::Core(Action::Top { .. } | Action::Snap { .. }) => Some("windows"),
            _ => None,
        };
        let trash = match &operation {
            Operation::Trash(paths) => paths.clone(),
            _ => vec![],
        };
        let permit = match &operation {
            Operation::Core(action) => shared.reserve_action(action),
            _ => shared.reserve(),
        };
        if self.task.start(ui.ctx(), move |cancel, _| {
            cancel.check()?;
            operation.execute(&shared, permit, &cancel)
        }) {
            self.on_success_refresh = refresh;
            self.on_success_trash = trash;
        }
    }
    fn windows_picker(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(
                !self.read.running(),
                egui::Button::new("Refresh application windows"),
            )
            .clicked()
        {
            self.read.start(ui.ctx(), |cancel, _| {
                cancel.check()?;
                Ok(Read::Windows(window_top::windows()?))
            });
        }
        let text = self
            .selected_window
            .and_then(|(id, pid)| self.windows.iter().find(|w| w.id == id && w.pid == pid))
            .map_or("Choose a window", |w| w.title.as_str());
        egui::ComboBox::from_label("Application window")
            .selected_text(text)
            .width(ui.available_width().min(500.0))
            .show_ui(ui, |ui| {
                for window in &self.windows {
                    ui.selectable_value(
                        &mut self.selected_window,
                        Some((window.id, window.pid)),
                        format!(
                            "{} · PID {}{}",
                            window.title,
                            window.pid,
                            if window.topmost { " · topmost" } else { "" }
                        ),
                    );
                }
            });
        if self.windows.is_empty() {
            ui.label("Refresh to enumerate available windows. Protected or closed windows may be unavailable.");
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, shared: &Arc<Shared>) -> bool {
        if !self.read.running()
            && let Some(refresh) = self.needs_refresh
        {
            let relevant = refresh == id
                || (refresh == "windows"
                    && matches!(
                        id,
                        "window-on-top" | "window-manager" | "close-apps" | "focus-window"
                    ));
            if relevant {
                self.needs_refresh = None;
                self.read.start(ui.ctx(), move |cancel, _| {
                    cancel.check()?;
                    match refresh {
                        "hidden-files" => Ok(Read::Hidden(hidden_files::status()?)),
                        "energy-saver" => Ok(Read::Profiles(energy_saver::profiles()?)),
                        "eject-volumes" => Ok(Read::Volumes(eject_volumes::volumes()?)),
                        _ => Ok(Read::Windows(window_top::windows()?)),
                    }
                });
            }
        }
        if !matches!(
            id,
            "window-on-top"
                | "window-manager"
                | "show-desktop"
                | "focus-window"
                | "close-apps"
                | "switch-resolution"
                | "eject-volumes"
                | "energy-saver"
                | "hidden-files"
                | "uninstall-apps"
        ) {
            return false;
        }
        if Os::current() != Os::Windows && !(id == "hidden-files" && Os::current() == Os::Macos) {
            ui.label("This tool is not available on this operating system.");
            return true;
        }
        match id {
            "window-on-top" => {
                self.windows_picker(ui);
                if let Some((id, pid)) = self.selected_window {
                    ui.horizontal(|ui| {
                        for (enabled, label) in [
                            (true, "Keep above other windows"),
                            (false, "Remove topmost status"),
                        ] {
                            if ui.button(label).clicked() {
                                self.action(
                                    ui,
                                    shared.clone(),
                                    Operation::Core(Action::Top { id, pid, enabled }),
                                );
                            }
                        }
                    });
                }
                ui.label("Changes owned by this session are restored on Quit if the window still has the state Oxbox applied.");
            }
            "window-manager" => {
                self.windows_picker(ui);
                if let Some((id, pid)) = self.selected_window {
                    ui.horizontal_wrapped(|ui| {
                        for (placement, label) in [
                            (Snap::LeftHalf, "Left half"),
                            (Snap::RightHalf, "Right half"),
                            (Snap::TopHalf, "Top half"),
                            (Snap::BottomHalf, "Bottom half"),
                            (Snap::LeftThird, "Left third"),
                            (Snap::CenterThird, "Center third"),
                            (Snap::RightThird, "Right third"),
                            (Snap::TopLeft, "Top left"),
                            (Snap::TopRight, "Top right"),
                            (Snap::BottomLeft, "Bottom left"),
                            (Snap::BottomRight, "Bottom right"),
                            (Snap::Full, "Full work area"),
                        ] {
                            if ui.button(label).clicked() {
                                self.action(
                                    ui,
                                    shared.clone(),
                                    Operation::Core(Action::Snap { id, pid, placement }),
                                );
                            }
                        }
                    });
                }
                ui.label("Windows may constrain the requested layout to their minimum dimensions.");
            }
            "show-desktop" => {
                ui.label("Minimize normal application windows to show the desktop. Restore only the windows tracked by this action.");
                ui.horizontal(|ui| {
                    if ui.button("Show desktop").clicked() {
                        self.action(ui, shared.clone(), Operation::Core(Action::Minimize));
                    }
                    if ui.button("Restore tracked windows").clicked() {
                        self.action(ui, shared.clone(), Operation::Core(Action::RestoreWindows));
                    }
                });
            }
            "focus-window" => {
                self.windows_picker(ui);
                ui.add(egui::Slider::new(&mut self.opacity, 0.1..=0.85).text("Dimming opacity"));
                if let Some((id, pid)) = self.selected_window
                    && ui.button("Start dimming around window").clicked()
                {
                    self.action(
                        ui,
                        shared.clone(),
                        Operation::Core(Action::Focus {
                            id,
                            pid,
                            opacity: self.opacity,
                        }),
                    );
                }
                if ui.button("Stop dimming").clicked() {
                    self.action(ui, shared.clone(), Operation::Core(Action::FocusStop));
                }
                if let Ok(s) = shared.state.try_lock()
                    && let Some(f) = &s.focus
                {
                    let status = f.status();
                    ui.label(if status.active {
                        "Focus dimming is active."
                    } else {
                        "Focus dimming is stopped."
                    });
                    if let Some(e) = status.error {
                        crate::accessibility::alert(ui, e);
                    }
                }
                ui.label("Native click-through shades track the selected window while Oxbox is hidden. Stop/Quit or closing/minimizing the target removes them. Nothing is recorded.");
            }
            "close-apps" => {
                if ui.button("Refresh windows").clicked() {
                    self.read.start(ui.ctx(), |cancel, _| {
                        cancel.check()?;
                        Ok(Read::Windows(window_top::windows()?))
                    });
                }
                for w in &self.windows {
                    let mut selected = self.checked.contains(&w.id);
                    if ui
                        .checkbox(&mut selected, format!("{} · PID {}", w.title, w.pid))
                        .changed()
                    {
                        if selected {
                            self.checked.insert(w.id);
                        } else {
                            self.checked.remove(&w.id);
                        }
                    }
                }
                if ui
                    .add_enabled(
                        !self.checked.is_empty(),
                        egui::Button::new("Review close requests…"),
                    )
                    .clicked()
                {
                    let targets = self
                        .windows
                        .iter()
                        .filter(|w| self.checked.contains(&w.id))
                        .map(|w| (w.id, w.pid))
                        .collect();
                    let names = self
                        .windows
                        .iter()
                        .filter(|w| self.checked.contains(&w.id))
                        .map(|w| w.title.clone())
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.pending = Some((
                        Operation::Close(targets),
                        format!(
                            "Request normal close for these windows?\n{names}\nApplications may ask about unsaved work. Oxbox does not force-kill."
                        ),
                    ));
                }
            }
            "switch-resolution" => {
                ui.label("Test a display mode with a native 15-second rollback deadline. Keep is accepted only before the deadline; the guard works while the launcher is hidden.");
                if ui.button("Refresh display modes").clicked() {
                    self.read.start(ui.ctx(), |cancel, _| {
                        cancel.check()?;
                        Ok(Read::Displays(resolution::displays()?))
                    });
                }
                egui::ComboBox::from_label("Display")
                    .selected_text(
                        self.displays
                            .get(self.display_index)
                            .map_or("Choose a display", |d| d.name.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        for (i, d) in self.displays.iter().enumerate() {
                            if ui
                                .selectable_value(&mut self.display_index, i, &d.name)
                                .changed()
                            {
                                self.mode_index = 0;
                            }
                        }
                    });
                if let Some(display) = self.displays.get(self.display_index) {
                    ui.label(format!(
                        "Current: {} × {} @ {} Hz",
                        display.current.width, display.current.height, display.current.refresh
                    ));
                    egui::ComboBox::from_label("Display mode")
                        .selected_text(
                            display
                                .modes
                                .get(self.mode_index)
                                .map_or("Choose a mode".into(), |m| {
                                    format!("{} × {} @ {} Hz", m.width, m.height, m.refresh)
                                }),
                        )
                        .show_ui(ui, |ui| {
                            for (i, m) in display.modes.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.mode_index,
                                    i,
                                    format!("{} × {} @ {} Hz", m.width, m.height, m.refresh),
                                );
                            }
                        });
                    if let Some(mode) = display.modes.get(self.mode_index)
                        && ui.button("Review display test…").clicked()
                    {
                        self.pending = Some((
                            Operation::Core(Action::TestMode(mode.clone())),
                            format!(
                                "Apply {} × {} @ {} Hz to {}? Confirm Keep within 15 seconds after the change or the previous mode is restored.",
                                mode.width, mode.height, mode.refresh, display.name
                            ),
                        ));
                    }
                }
                if let Ok(state) = shared.state.try_lock() {
                    if let Some(p) = &state.pending {
                        ui.strong(format!("Display test pending: {} seconds", p.remaining()));
                        ui.horizontal(|ui| {
                            if ui.button("Keep display mode").clicked() {
                                self.action(ui, shared.clone(), Operation::Core(Action::KeepMode));
                            }
                            if ui.button("Restore previous mode").clicked() {
                                self.action(
                                    ui,
                                    shared.clone(),
                                    Operation::Core(Action::RestoreMode),
                                );
                            }
                        });
                    }
                    if let Some(outcome) = &state.resolution_outcome {
                        ui.label(outcome);
                    }
                    if let Some(e) = &state.error {
                        crate::accessibility::alert(ui, e);
                    }
                }
            }
            "eject-volumes" => {
                ui.label("Review removable drives before requesting safe eject. Busy or protected devices can refuse the request.");
                if ui.button("Refresh removable volumes").clicked() {
                    self.read.start(ui.ctx(), |cancel, _| {
                        cancel.check()?;
                        Ok(Read::Volumes(eject_volumes::volumes()?))
                    });
                }
                for volume in self.volumes.clone() {
                    ui.horizontal(|ui|{ui.label(format!("{} {}",volume.root,volume.label));if ui.button("Review eject…").clicked(){self.pending=Some((Operation::Eject(volume.root.clone()),format!("Request safe eject for {} {}? Finish copying and close files on this volume first.",volume.root,volume.label)));}});
                }
                if self.volumes.is_empty() {
                    ui.label("No removable volumes listed. Refresh to inspect available drives.");
                }
            }
            "energy-saver" => {
                if ui.button("Refresh power plans").clicked() {
                    self.read.start(ui.ctx(), |cancel, _| {
                        cancel.check()?;
                        Ok(Read::Profiles(energy_saver::profiles()?))
                    });
                }
                for plan in self.profiles.clone() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{}{}",
                            plan.name,
                            if plan.active { " · active" } else { "" }
                        ));
                        if ui
                            .add_enabled(!plan.active, egui::Button::new("Review plan change…"))
                            .clicked()
                        {
                            self.pending = Some((
                                Operation::Profile(plan.id.clone()),
                                format!(
                                    "Change the operating-system power plan to {} ({})?",
                                    plan.name, plan.id
                                ),
                            ));
                        }
                    });
                }
                ui.label("Available profiles depend on the device. This tool does not claim a particular battery saving.");
            }
            "hidden-files" => {
                ui.label("Change file-manager hidden-file visibility. Protected operating-system files are not enabled. macOS may restart Finder.");
                if ui.button("Read current visibility").clicked() {
                    self.read.start(ui.ctx(), |cancel, _| {
                        cancel.check()?;
                        Ok(Read::Hidden(hidden_files::status()?))
                    });
                }
                if let Some(show) = self.hidden {
                    ui.label(if show {
                        "Hidden files are currently shown."
                    } else {
                        "Hidden files are currently hidden."
                    });
                    if ui
                        .button(if show {
                            "Review hiding hidden files…"
                        } else {
                            "Review showing hidden files…"
                        })
                        .clicked()
                    {
                        self.pending = Some((
                            Operation::Hidden(!show),
                            format!(
                                "{} hidden files in the operating-system file manager?",
                                if show { "Hide" } else { "Show" }
                            ),
                        ));
                    }
                }
            }
            "uninstall-apps" => self.uninstall(ui),
            _ => {}
        }
        if let Some((operation, description)) = self.pending.clone() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong("Review and confirm");
                ui.label(description);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.task.running(), egui::Button::new("Confirm action"))
                        .clicked()
                    {
                        self.pending = None;
                        self.action(ui, shared.clone(), operation);
                    }
                    if ui.button("Cancel review").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
        self.read.show(ui);
        self.task.show(ui);
        if let Some(message) = &self.task.result {
            ui.label(message);
        }
        if let Some(error) = &self.error {
            ui.label(error);
        }
        true
    }
    fn uninstall(&mut self, ui: &mut egui::Ui) {
        ui.label("Review registered Windows desktop uninstallers. Vendor prompts and elevation requests remain under your control; Oxbox does not bypass them or remove user data automatically.");
        if ui.button("Refresh installed applications").clicked() {
            self.read.start(ui.ctx(), |cancel, _| {
                cancel.check()?;
                Ok(Read::Apps(uninstall::apps()?))
            });
        }
        let label = ui.label("Search name or publisher");
        ui.text_edit_singleline(&mut self.query)
            .labelled_by(label.id);
        let previous = self.app_id.clone();
        egui::ComboBox::from_label("Registered application")
            .selected_text(
                self.apps
                    .iter()
                    .find(|a| a.id == self.app_id)
                    .map_or("Choose an application", |a| a.name.as_str()),
            )
            .width(ui.available_width().min(500.0))
            .show_ui(ui, |ui| {
                for app in &self.apps {
                    if format!("{} {}", app.name, app.publisher)
                        .to_lowercase()
                        .contains(&self.query.to_lowercase())
                    {
                        ui.selectable_value(
                            &mut self.app_id,
                            app.id.clone(),
                            format!("{} · {}", app.name, app.publisher),
                        );
                    }
                }
            });
        if previous != self.app_id {
            self.leftovers.clear();
            self.leftover_checked.clear();
            self.pending = None;
        }
        if let Some(app) = self.apps.iter().find(|a| a.id == self.app_id).cloned() {
            ui.strong(&app.name);
            ui.label(format!("{} · {}", app.publisher, app.version));
            ui.label("Exact registered command");
            ui.monospace(&app.command);
            if !app.install_location.is_empty() {
                ui.label(format!("Install location: {}", app.install_location));
            }
            ui.horizontal(|ui| {
                if ui.button("Review and start uninstaller…").clicked() {
                    self.pending = Some((Operation::Uninstall(app.id.clone(), app.command.clone()), format!("Start the registered uninstaller for {}?\nPublisher: {}\nExact command:\n{}\nIts own prompts control removal. User-data folders are kept.", app.name, app.publisher, app.command)));
                }
                if ui.button("Scan exact-name user-data folders").clicked() {
                    let owner = app.id.clone();
                    let name = app.name.clone();
                    let publisher = app.publisher.clone();
                    self.leftovers.clear();
                    self.leftover_checked.clear();
                    self.read.start(ui.ctx(), move |cancel, progress| {
                        Ok(Read::Leftovers(owner, uninstall::scan_leftovers(&name, &publisher, &cancel, progress)?))
                    });
                }
            });
        }
        for item in &self.leftovers {
            let mut checked = self.leftover_checked.contains(&item.path);
            if ui
                .checkbox(
                    &mut checked,
                    format!(
                        "{} · {} files · {}",
                        item.path.display(),
                        item.files,
                        bytes(item.bytes)
                    ),
                )
                .changed()
            {
                if checked {
                    self.leftover_checked.insert(item.path.clone());
                } else {
                    self.leftover_checked.remove(&item.path);
                }
            }
        }
        if !self.leftovers.is_empty() {
            ui.label("Exact-name candidates are not proof that folders are unused. No candidate is selected automatically.");
            if ui
                .add_enabled(
                    !self.leftover_checked.is_empty(),
                    egui::Button::new("Review and trash selected folders…"),
                )
                .clicked()
            {
                let paths = self.leftover_checked.iter().cloned().collect::<Vec<_>>();
                self.pending = Some((
                    Operation::Trash(paths.clone()),
                    format!(
                        "Move these user-data folders to the Recycle Bin / Trash? They may contain personal data.\n{}",
                        paths
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                ));
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.read.shutdown();
        self.task.shutdown();
    }
    pub fn leave_tool(&mut self, id: &str) {
        if matches!(
            id,
            "window-on-top"
                | "window-manager"
                | "show-desktop"
                | "focus-window"
                | "close-apps"
                | "switch-resolution"
                | "eject-volumes"
                | "energy-saver"
                | "hidden-files"
                | "uninstall-apps"
        ) {
            self.read.abandon();
            self.task.abandon();
            self.pending = None;
            self.leftovers.clear();
            self.leftover_checked.clear();
            self.on_success_refresh = None;
            self.needs_refresh = None;
            self.on_success_trash.clear();
        }
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;

    #[test]
    fn hidden_file_change_reports_refused_readback_instead_of_success() {
        for requested in [false, true] {
            assert!(
                set_hidden_visibility_with(requested, |value| {
                    assert_eq!(value, requested);
                    Ok(!requested)
                })
                .unwrap_err()
                .to_string()
                .contains("did not apply")
            );
            assert!(set_hidden_visibility_with(requested, Ok).is_ok());
        }
        assert!(
            set_hidden_visibility_with(true, |_| {
                Err(runtime::error("generated backend refusal"))
            })
            .unwrap_err()
            .to_string()
            .contains("generated backend refusal")
        );
    }

    #[test]
    fn leaving_system_tools_cancels_ephemeral_scan_and_clears_review() {
        let ctx = egui::Context::default();
        let mut view = WindowsViews::default();
        let (ready, started) = std::sync::mpsc::channel();
        let (cancelled, observed) = std::sync::mpsc::channel();
        assert!(view.read.start(&ctx, move |cancel, _| {
            ready.send(()).unwrap();
            for _ in 0..400 {
                if cancel.check().is_err() {
                    cancelled.send(()).unwrap();
                    return Err(oxbox_core::CoreError::Cancelled);
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(runtime::error("fake scan safety timeout"))
        }));
        started
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap();
        view.pending = Some((Operation::Hidden(true), "Generated review only".into()));
        view.leftover_checked
            .insert(PathBuf::from("generated-fixture"));
        view.leave_tool("uninstall-apps");
        observed
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("navigation must cancel the fake scan without UI polling");
        assert!(view.pending.is_none());
        assert!(view.leftover_checked.is_empty());
        view.read.poll();
        assert!(view.read.result.is_none());
        assert!(view.read.error.is_none());
        view.shutdown();
    }
}

use super::{
    Stored,
    runtime::{self, Shared},
};
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::{
    CoreResult,
    tools::launch_apps::{self, AppSet, LaunchEntry, LaunchKind},
};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
pub struct AppSetsView {
    selected: String,
    draft: AppSet,
    task: Task<String>,
    pending_launch: Option<AppSet>,
    pending_delete: Option<String>,
    error: Option<String>,
    message: Option<String>,
}
impl Default for AppSetsView {
    fn default() -> Self {
        Self {
            selected: String::new(),
            draft: empty_set(),
            task: Default::default(),
            pending_launch: None,
            pending_delete: None,
            error: None,
            message: None,
        }
    }
}
fn empty_set() -> AppSet {
    AppSet {
        id: format!(
            "set-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_nanos())
        ),
        name: String::new(),
        entries: vec![],
    }
}
pub fn validate(sets: &[AppSet]) -> CoreResult<()> {
    if sets.len() > 64 {
        return Err(runtime::error("Save at most 64 application sets."));
    }
    let mut ids = HashSet::new();
    for set in sets {
        launch_apps::validate_set(set)?;
        if !ids.insert(&set.id) {
            return Err(runtime::error(
                "Application-set identifiers must be unique.",
            ));
        }
    }
    Ok(())
}
impl AppSetsView {
    pub fn poll(&mut self) {
        self.task.poll();
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        stored: &mut Stored,
        ready: bool,
        host: &mut Host,
        shared: &Arc<Shared>,
    ) {
        ui.label("Save named groups of local applications and files. Application arguments are literal values, never a shell command. URLs, network paths and shell scripts are refused.");
        let previous = self.selected.clone();
        egui::ComboBox::from_label("Saved application set")
            .selected_text(
                stored
                    .sets
                    .iter()
                    .find(|s| s.id == self.selected)
                    .map_or("Choose a saved set", |s| s.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for set in &stored.sets {
                    ui.selectable_value(&mut self.selected, set.id.clone(), &set.name);
                }
            });
        if previous != self.selected
            && let Some(set) = stored.sets.iter().find(|s| s.id == self.selected)
        {
            self.draft = set.clone();
            self.pending_launch = None;
            self.pending_delete = None;
        }
        if ui.button("New application set").clicked() {
            self.draft = empty_set();
            self.selected = self.draft.id.clone();
            self.message = None;
        }
        let label = ui.label("Set name");
        ui.text_edit_singleline(&mut self.draft.name)
            .labelled_by(label.id);
        ui.horizontal(|ui| {
            for (kind, label) in [
                (LaunchKind::Application, "Choose applications…"),
                (LaunchKind::File, "Choose local files…"),
            ] {
                if ui
                    .add_enabled(self.draft.entries.len() < 32, egui::Button::new(label))
                    .clicked()
                    && let Some(paths) = crate::host::dialogs::file(ui.ctx())
                        .set_title(label)
                        .pick_files()
                {
                    for path in paths.into_iter().take(32 - self.draft.entries.len()) {
                        self.draft.entries.push(LaunchEntry {
                            kind,
                            target: path.to_string_lossy().into_owned(),
                            args: vec![],
                        });
                    }
                }
            }
        });
        let mut remove = None;
        for (index, entry) in self.draft.entries.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(if entry.kind == LaunchKind::Application {
                            "Application"
                        } else {
                            "File with default app"
                        });
                        if crate::accessibility::button(
                            ui,
                            "Remove target",
                            &format!("Remove target {}: {}", index + 1, entry.target),
                        )
                        .clicked()
                        {
                            remove = Some(index);
                        }
                    });
                    let label = ui.label("Absolute local path");
                    ui.text_edit_singleline(&mut entry.target)
                        .labelled_by(label.id);
                    if entry.kind == LaunchKind::Application {
                        ui.label("Literal arguments (one input per argument)");
                        let mut remove_arg = None;
                        for (i, arg) in entry.args.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                let response = ui
                                    .add(egui::TextEdit::singleline(arg).id_salt(("argument", i)));
                                crate::accessibility::name(
                                    &response,
                                    &format!("Literal argument {}", i + 1),
                                );
                                if crate::accessibility::button(
                                    ui,
                                    &format!("Remove argument {}", i + 1),
                                    &format!(
                                        "Remove argument {} for target {}: {}",
                                        i + 1,
                                        index + 1,
                                        entry.target
                                    ),
                                )
                                .clicked()
                                {
                                    remove_arg = Some(i);
                                }
                            });
                        }
                        if let Some(i) = remove_arg {
                            entry.args.remove(i);
                        }
                        if ui
                            .add_enabled(
                                entry.args.len() < 64,
                                egui::Button::new("Add literal argument"),
                            )
                            .clicked()
                        {
                            entry.args.push(String::new());
                        }
                    }
                });
            });
        }
        if let Some(index) = remove {
            self.draft.entries.remove(index);
        }
        if self.draft.entries.is_empty() {
            ui.label("Add one or more local targets, then save the set.");
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    ready && !self.draft.name.trim().is_empty() && !self.draft.entries.is_empty(),
                    egui::Button::new("Save set"),
                )
                .clicked()
            {
                let mut candidate = stored.clone();
                candidate.sets.retain(|s| s.id != self.draft.id);
                candidate.sets.push(self.draft.clone());
                candidate.sets.sort_by_key(|a| a.name.to_lowercase());
                match validate(&candidate.sets).and_then(|()| persist(&mut candidate, host, shared))
                {
                    Ok(()) => {
                        *stored = candidate;
                        self.selected = self.draft.id.clone();
                        self.error = None;
                        self.message = Some("Application set saved.".into());
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            if let Some(saved) = stored.sets.iter().find(|s| s.id == self.selected) {
                if ui
                    .add_enabled(
                        !self.task.running(),
                        egui::Button::new("Review saved launch…"),
                    )
                    .clicked()
                {
                    self.pending_launch = Some(saved.clone());
                }
                if ui.button("Delete saved set…").clicked() {
                    self.pending_delete = Some(saved.id.clone());
                }
            }
        });
        if let Some(set) = self.pending_launch.clone() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(format!("Launch saved set “{}”?", set.name));
                for entry in &set.entries {
                    ui.label(&entry.target);
                    for arg in &entry.args {
                        ui.monospace(format!("Argument: {arg}"));
                    }
                }
                ui.label("Launched applications may perform their own actions, including network activity. Unsaved edits are not used.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.task.running(), egui::Button::new("Confirm launch")).clicked() {
                        self.pending_launch = None;
                        let shared = shared.clone();
                        self.task.start(ui.ctx(), move |cancel, _| {
                            cancel.check()?;
                            shared.with_live(|_| {
                                let count = launch_apps::launch(&set, &mut |path| open::that(path).map_err(|e| runtime::error(e.to_string())))?;
                                Ok(format!("Requested launch of {count} saved applications/files."))
                            })
                        });
                    }
                    if ui.button("Cancel launch review").clicked() {
                        self.pending_launch = None;
                    }
                });
            });
        }
        if let Some(id) = self.pending_delete.clone() {
            ui.label(
                "Delete this saved configuration? The applications and files themselves are kept.",
            );
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(ready, egui::Button::new("Confirm configuration deletion"))
                    .clicked()
                {
                    let mut candidate = stored.clone();
                    candidate.sets.retain(|s| s.id != id);
                    match persist(&mut candidate, host, shared) {
                        Ok(()) => {
                            *stored = candidate;
                            self.draft = empty_set();
                            self.selected.clear();
                            self.pending_delete = None;
                            self.message =
                                Some("Saved configuration deleted. Targets were kept.".into());
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
                if ui.button("Keep saved configuration").clicked() {
                    self.pending_delete = None;
                }
            });
        }
        self.task.show(ui);
        if let Some(message) = &self.task.result {
            ui.label(message);
        }
        if let Some(message) = &self.message {
            ui.label(message);
        }
        if let Some(error) = &self.error {
            crate::accessibility::alert(ui, error);
        }
    }
    pub fn shutdown(&mut self) {
        self.task.shutdown();
    }
}

fn persist(candidate: &mut Stored, host: &Host, shared: &Shared) -> CoreResult<()> {
    shared.with_live(|state| {
        candidate.presentation = state.presentation.status().options;
        host.save("native-system.json", candidate)
    })
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;

    #[test]
    fn two_application_targets_have_distinct_named_remove_actions() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let shared = Arc::new(Shared::default());
        let mut stored = Stored::default();
        let mut view = AppSetsView::default();
        view.draft.entries = ["/fixture/alpha.exe", "/fixture/beta.exe"]
            .into_iter()
            .map(|target| LaunchEntry {
                kind: LaunchKind::Application,
                target: target.into(),
                args: vec!["--fixture".into()],
            })
            .collect();
        let nodes = crate::accessibility::test_tree(|ui| {
            crate::jobs::register_context(ui.ctx()).close();
            view.show(ui, &mut stored, false, &mut host, &shared);
        });
        let names = nodes
            .iter()
            .filter(|node| node.role() == egui::accesskit::Role::Button)
            .filter_map(|node| node.label())
            .collect::<Vec<_>>();
        for (index, target) in ["/fixture/alpha.exe", "/fixture/beta.exe"]
            .into_iter()
            .enumerate()
        {
            for expected in [
                format!("Remove target {}: {target}", index + 1),
                format!("Remove argument 1 for target {}: {target}", index + 1),
            ] {
                assert!(
                    names.contains(&expected.as_str()),
                    "missing contextual action {expected}: {names:?}"
                );
            }
        }
        assert_eq!(view.draft.entries.len(), 2);
        assert!(!view.task.running());
        view.shutdown();
        host.shutdown();
    }
}

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
    fn accept_paths(&mut self, kind: LaunchKind, paths: &[std::path::PathBuf]) -> CoreResult<()> {
        // Validate the whole admitted batch before appending its first entry.
        let selected = paths
            .iter()
            .take(32usize.saturating_sub(self.draft.entries.len()))
            .map(|path| {
                crate::host::path_text(path).map(|target| LaunchEntry {
                    kind,
                    target,
                    args: vec![],
                })
            })
            .collect::<CoreResult<Vec<_>>>();
        self.pending_launch = None;
        self.pending_delete = None;
        self.task.abandon();
        self.message = None;
        match selected {
            Ok(selected) => {
                self.draft.entries.extend(selected);
                self.error = None;
                Ok(())
            }
            Err(error) => {
                self.error = Some(format!(
                    "Selected application/file paths: {error}. The saved set and draft were kept; review them before launching."
                ));
                Err(error)
            }
        }
    }
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
            self.new_set();
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
                    let _ = self.accept_paths(kind, &paths);
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
                ui.label("Launched applications may perform their own actions, including network activity. Unsaved edits are not used. Cancel/Quit prevents remaining launch requests; applications already requested remain under your control.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.task.running(), egui::Button::new("Confirm launch")).clicked() {
                        self.pending_launch = None;
                        let shared = shared.clone();
                        let permit = shared.reserve();
                        self.task.start(ui.ctx(), move |cancel, _| {
                            let count = launch_saved_set(&shared, permit, &set, &cancel, &mut |path| {
                                // Observe immediate launcher failures without waiting for an
                                // intentionally external application to exit. A request already
                                // admitted to the OS cannot be revoked by a later cancellation.
                                open::that_detached_with_timeout(path, std::time::Duration::from_millis(250))
                                    .map_err(|e| runtime::error(e.to_string()))
                            })?;
                            Ok(format!("Requested launch of {count} saved applications/files."))
                        });
                    }
                    if ui.button("Cancel launch review").clicked() {
                        self.pending_launch = None;
                    }
                });
            });
        }
        if let Some(id) = self.pending_delete.clone() {
            let name = stored
                .sets
                .iter()
                .find(|set| set.id == id)
                .map_or(id.as_str(), |set| set.name.as_str());
            ui.label(format!("Delete saved configuration “{name}”? The applications and files themselves are kept."));
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
    fn new_set(&mut self) {
        self.draft = empty_set();
        self.selected = self.draft.id.clone();
        self.pending_launch = None;
        self.pending_delete = None;
        self.message = None;
        self.error = None;
    }
    pub fn leave_tool(&mut self, id: &str) {
        if id == "launch-app-sets" {
            self.task.abandon();
            self.pending_launch = None;
            self.pending_delete = None;
        }
    }
}

fn launch_saved_set(
    shared: &Shared,
    permit: u64,
    set: &AppSet,
    cancel: &oxbox_core::jobs::CancelToken,
    open: &mut dyn FnMut(&str) -> CoreResult<()>,
) -> CoreResult<usize> {
    cancel.check()?;
    // Admission is synchronized with shutdown, without holding system state
    // across an external file handler. Per-target checks stop the remaining list.
    // Cancellation cannot revoke an OS call admitted immediately before it.
    shared.with_reserved_cancellable(permit, cancel, |_| Ok(()))?;
    launch_apps::launch_with_cancel(set, cancel, open)
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
    fn verification56_app_set_picker_refuses_a_lossy_or_multiline_batch_without_partial_edits() {
        let directory = tempfile::tempdir().unwrap();
        #[cfg(windows)]
        let unrepresentable = {
            use std::os::windows::ffi::OsStringExt;
            std::path::PathBuf::from(std::ffi::OsString::from_wide(&[b'x' as u16, 0xd800]))
        };
        #[cfg(unix)]
        let unrepresentable = {
            use std::os::unix::ffi::OsStringExt;
            std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![b'x', 0xff]))
        };
        #[cfg(not(any(windows, unix)))]
        let unrepresentable = std::path::PathBuf::from("generated\ninvalid");
        for invalid in [
            unrepresentable,
            directory.path().join("generated\ninvalid"),
            directory.path().join("generated\rinvalid"),
        ] {
            let mut view = AppSetsView::default();
            view.draft.name = "Generated previous set".into();
            view.draft.entries.push(LaunchEntry {
                kind: LaunchKind::File,
                target: directory.path().join("old.txt").to_str().unwrap().into(),
                args: vec![],
            });
            let previous = serde_json::to_value(&view.draft).unwrap();
            view.pending_launch = Some(view.draft.clone());
            view.pending_delete = Some(view.draft.id.clone());
            view.task.result = Some("Generated old launch completion".into());
            view.message = Some("Generated previous success".into());
            assert!(
                view.accept_paths(
                    LaunchKind::File,
                    &[directory.path().join("valid-first.txt"), invalid]
                )
                .is_err()
            );
            assert_eq!(
                serde_json::to_value(&view.draft).unwrap(),
                previous,
                "no earlier path from a refused batch may be appended"
            );
            assert!(
                view.pending_launch.is_none()
                    && view.pending_delete.is_none()
                    && view.task.result.is_none()
                    && view.message.is_none()
            );
            assert!(
                view.error
                    .as_deref()
                    .is_some_and(|error| error.contains("Selected application/file paths"))
            );
            let valid = directory.path().join("画像 with spaces ");
            view.accept_paths(LaunchKind::File, std::slice::from_ref(&valid))
                .unwrap();
            assert_eq!(
                view.draft.entries.last().unwrap().target,
                valid.to_str().unwrap()
            );
            assert!(view.error.is_none());
            view.shutdown();
        }
    }

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

    #[test]
    fn launch_admission_releases_system_gate_and_honors_cancel_between_targets() {
        let directory = tempfile::tempdir().unwrap();
        let shared = Shared::default();
        let cancel = oxbox_core::jobs::CancelToken::default();
        let mut set = empty_set();
        set.name = "Generated launch fixture".into();
        for name in ["first.txt", "second.txt"] {
            let filename = directory.path().join(name);
            std::fs::write(&filename, "generated").unwrap();
            set.entries.push(LaunchEntry {
                kind: LaunchKind::File,
                target: filename.to_string_lossy().into_owned(),
                args: vec![],
            });
        }
        let mut opened = 0;
        assert_eq!(
            launch_saved_set(&shared, shared.reserve(), &set, &cancel, &mut |_| {
                assert!(
                    shared.state.try_lock().is_ok(),
                    "external handler must not hold system state gate"
                );
                opened += 1;
                cancel.cancel();
                Ok(())
            })
            .unwrap_err()
            .code(),
            "cancelled"
        );
        assert_eq!(opened, 1);
        let stale = shared.reserve();
        shared.invalidate_pending();
        let fresh_token = oxbox_core::jobs::CancelToken::default();
        assert_eq!(
            launch_saved_set(&shared, stale, &set, &fresh_token, &mut |_| {
                panic!("stale launch must not reach a backend");
            })
            .unwrap_err()
            .code(),
            "cancelled"
        );
    }

    #[test]
    fn a_new_application_set_discards_reviews_for_the_preceding_set() {
        let mut view = AppSetsView::default();
        view.pending_launch = Some(view.draft.clone());
        view.pending_delete = Some(view.draft.id.clone());
        view.new_set();
        assert!(view.pending_launch.is_none());
        assert!(view.pending_delete.is_none());
        assert_eq!(view.selected, view.draft.id);
        assert!(view.draft.name.is_empty());
    }
}

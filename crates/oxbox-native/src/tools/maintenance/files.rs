//! Concrete reviewed Batch Rename and explicit permanent File Shred forms.
use crate::{
    host::{self, Host},
    jobs::Task,
};
use eframe::egui;
use oxbox_core::{
    CoreError, CoreResult,
    tools::{
        batch_rename::{self, Counter, EntryState, Journal, Options},
        file_shred,
    },
};
use std::path::PathBuf;

const JOURNAL: &str = "native-batch-rename.json";
#[derive(Default)]
pub struct FileMaintenance {
    paths: String,
    options: Options,
    counter_enabled: bool,
    counter_start: u64,
    counter_width: u8,
    counter_separator: String,
    rename_preview: Task<batch_rename::Plan>,
    shred_review: Task<file_shred::Plan>,
    action: Task<String>,
    journal: Option<Journal>,
    loaded: bool,
    reload_journal: bool,
    storage_error: Option<String>,
    rename_draft: Option<(String, Options)>,
    shred_draft: Option<String>,
    confirmation: String,
    acknowledged: bool,
}
impl FileMaintenance {
    pub fn tick(&mut self) {
        self.rename_preview.poll();
        self.shred_review.poll();
        let was_running = self.action.running();
        self.action.poll();
        if was_running && !self.action.running() {
            self.reload_journal = true;
            self.rename_preview.result = None;
            self.shred_review.result = None;
            self.confirmation.clear();
            self.acknowledged = false;
        }
    }
    fn load(&mut self, host: &Host) {
        if self.loaded && !self.reload_journal {
            return;
        }
        self.loaded = true;
        self.reload_journal = false;
        match host.load::<Journal>(JOURNAL).and_then(|journal| {
            if let Some(journal) = &journal {
                batch_rename::validate_journal(journal)?;
            }
            Ok(journal)
        }) {
            Ok(journal) => {
                self.journal = journal;
                self.storage_error = None;
            }
            Err(error) => {
                self.storage_error = Some(format!(
                    "Undo storage could not load: {error}. Preserve the file and repair it before applying renames."
                ));
            }
        }
    }
    fn paths(&self) -> CoreResult<Vec<PathBuf>> {
        if self.paths.len() > 128 * 1024 {
            return Err(CoreError::InvalidInput(
                "selected path text exceeds 128 KiB".into(),
            ));
        }
        Ok(self
            .paths
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect())
    }
    fn selected_files(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.add_enabled_ui(!busy, |ui| {
            let label = ui.label("Selected files · one absolute path per line");
            ui.add(
                egui::TextEdit::multiline(&mut self.paths)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(label.id);
            ui.horizontal(|ui| {
                if ui.button("Choose files…").clicked()
                    && let Some(paths) = host::dialogs::file(ui.ctx())
                        .set_title("Select explicit regular files")
                        .pick_files()
                {
                    self.paths = paths
                        .iter()
                        .map(|path| path.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("\n");
                }
                if ui.button("Clear selected files").clicked() {
                    self.paths.clear();
                }
            });
            if ui.is_enabled() {
                let dropped = ui.ctx().input(|input| {
                    input
                        .raw
                        .dropped_files
                        .iter()
                        .map(|file| file.path().to_path_buf())
                        .collect::<Vec<_>>()
                });
                if !dropped.is_empty() {
                    self.paths = dropped
                        .iter()
                        .map(|path| path.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("\n");
                }
            }
        });
        ui.small("Up to 256 local regular files. No folder recursion, symbolic/reparse links, hard links or system files. Remote and unrecognized filesystems are refused.");
    }
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, host: &mut Host) -> bool {
        if !matches!(id, "batch-rename" | "file-shred") {
            return false;
        }
        self.tick();
        self.load(host);
        match id {
            "batch-rename" => self.rename(ui, host),
            "file-shred" => self.shred(ui),
            _ => {}
        }
        true
    }
    fn rename(&mut self, ui: &mut egui::Ui, host: &Host) {
        ui.label("Preview literal filename changes, then review the exact before/after paths. Existing files are never replaced. Undo covers the latest journaled batch and refuses changed files.");
        let busy =
            self.rename_preview.running() || self.action.running() || self.shred_review.running();
        self.selected_files(ui, busy);
        ui.add_enabled_ui(!busy, |ui| {
            field(ui, "Replace literal text", &mut self.options.replace_from);
            field(ui, "Replacement text", &mut self.options.replace_to);
            field(ui, "Prefix", &mut self.options.prefix);
            field(ui, "Suffix", &mut self.options.suffix);
            ui.checkbox(
                &mut self.options.preserve_extension,
                "Preserve the filename extension",
            );
            ui.checkbox(&mut self.counter_enabled, "Append a counter");
            if self.counter_enabled {
                if self.counter_width == 0 {
                    self.counter_width = 3;
                    self.counter_start = 1;
                    self.counter_separator = "_".into();
                }
                number(ui, "First counter value", &mut self.counter_start);
                let label = ui.label("Counter digits (1–20)");
                ui.add(egui::DragValue::new(&mut self.counter_width).range(1..=20))
                    .labelled_by(label.id);
                field(ui, "Counter separator", &mut self.counter_separator);
            }
            self.options.counter = self.counter_enabled.then(|| Counter {
                start: self.counter_start,
                width: self.counter_width,
                separator: self.counter_separator.clone(),
            });
        });
        let draft = (self.paths.clone(), self.options.clone());
        if self.rename_draft.as_ref() != Some(&draft) {
            self.rename_draft = Some(draft);
            self.rename_preview.abandon();
            if !self.action.running() {
                self.action.result = None;
                self.action.error = None;
                self.action.progress = None;
            }
        }
        if ui
            .add_enabled(
                !busy && !self.paths.trim().is_empty(),
                egui::Button::new("Preview rename"),
            )
            .clicked()
        {
            match self.paths() {
                Ok(paths) => {
                    let options = self.options.clone();
                    self.rename_preview
                        .start(ui.ctx(), move |cancel, progress| {
                            batch_rename::preview(&paths, &options, &cancel, progress)
                        });
                }
                Err(error) => self.rename_preview.error = Some(error.to_string()),
            }
        }
        self.rename_preview.show(ui);
        if let Some(plan) = self.rename_preview.result.clone() {
            ui.strong(format!("Reviewed preview · {} file(s)", plan.entries.len()));
            egui::Grid::new("batch-rename-preview")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Current path");
                    ui.strong("Proposed path");
                    ui.end_row();
                    for entry in &plan.entries {
                        ui.label(entry.from.display().to_string());
                        ui.label(entry.to.display().to_string());
                        ui.end_row();
                    }
                });
            if ui
                .add_enabled(
                    !busy && self.storage_error.is_none(),
                    egui::Button::new("Review and apply rename…"),
                )
                .clicked()
            {
                let names = plan
                    .entries
                    .iter()
                    .map(|entry| format!("{}\n→ {}", entry.from.display(), entry.to.display()))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let description = format!(
                    "Rename exactly these {} files?\n\n{names}\n\nNo existing target is overwritten. The previous Undo record is replaced by this reviewed batch. If interrupted, open this tool to recover from its saved Undo journal.",
                    plan.entries.len()
                );
                if confirm(ui, "Apply reviewed batch rename", &description) {
                    let directory = host.data_dir().to_path_buf();
                    if self.action.start(ui.ctx(), move |cancel, progress| {
                        let journal =
                            batch_rename::apply(&plan, true, &cancel, progress, &mut |journal| {
                                host::save_at(&directory, JOURNAL, journal)
                            })?;
                        Ok(format!(
                            "Renamed {} file(s). Guarded Undo is saved.",
                            journal.entries.len()
                        ))
                    }) {
                        self.reload_journal = true;
                    }
                }
            }
        }
        if let Some(error) = &self.storage_error {
            crate::accessibility::alert(ui, error);
        }
        self.undo_view(ui, host, busy);
        self.action_view(ui);
        ui.small("A batch is not an atomic transaction. Cancellation or a refused file stops later moves; use the saved journal to review partial Undo. On Unix, concurrent directory changes cannot be made an atomic identity transaction.");
    }
    fn undo_view(&mut self, ui: &mut egui::Ui, host: &Host, busy: bool) {
        let Some(journal) = self
            .journal
            .clone()
            .filter(|journal| !journal.entries.is_empty())
        else {
            return;
        };
        ui.separator();
        ui.strong("Latest batch Undo record");
        for item in &journal.entries {
            ui.label(format!(
                "{:?}: {} → {}",
                item.state,
                item.entry.from.display(),
                item.entry.to.display()
            ));
        }
        let pending = journal
            .entries
            .iter()
            .any(|item| item.state != EntryState::Undone);
        if ui
            .add_enabled(
                pending && !busy && self.storage_error.is_none(),
                egui::Button::new("Review Undo…"),
            )
            .clicked()
        {
            let names = journal
                .entries
                .iter()
                .map(|item| {
                    format!(
                        "{}\n→ {}",
                        item.entry.to.display(),
                        item.entry.from.display()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            if confirm(
                ui,
                "Undo reviewed batch rename",
                &format!(
                    "Restore only matching renamed files to these original names?\n\n{names}\n\nChanged or occupied paths are refused. Original filenames are never overwritten."
                ),
            ) {
                let directory = host.data_dir().to_path_buf();
                self.action.start(ui.ctx(), move |cancel, progress| {
                    let journal =
                        batch_rename::undo(&journal, true, &cancel, progress, &mut |journal| {
                            host::save_at(&directory, JOURNAL, journal)
                        })?;
                    Ok(format!(
                        "Undo complete for {} reviewed file(s).",
                        journal.entries.len()
                    ))
                });
            }
        }
        if ui
            .add_enabled(
                !busy && self.storage_error.is_none(),
                egui::Button::new("Discard Undo record…"),
            )
            .clicked()
            && confirm(
                ui,
                "Discard only the Undo record",
                "Discard this saved rename recovery record? This changes no user file, but these renames can no longer be restored through this journal.",
            )
        {
            match self.discard_undo(ui.ctx(), host) {
                Ok(()) => {}
                Err(error) => self.storage_error = Some(error.to_string()),
            }
        }
    }
    fn discard_undo(&mut self, context: &egui::Context, host: &Host) -> CoreResult<()> {
        crate::jobs::register_context(context).check_open()?;
        host.save(JOURNAL, &Journal::default())?;
        self.journal = Some(Journal::default());
        Ok(())
    }
    fn shred(&mut self, ui: &mut egui::Ui) {
        ui.strong("Permanent deletion · bypasses Recycle Bin/Trash · no Undo");
        ui.label(file_shred::LIMITATION);
        ui.label("Only the selected file bytes receive one zero-fill pass, a sync and removal. No disk wipe, free-space wipe, folder recursion, elevated access or automatic start is provided.");
        let busy =
            self.shred_review.running() || self.rename_preview.running() || self.action.running();
        self.selected_files(ui, busy);
        if self.shred_draft.as_ref() != Some(&self.paths) {
            self.shred_draft = Some(self.paths.clone());
            self.shred_review.abandon();
            self.confirmation.clear();
            self.acknowledged = false;
            if !self.action.running() {
                self.action.result = None;
                self.action.error = None;
                self.action.progress = None;
            }
        }
        if ui
            .add_enabled(
                !busy && !self.paths.trim().is_empty(),
                egui::Button::new("Review selected files without changing them"),
            )
            .clicked()
        {
            match self.paths() {
                Ok(paths) => {
                    self.shred_review.start(ui.ctx(), move |cancel, progress| {
                        file_shred::review(&paths, &cancel, progress)
                    });
                }
                Err(error) => self.shred_review.error = Some(error.to_string()),
            }
        }
        self.shred_review.show(ui);
        if let Some(plan) = self.shred_review.result.clone() {
            ui.strong(format!(
                "Exact reviewed selection: {} file(s), {} bytes",
                plan.files.len(),
                plan.total_bytes
            ));
            for file in &plan.files {
                ui.label(format!("{} · {} bytes", file.path.display(), file.bytes));
            }
            ui.add_enabled_ui(!busy,|ui|{
                ui.checkbox(&mut self.acknowledged,"I understand SSD/flash, snapshots, copy-on-write and backups may retain data; Cancel may leave a partially overwritten file.");
                field(ui,"Type DELETE to enable permanent removal",&mut self.confirmation);
            });
            if ui
                .add_enabled(
                    !busy && self.acknowledged && self.confirmation == file_shred::CONFIRMATION,
                    egui::Button::new("Review permanent deletion…"),
                )
                .clicked()
            {
                let names = plan
                    .files
                    .iter()
                    .map(|file| format!("{} · {} bytes", file.path.display(), file.bytes))
                    .collect::<Vec<_>>()
                    .join("\n");
                let description = format!(
                    "Permanently overwrite and remove exactly {} selected file(s), {} bytes?\n\n{names}\n\nThis bypasses Trash and cannot be undone. {}",
                    plan.files.len(),
                    plan.total_bytes,
                    file_shred::LIMITATION
                );
                if confirm(
                    ui,
                    "Confirm irreversible selected-file removal",
                    &description,
                ) {
                    let phrase = self.confirmation.clone();
                    let acknowledged = self.acknowledged;
                    if self.action.start(ui.ctx(),move|cancel,progress|{
                        let report=file_shred::shred(&plan,&phrase,acknowledged,&cancel,progress)?;
                        Ok(format!("Permanently removed {} selected file(s) after overwriting {} bytes. Physical erasure is not guaranteed.",report.removed_files,report.overwritten_bytes))
                    }){self.confirmation.clear();self.acknowledged=false;}
                }
            }
        }
        self.action_view(ui);
        ui.small("Cancel stops future chunks/removals; it cannot recover bytes already overwritten or files already removed. Concurrent Unix directory changes remain a namespace race; this is not an adversarial or certified sanitization service.");
    }
    fn action_view(&mut self, ui: &mut egui::Ui) {
        self.action.show(ui);
        if let Some(message) = &self.action.result {
            crate::accessibility::status(ui, message);
        }
    }
    pub fn deactivate(&mut self) {
        self.confirmation.clear();
        self.acknowledged = false;
    }
    pub fn leave_tool(&mut self, id: &str) {
        if matches!(id, "batch-rename" | "file-shred") {
            self.rename_preview.abandon();
            self.shred_review.abandon();
            self.action.abandon();
            self.reload_journal = true;
            self.deactivate();
        }
    }
    pub fn shutdown(&mut self) {
        self.rename_preview.shutdown();
        self.shred_review.shutdown();
        self.action.shutdown();
        self.deactivate();
    }
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    let label = ui.label(label);
    ui.text_edit_singleline(value).labelled_by(label.id);
}
fn number(ui: &mut egui::Ui, label: &str, value: &mut u64) {
    let label = ui.label(label);
    ui.add(egui::DragValue::new(value)).labelled_by(label.id);
}
fn confirm(ui: &egui::Ui, title: &str, description: &str) -> bool {
    host::dialogs::message(ui.ctx())
        .set_title(title)
        .set_description(description)
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

#[cfg(test)]
mod tests {
    use super::*;
    fn related_text(
        nodes: &[(egui::accesskit::NodeId, egui::accesskit::Node)],
        id: egui::accesskit::NodeId,
        depth: usize,
    ) -> String {
        if depth > 8 {
            return String::new();
        }
        let Some((_, node)) = nodes.iter().find(|(node_id, _)| *node_id == id) else {
            return String::new();
        };
        if let Some(text) = node.label().or_else(|| node.value()) {
            return text.to_owned();
        }
        node.children()
            .iter()
            .map(|child| related_text(nodes, *child, depth + 1))
            .collect::<Vec<_>>()
            .join(" ")
    }
    #[test]
    fn both_file_maintenance_forms_start_idle_with_named_controls_and_no_effects() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut tools = FileMaintenance::default();
        for id in ["batch-rename", "file-shred"] {
            let nodes =
                crate::accessibility::test_tree(|ui| assert!(tools.show(ui, id, &mut host)));
            crate::accessibility::assert_named_controls(&nodes, id);
            assert!(!tools.action.running());
            assert!(!tools.rename_preview.running());
            assert!(!tools.shred_review.running());
        }
        assert!(!tools.acknowledged);
        assert!(tools.confirmation.is_empty());
        assert!(tools.journal.is_none());
        tools.shutdown();
        host.shutdown();
    }
    #[test]
    fn changing_paths_clears_shred_admission_without_starting_work() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut tools = FileMaintenance {
            confirmation: "DELETE".into(),
            acknowledged: true,
            shred_draft: Some("old".into()),
            ..Default::default()
        };
        let _ =
            crate::accessibility::test_tree(|ui| assert!(tools.show(ui, "file-shred", &mut host)));
        assert!(!tools.acknowledged);
        assert!(tools.confirmation.is_empty());
        assert!(!tools.action.running());
        tools.shutdown();
        host.shutdown();
    }
    #[test]
    fn exact_review_and_undo_branches_render_named_controls_without_invoking_effects() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let path = directory.path().join("generated-only-model.txt");
        let target = directory.path().join("renamed-only-model.txt");
        // Serialized model only: neither file is created, opened, renamed or removed.
        let snapshot=serde_json::from_value(serde_json::json!({"path":path,"bytes":7,"device":1,"file_id":1,"modified_secs":1,"modified_nanos":0,"links":1,"hash":[7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7]})).unwrap();
        let entry = batch_rename::Entry {
            from: path.clone(),
            to: target,
            snapshot,
        };
        let mut tools = FileMaintenance::default();
        tools.paths = path.to_string_lossy().into_owned();
        tools.loaded = true;
        tools.rename_draft = Some((tools.paths.clone(), tools.options.clone()));
        tools.rename_preview.result = Some(batch_rename::Plan {
            entries: vec![entry.clone()],
        });
        tools.journal = Some(Journal {
            entries: vec![batch_rename::JournalEntry {
                entry: entry.clone(),
                state: EntryState::Moved,
            }],
        });
        let rename_nodes = crate::accessibility::test_tree(|ui| {
            assert!(tools.show(ui, "batch-rename", &mut host))
        });
        crate::accessibility::assert_named_controls(&rename_nodes, "batch-rename review and Undo");
        assert!(
            rename_nodes
                .iter()
                .any(|node| node.label() == Some("Review Undo…"))
        );
        tools.shred_draft = Some(tools.paths.clone());
        tools.shred_review.result = Some(file_shred::Plan {
            files: vec![entry.snapshot],
            total_bytes: 7,
        });
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            assert!(tools.show(ui, "file-shred", &mut host))
        });
        output.textures_delta.clear();
        let shred_nodes = output
            .platform_output
            .accesskit_update
            .take()
            .unwrap()
            .nodes;
        crate::accessibility::assert_named_controls(
            &shred_nodes
                .iter()
                .map(|(_, node)| node.clone())
                .collect::<Vec<_>>(),
            "file-shred exact review",
        );
        assert!(shred_nodes.iter().any(|(_, node)| node.role()
            == egui::accesskit::Role::TextInput
            && node.labelled_by().iter().any(|id| {
                related_text(&shred_nodes, *id, 0)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    == "Type DELETE to enable permanent removal"
            })));
        assert!(!tools.action.running());
        assert!(!path.exists());
        tools.shutdown();
        host.shutdown();
    }
    #[test]
    fn discard_undo_after_quit_preserves_the_saved_journal() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let from = directory.path().join("original-model.txt");
        let to = directory.path().join("renamed-model.txt");
        let journal: Journal = serde_json::from_value(serde_json::json!({"entries":[{"entry":{"from":from,"to":to,"snapshot":{"path":from,"bytes":1,"device":1,"file_id":1,"modified_secs":1,"modified_nanos":0,"links":1,"hash":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}},"state":"Moved"}]})).unwrap();
        host.save(JOURNAL, &journal).unwrap();
        let before = std::fs::read(directory.path().join(JOURNAL)).unwrap();
        let mut tools = FileMaintenance {
            journal: Some(journal.clone()),
            ..Default::default()
        };
        let context = egui::Context::default();
        crate::jobs::register_context(&context).close();
        assert!(tools.discard_undo(&context, &host).is_err());
        assert_eq!(tools.journal, Some(journal));
        assert_eq!(
            std::fs::read(directory.path().join(JOURNAL)).unwrap(),
            before
        );
        tools.shutdown();
        host.shutdown();
    }
}

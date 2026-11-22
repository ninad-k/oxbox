//! Explicit, reviewed maintenance forms. Opening a form starts no scan or OS effect.
use crate::{host::Host, jobs::Task};
use eframe::egui::{self, Ui};
use oxbox_core::{
    platform::Os,
    tools::{registry_review as registry, startup_manager as startup, storage_analysis as storage},
};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

enum ReadResult {
    Startup(Vec<startup::StartupEntry>),
    Storage(storage::StorageReport),
    Registry(Vec<registry::Candidate>),
    StartupRecovery(startup::RecoveryReview),
    RegistryRecovery(registry::RecoveryReview),
}
#[derive(Clone)]
enum Pending {
    Disable(startup::StartupEntry),
    Remove(registry::Candidate),
    RestoreStartup(startup::RecoveryReview),
    RestoreRegistry(registry::RecoveryReview),
}
#[derive(Default)]
pub struct SystemMaintenance {
    read: Task<ReadResult>,
    action: Task<String>,
    startup: Vec<startup::StartupEntry>,
    registry: Vec<registry::Candidate>,
    storage: Option<storage::StorageReport>,
    root: String,
    options: storage::AnalysisOptions,
    minimum_mib: u64,
    backup: String,
    recovery: String,
    pending: Option<Pending>,
}
fn bytes(value: u64) -> String {
    if value >= 1_073_741_824 {
        format!("{:.2} GiB", value as f64 / 1_073_741_824.0)
    } else if value >= 1_048_576 {
        format!("{:.1} MiB", value as f64 / 1_048_576.0)
    } else {
        format!("{value} bytes")
    }
}
fn text(ui: &mut Ui, label: &str, value: &mut String) -> bool {
    let label = ui.label(label);
    ui.text_edit_singleline(value)
        .labelled_by(label.id)
        .changed()
}
fn default_backup(host: &Host, prefix: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    host.data_dir()
        .join(format!("{prefix}-{stamp}.json"))
        .display()
        .to_string()
}
#[cfg(test)]
fn is_owned_startup_command(name: &str, command: &str, executable: &std::path::Path) -> bool {
    if !name.eq_ignore_ascii_case("Oxbox") {
        return false;
    }
    let Some(path) = executable.to_str() else {
        return false;
    };
    let Ok(quoted) = oxbox_core::startup::executable_path(path, true) else {
        return false;
    };
    let expected = format!("{quoted} --autostart");
    // Reuse the core's exact ownership policy with a no-effect callback. This
    // is an identity query, never a request to remove the registration.
    oxbox_core::startup::remove_owned_command(command, &expected, &mut || Ok(())).unwrap_or(false)
}
#[cfg(test)]
fn is_owned_startup_recovery(
    review: &startup::RecoveryReview,
    executable: &std::path::Path,
) -> bool {
    review
        .run_entry_identity()
        .is_some_and(|(name, command)| is_owned_startup_command(&name, &command, executable))
}
fn current_app_identity() -> oxbox_core::CoreResult<(PathBuf, PathBuf)> {
    Ok((
        std::env::current_exe()?,
        dirs::home_dir().ok_or_else(|| {
            oxbox_core::CoreError::InvalidInput(
                "Cannot identify the current app's startup scope.".into(),
            )
        })?,
    ))
}
impl SystemMaintenance {
    pub fn tick(&mut self) {
        self.read.poll();
        self.action.poll();
        if let Some(result) = self.read.result.take() {
            match result {
                ReadResult::Startup(entries) => self.startup = entries,
                ReadResult::Registry(entries) => self.registry = entries,
                ReadResult::Storage(report) => self.storage = Some(report),
                ReadResult::StartupRecovery(review) => {
                    self.pending = Some(Pending::RestoreStartup(review))
                }
                ReadResult::RegistryRecovery(review) => {
                    self.pending = Some(Pending::RestoreRegistry(review))
                }
            }
        }
    }
    pub fn show(&mut self, ui: &mut Ui, id: &str, host: &mut Host) -> bool {
        if !matches!(
            id,
            "startup-manager" | "storage-analyzer" | "registry-review"
        ) {
            return false;
        }
        self.tick();
        match id {
            "storage-analyzer" => self.storage(ui),
            "startup-manager" => self.startup(ui, host),
            _ => self.registry(ui, host),
        }
        self.confirmation(ui);
        self.read.show(ui);
        self.action.show(ui);
        if let Some(message) = &self.action.result {
            crate::accessibility::status(ui, message);
        }
        if id != "storage-analyzer" {
            ui.small("Cancel prevents further requested work; a setting change already admitted to the OS may have completed. Refresh the entry and keep its recovery backup. No other application is stopped.");
        }
        true
    }
    fn storage(&mut self, ui: &mut Ui) {
        ui.label("Analyze an explicitly selected local folder without deleting or changing files. Logical file sizes can differ from allocated disk space; hard-linked paths can be counted more than once. Links and other mounted file systems are skipped.");
        let mut changed = false;
        ui.add_enabled_ui(!self.read.running(), |ui| {
            changed |= text(ui, "Folder to analyze", &mut self.root);
            if ui.button("Choose analysis folder…").clicked()
                && let Some(path) = crate::host::dialogs::file(ui.ctx()).pick_folder()
            {
                self.root = path.display().to_string();
                changed = true;
            }
            ui.horizontal(|ui| {
                let label = ui.label("Maximum entries");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.options.max_entries).range(1..=1_000_000))
                    .labelled_by(label.id)
                    .changed();
            });
            ui.horizontal(|ui| {
                let label = ui.label("Maximum depth");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.options.max_depth).range(1..=128))
                    .labelled_by(label.id)
                    .changed();
            });
            ui.horizontal(|ui| {
                let label = ui.label("Large files at least (MiB)");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.minimum_mib).range(0..=1_048_576))
                    .labelled_by(label.id)
                    .changed();
            });
            ui.horizontal(|ui| {
                let label = ui.label("Maximum large-file results");
                changed |= ui
                    .add(egui::DragValue::new(&mut self.options.max_large_files).range(1..=1000))
                    .labelled_by(label.id)
                    .changed();
            });
        });
        if changed {
            self.storage = None;
            self.read.abandon();
        }
        if ui
            .add_enabled(
                !self.root.is_empty() && !self.read.running(),
                egui::Button::new("Analyze storage"),
            )
            .clicked()
        {
            self.storage = None;
            let root = PathBuf::from(&self.root);
            let mut options = self.options.clone();
            options.minimum_file_bytes = self.minimum_mib.saturating_mul(1_048_576);
            self.read.start(ui.ctx(), move |cancel, progress| {
                Ok(ReadResult::Storage(storage::analyze(
                    &root, &options, &cancel, progress,
                )?))
            });
        }
        if let Some(report) = &self.storage {
            ui.heading(format!(
                "{} in {} files",
                bytes(report.total_bytes),
                report.total_files
            ));
            ui.label(format!(
                "{} paths visited · {} folders · {} skipped",
                report.visited, report.directories, report.skipped
            ));
            if report.truncated {
                crate::accessibility::alert(
                    ui,
                    "Partial analysis: the entry or depth budget was reached. Totals cover only visited paths; choose a smaller folder or increase the budget.",
                );
            }
            egui::Grid::new("storage-categories")
                .striped(true)
                .show(ui, |ui| {
                    for category in &report.categories {
                        ui.label(&category.name);
                        ui.label(format!("{} files", category.files));
                        ui.label(bytes(category.bytes));
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.strong("Largest matching files (read-only)");
            for file in &report.large_files {
                ui.label(format!("{} · {}", bytes(file.bytes), file.path.display()));
            }
        }
    }
    fn startup(&mut self, ui: &mut Ui, host: &Host) {
        ui.label("Review current-user startup configuration. Disabling affects future logins; it does not stop a running program. Each change requires an exclusive recovery backup and exact revalidation. System services, scheduled tasks and machine-wide entries are outside this tool.");
        ui.small(match Os::current() { Os::Windows => "Windows: HKCU Run values only. OS StartupApproved/Task Manager choices are not changed or inferred.", Os::Macos => "macOS: this user's legacy Library/LaunchAgents files only. Modern Login Items and loaded agents are unchanged.", Os::Linux => "Linux: current-user XDG autostart desktop files only. Hidden=true preserves the user override; system entries are untouched.", Os::Other => "Current-user startup management is unavailable on this platform." });
        if ui
            .add_enabled(
                !self.read.running() && !self.action.running() && Os::current() != Os::Other,
                egui::Button::new("Review current-user startup entries"),
            )
            .clicked()
        {
            self.pending = None;
            self.read.start(ui.ctx(), |cancel, progress| {
                Ok(ReadResult::Startup(startup::list(&cancel, progress)?))
            });
        }
        let current_app = current_app_identity().ok();
        for entry in self.startup.clone() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(&entry.name); ui.label(&entry.location); ui.monospace(&entry.command); ui.small(&entry.note);
                ui.label(if entry.enabled { "Startup registration/file is present; OS conditions may still prevent it from running." } else { "This desktop entry is already hidden." });
                if current_app.as_ref().is_some_and(|(executable, home)| entry.belongs_to_application("Oxbox", executable, home, Os::current())) {
                    ui.label("Manage Oxbox startup in Settings");
                    return;
                }
                let disable = ui.add_enabled(entry.enabled && !self.action.running() && !self.read.running(), egui::Button::new("Review disabling…"));
                crate::accessibility::name(&disable, &format!("Review disabling startup {}: {}", entry.name, entry.location));
                if disable.clicked() {
                    self.backup = default_backup(host, "startup-backup"); self.pending = Some(Pending::Disable(entry.clone()));
                }
            });
        }
        self.recovery_picker(ui, true);
    }
    fn registry(&mut self, ui: &mut Ui, host: &Host) {
        ui.label("Conservative registry review: only obsolete default executable mappings under HKCU App Paths. A candidate must point to a missing unambiguous .exe on an available fixed local drive. No HKLM, COM, driver, service, uninstall-key or performance heuristics are used.");
        if Os::current() != Os::Windows {
            ui.label("Registry review is available only on Windows.");
            return;
        }
        if ui
            .add_enabled(
                !self.read.running() && !self.action.running(),
                egui::Button::new("Analyze current-user executable mappings"),
            )
            .clicked()
        {
            self.pending = None;
            self.read.start(ui.ctx(), |cancel, progress| {
                Ok(ReadResult::Registry(registry::scan(&cancel, progress)?))
            });
        }
        for entry in self.registry.clone() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(&entry.location);
                ui.label(format!("Missing target: {}", entry.target.display()));
                ui.small(&entry.reason);
                let remove = ui.add_enabled(
                    !self.action.running() && !self.read.running(),
                    egui::Button::new("Review removing mapping…"),
                );
                crate::accessibility::name(
                    &remove,
                    &format!("Review removing executable mapping {}", entry.location),
                );
                if remove.clicked() {
                    self.backup = default_backup(host, "registry-backup");
                    self.pending = Some(Pending::Remove(entry.clone()));
                }
            });
        }
        ui.small("No candidates does not mean a registry is healthy or unhealthy. Inaccessible, network, environment-expanded and ambiguous targets are kept. Backups are Oxbox JSON recovery files, not scripts or .reg files.");
        self.recovery_picker(ui, false);
    }
    fn recovery_picker(&mut self, ui: &mut Ui, startup: bool) {
        ui.separator();
        ui.strong("Restore / enable from an Oxbox recovery backup");
        let mut changed = false;
        ui.add_enabled_ui(!self.read.running() && !self.action.running(), |ui| {
            changed = text(ui, "Recovery backup (.json)", &mut self.recovery);
            if ui.button("Choose recovery backup…").clicked()
                && let Some(path) = crate::host::dialogs::file(ui.ctx())
                    .add_filter("Oxbox recovery", &["json"])
                    .pick_file()
            {
                self.recovery = path.display().to_string();
                changed = true;
            }
        });
        if changed {
            self.pending = None;
            self.read.abandon();
        }
        if ui
            .add_enabled(
                !self.recovery.is_empty() && !self.read.running() && !self.action.running(),
                egui::Button::new("Review recovery backup"),
            )
            .clicked()
        {
            self.pending = None;
            let path = PathBuf::from(&self.recovery);
            self.read.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                if startup {
                    Ok(ReadResult::StartupRecovery(startup::review_recovery(
                        &path,
                    )?))
                } else {
                    Ok(ReadResult::RegistryRecovery(registry::review_recovery(
                        &path,
                    )?))
                }
            });
        }
    }
    fn confirmation(&mut self, ui: &mut Ui) {
        let Some(pending) = self.pending.clone() else {
            return;
        };
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("Review the exact change");
            if let Pending::RestoreStartup(review) = &pending
                && current_app_identity().is_ok_and(|(executable, home)| review.belongs_to_application("Oxbox", &executable, &home, Os::current()))
            {
                ui.label("Manage Oxbox startup in Settings");
                ui.small("This backup belongs to the current Oxbox executable. Startup Manager will not override the app's saved login choice.");
                if ui.button("Close recovery review").clicked() { self.pending = None; }
                return;
            }
            match &pending {
                Pending::Disable(entry) => { ui.label(format!("Disable {} for future logins?\n{}\n{}", entry.name, entry.location, entry.command)); },
                Pending::Remove(entry) => { ui.label(format!("Remove only the reviewed default value at {}?\nMissing target: {}\nAll other values/keys are kept.", entry.location, entry.target.display())); },
                Pending::RestoreStartup(review) => { ui.label(&review.description); },
                Pending::RestoreRegistry(review) => { ui.label(&review.description); },
            }
            let needs_backup = matches!(pending, Pending::Disable(_) | Pending::Remove(_));
            if needs_backup { ui.add_enabled_ui(!self.action.running(), |ui| { text(ui, "New exclusive recovery backup", &mut self.backup); if ui.button("Choose new backup filename…").clicked() && let Some(path) = crate::host::dialogs::file(ui.ctx()).set_file_name("oxbox-recovery.json").save_file() { self.backup = path.display().to_string(); } }); ui.small("The backup must be new. If the entry changes or backup verification fails, the reviewed operation is refused. Keep the backup after any error or cancellation."); }
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.action.running() && (!needs_backup || !self.backup.is_empty()), egui::Button::new("Confirm reviewed change")).clicked() {
                    let backup = PathBuf::from(&self.backup);
                    if self.action.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        match pending {
                            Pending::Disable(entry) => { let (executable, home) = current_app_identity()?; if entry.belongs_to_application("Oxbox", &executable, &home, Os::current()) { return Err(oxbox_core::CoreError::InvalidInput("Manage Oxbox startup in Settings".into())); } startup::disable(&entry, &backup, true, &cancel)?; Ok(format!("Disabled startup “{}” at {} for future logins. Backup: {}. Running applications were not stopped. Refresh to inspect the current entry.", entry.name, entry.location, backup.display())) },
                            Pending::Remove(entry) => { registry::remove(&entry, &backup, true, &cancel)?; Ok(format!("Removed only the reviewed stale mapping at {}. Backup: {}. Refresh to inspect the current list.", entry.location, backup.display())) },
                            Pending::RestoreStartup(review) => { let (executable, home) = current_app_identity()?; if review.belongs_to_application("Oxbox", &executable, &home, Os::current()) { return Err(oxbox_core::CoreError::InvalidInput("Manage Oxbox startup in Settings".into())); } startup::restore_reviewed(&review, true, &cancel)?; Ok("Restored the reviewed startup configuration. It applies to future logins; no application was launched. Refresh to inspect it.".into()) },
                            Pending::RestoreRegistry(review) => { registry::restore_reviewed(&review, true, &cancel)?; Ok("Restored the exact reviewed mapping without overwriting an existing value. Refresh to inspect it.".into()) },
                        }
                    }) { self.pending = None; self.read.abandon(); self.startup.clear(); self.registry.clear(); }
                }
                if ui.button("Cancel review").clicked() { self.pending = None; }
            });
        });
    }
    pub fn leave_tool(&mut self, id: &str) {
        if matches!(
            id,
            "startup-manager" | "storage-analyzer" | "registry-review"
        ) {
            self.read.abandon();
            self.action.abandon();
            self.pending = None;
            self.recovery.clear();
        }
    }
    pub fn shutdown(&mut self) {
        self.read.shutdown();
        self.action.shutdown();
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_manager_own_command_is_delegated_to_settings() {
        let exe = std::path::Path::new("C:\\Apps\\Oxbox\\oxbox-native.exe");
        assert!(is_owned_startup_command(
            "Oxbox",
            "\"C:\\Apps\\Oxbox\\oxbox-native.exe\" --autostart",
            exe
        ));
        for (name, command) in [
            (
                "Other application",
                "\"C:\\Apps\\Oxbox\\oxbox-native.exe\" --autostart",
            ),
            ("Oxbox", "\"C:\\Other\\oxbox-native.exe\" --autostart"),
            (
                "Oxbox",
                "\"C:\\Apps\\Oxbox\\oxbox-native.exe\" --autostart --extra",
            ),
            ("Oxbox", "custom-user-command"),
        ] {
            assert!(
                !is_owned_startup_command(name, command, exe),
                "a repurposed entry must not be misidentified"
            );
        }
    }
    #[test]
    fn startup_manager_owned_recovery_is_delegated_to_settings() {
        let root = tempfile::tempdir().unwrap();
        let executable = std::path::Path::new("C:\\Apps\\Oxbox\\oxbox-native.exe");
        for (index, (name, command, owned)) in [
            (
                "Oxbox",
                "\"C:\\Apps\\Oxbox\\oxbox-native.exe\" --autostart",
                true,
            ),
            (
                "Other",
                "\"C:\\Apps\\Oxbox\\oxbox-native.exe\" --autostart",
                false,
            ),
            (
                "Oxbox",
                "\"C:\\Other\\oxbox-native.exe\" --autostart",
                false,
            ),
            (
                "Oxbox",
                "\"C:\\Apps\\Oxbox\\oxbox-native.exe\" --autostart --custom",
                false,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let filename = root.path().join(format!("generated-{index}.json"));
            let data: Vec<_> = command
                .encode_utf16()
                .chain(Some(0))
                .flat_map(u16::to_le_bytes)
                .collect();
            let backup = serde_json::json!({"schema":1,"purpose":"oxbox-startup","source":{"Run":{"key":"Software\\Microsoft\\Windows\\CurrentVersion\\Run","name":name,"kind":1,"data":data}}});
            std::fs::write(&filename, serde_json::to_vec(&backup).unwrap()).unwrap();
            let review = startup::review_recovery(&filename).unwrap();
            assert_eq!(is_owned_startup_recovery(&review, executable), owned);
        }
    }
    #[test]
    fn maintenance_system_forms_have_named_controls_and_start_no_scan_or_effect() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut tools = SystemMaintenance::default();
        for id in ["startup-manager", "storage-analyzer", "registry-review"] {
            let nodes = crate::accessibility::test_tree(|ui| {
                crate::jobs::register_context(ui.ctx()).close();
                assert!(tools.show(ui, id, &mut host));
            });
            crate::accessibility::assert_named_controls(&nodes, id);
            assert!(!tools.read.running());
            assert!(!tools.action.running());
            assert!(tools.pending.is_none());
        }
        tools.shutdown();
        host.shutdown();
    }
}

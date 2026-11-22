//! Non-Windows application review. Package-manager entries are never executed.
use crate::{
    host::{self, Host},
    jobs::Task,
};
use eframe::egui;
use oxbox_core::tools::application_review::{self, Inventory};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Default)]
pub struct ApplicationReview {
    inventory: Task<Inventory>,
    remove: Task<usize>,
    selected: BTreeSet<PathBuf>,
    query: String,
}
impl ApplicationReview {
    pub fn tick(&mut self) {
        self.inventory.poll();
        let was_running = self.remove.running();
        self.remove.poll();
        if was_running && !self.remove.running() {
            self.selected.clear();
            self.inventory.result = None;
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, _host: &mut Host) {
        self.tick();
        ui.label("Review locally installed applications. Only eligible user-owned application bundles or portable files can be moved to Trash after an exact review. Saved data outside the selected app is left alone.");
        #[cfg(target_os="macos")]
        ui.label("macOS: immediate .app children of /Applications and ~/Applications. Only current-user-owned bundles with no protected flags and no Apple/system name are eligible. Root-owned and system apps need their normal OS/vendor uninstaller. Vendor code signatures are not verified here.");
        #[cfg(target_os="linux")]
        ui.label("Linux: desktop entries are a read-only inventory, not a complete package database. Use your OS software manager for packaged apps. Only current-user-owned regular .AppImage files directly in ~/Applications can be moved to Trash here.");
        #[cfg(not(any(target_os="macos",target_os="linux")))]
        ui.label("This application review is available on macOS and Linux. Windows uses its existing registered-uninstaller view.");
        ui.small("No package-manager command, installer, application launch, signature check, driver download or network request is performed. Inventories are bounded; permission and unavailable-root errors are shown.");
        let busy = self.inventory.running() || self.remove.running();
        if ui
            .add_enabled(
                cfg!(any(target_os = "macos", target_os = "linux")) && !busy,
                egui::Button::new("Scan local applications"),
            )
            .clicked()
        {
            self.selected.clear();
            self.inventory.start(ui.ctx(), move |cancel, progress| {
                application_review::scan(&cancel, progress)
            });
        }
        let label = ui.label("Filter application name or local path");
        ui.text_edit_singleline(&mut self.query)
            .labelled_by(label.id);
        self.inventory.show(ui);
        if let Some(inventory) = self.inventory.result.clone() {
            ui.strong(format!(
                "{} local application item(s)",
                inventory.applications.len()
            ));
            for warning in &inventory.warnings {
                ui.label(format!("Inventory limitation: {warning}"));
            }
            let query = self.query.to_lowercase();
            let mut visible = 0;
            for application in &inventory.applications {
                if !query.is_empty()
                    && !format!("{} {}", application.name, application.path.display())
                        .to_lowercase()
                        .contains(&query)
                {
                    continue;
                }
                visible += 1;
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    let mut checked = self.selected.contains(&application.path);
                    let response = ui.add_enabled(
                        !busy && application.can_trash,
                        egui::Checkbox::new(
                            &mut checked,
                            format!(
                                "Select {} · {}",
                                application.name,
                                application.path.display()
                            ),
                        ),
                    );
                    if response.changed() {
                        if checked {
                            self.selected.insert(application.path.clone());
                        } else {
                            self.selected.remove(&application.path);
                        }
                    }
                    ui.label(format!(
                        "{:?} · {}",
                        application.kind,
                        application.path.display()
                    ));
                    ui.label(&application.limitation);
                    if let Some(executable) = &application.executable {
                        ui.collapsing("Desktop Exec metadata · never executed", |ui| {
                            ui.monospace(executable);
                        });
                    }
                });
            }
            if visible == 0 {
                ui.label("No application items match this filter.");
            }
            ui.label(format!(
                "{} item(s) selected, including any hidden by the filter.",
                self.selected.len()
            ));
            if ui
                .add_enabled(
                    !busy && !self.selected.is_empty(),
                    egui::Button::new("Review selected applications for Trash…"),
                )
                .clicked()
            {
                let selected = self.selected.iter().cloned().collect::<Vec<_>>();
                let names = selected
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                let description = format!(
                    "Move exactly these {} reviewed application items to Trash?\n\n{names}\n\nThis is not a package-manager uninstall. Saved user data outside these app paths stays in place. Current ownership, links, identity and fixed allowed roots are rechecked. Applications in use may refuse removal. Cancel can stop later items; earlier items may already be in Trash.",
                    selected.len()
                );
                if host::dialogs::message(ui.ctx())
                    .set_title("Review local application removal")
                    .set_description(description)
                    .set_level(rfd::MessageLevel::Warning)
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show()
                    == rfd::MessageDialogResult::Yes
                {
                    let reviewed = inventory.applications.clone();
                    self.remove.start(ui.ctx(), move |cancel, progress| {
                        application_review::trash_selected(
                            &reviewed, &selected, true, &cancel, progress,
                        )
                    });
                }
            }
        } else if !busy {
            ui.label("No inventory loaded. Scan explicitly to inspect known local application locations.");
        }
        self.remove.show(ui);
        if let Some(count) = self.remove.result {
            crate::accessibility::status(
                ui,
                &format!(
                    "Moved {count} reviewed application item(s) to Trash. Scan again for a fresh inventory."
                ),
            );
        }
        if self.remove.error.is_some() {
            ui.label("Some earlier items may already be in Trash. Scan again before another removal attempt; OS Trash remains the recovery location.");
        }
    }
    pub fn leave_tool(&mut self) {
        self.inventory.abandon();
        self.remove.abandon();
        self.selected.clear();
    }
    pub fn shutdown(&mut self) {
        self.inventory.shutdown();
        self.remove.shutdown();
        self.selected.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_review_starts_empty_without_inventory_or_removal() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut view = ApplicationReview::default();
        let nodes = crate::accessibility::test_tree(|ui| view.show(ui, &mut host));
        crate::accessibility::assert_named_controls(&nodes, "non-Windows application review");
        assert!(view.inventory.result.is_none());
        assert!(!view.inventory.running());
        assert!(!view.remove.running());
        assert!(view.selected.is_empty());
        view.shutdown();
        host.shutdown();
    }
}

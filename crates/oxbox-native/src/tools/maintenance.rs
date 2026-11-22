//! Explicit maintenance reviews and separate confirmed mutations.
#[cfg(not(windows))]
mod applications;
mod browser;
mod files;
mod system;
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::tools::clean_drive;
#[derive(Default)]
pub struct MaintenanceTools {
    files: files::FileMaintenance,
    system: system::SystemMaintenance,
    browser: browser::BrowserTools,
    overview: Task<Vec<clean_drive::CleanCategory>>,
    navigation: Option<String>,
    #[cfg(not(windows))]
    applications: applications::ApplicationReview,
}
impl MaintenanceTools {
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, host: &mut Host) -> bool {
        #[cfg(not(windows))]
        if id == "uninstall-apps" {
            self.applications.show(ui, host);
            return true;
        }
        match id {
            "batch-rename" | "file-shred" => self.files.show(ui, id, host),
            "startup-manager" | "storage-analyzer" | "registry-review" => {
                self.system.show(ui, id, host)
            }
            "browser-review" => {
                self.browser.show(ui, host);
                true
            }
            "maintenance-review" => {
                self.review(ui);
                true
            }
            _ => false,
        }
    }
    fn review(&mut self, ui: &mut egui::Ui) {
        ui.label("Review storage and startup choices using local tools. Changes always have their own review and confirmation.");
        if ui
            .add_enabled(
                !self.overview.running(),
                egui::Button::new("Scan eligible cache and temporary files"),
            )
            .clicked()
        {
            self.overview.start(ui.ctx(), move |cancel, progress| {
                clean_drive::scan_current(&cancel, progress)
            });
        }
        self.overview.show(ui);
        if let Some(categories) = &self.overview.result {
            let bytes = categories
                .iter()
                .filter(|c| !c.read_only)
                .map(|c| c.bytes)
                .sum::<u64>();
            let files = categories
                .iter()
                .filter(|c| !c.read_only)
                .map(|c| c.files)
                .sum::<u64>();
            ui.strong(format!(
                "{files} eligible files · {:.1} MiB",
                bytes as f64 / (1024.0 * 1024.0)
            ));
            ui.weak("Estimates do not measure guaranteed reclaimed space or system speed. Review individual paths before cleanup.");
            for category in categories {
                ui.label(format!(
                    "{} · {} files · {:.1} MiB{}",
                    category.name,
                    category.files,
                    category.bytes as f64 / (1024.0 * 1024.0),
                    if category.limited {
                        " · partial scan"
                    } else {
                        ""
                    }
                ));
            }
        }
        ui.separator();
        for (target, label) in [
            ("clean-drive", "Review cleanup candidates"),
            ("storage-analyzer", "Analyze storage and large files"),
            ("find-duplicates", "Find duplicate files"),
            ("startup-manager", "Review current-user startup"),
            ("browser-review", "Review browser extensions"),
            ("uninstall-apps", "Review installed applications"),
            ("alarm", "Set a maintenance review reminder"),
        ] {
            if ui.button(label).clicked() {
                self.navigation = Some(target.into());
            }
        }
        #[cfg(windows)]
        if ui.button("Review Windows registry entries").clicked() {
            self.navigation = Some("registry-review".into());
        }
        ui.weak("Reminders use Alarm and require Oxbox to stay running. They do not automatically delete files or registry entries. OS cleanup rules and unavailable capabilities are shown in each tool.");
    }
    pub fn take_navigation(&mut self) -> Option<String> {
        self.navigation.take()
    }
    pub fn tick(&mut self) {
        self.files.tick();
        self.system.tick();
        self.browser.tick();
        self.overview.poll();
        #[cfg(not(windows))]
        self.applications.tick();
    }
    pub fn leave_tool(&mut self, id: &str) {
        self.files.leave_tool(id);
        self.system.leave_tool(id);
        if id == "browser-review" {
            self.browser.leave();
        }
        if id == "maintenance-review" {
            self.overview.abandon();
        }
        #[cfg(not(windows))]
        if id == "uninstall-apps" {
            self.applications.leave_tool();
        }
        self.navigation = None;
    }
    pub fn deactivate(&mut self) {
        self.files.deactivate();
    }
    pub fn shutdown(&mut self) {
        self.files.shutdown();
        self.system.shutdown();
        self.browser.shutdown();
        self.overview.shutdown();
        #[cfg(not(windows))]
        self.applications.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintenance_default_forms_are_named_and_do_not_start_effects() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().into()).unwrap();
        let mut tools = MaintenanceTools::default();
        for id in [
            "batch-rename",
            "file-shred",
            "startup-manager",
            "storage-analyzer",
            "registry-review",
            "browser-review",
            "maintenance-review",
        ] {
            let nodes = crate::accessibility::test_tree(|ui| {
                assert!(tools.show(ui, id, &mut host));
            });
            crate::accessibility::assert_named_controls(&nodes, id);
            tools.leave_tool(id);
        }
        tools.shutdown();
        host.shutdown();
    }
}

use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::tools::browser_review::{self, Browser, BrowserReport, BrowserRoot};

#[derive(Default)]
pub(super) struct BrowserTools {
    custom: String,
    browser: usize,
    task: Task<BrowserReport>,
    error: Option<String>,
}
impl BrowserTools {
    fn admit_folder(&mut self, path: &std::path::Path) {
        self.task.abandon();
        match crate::host::path_text(path) {
            Ok(value) => {
                self.custom = value;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
    fn requested_roots(&self) -> Vec<BrowserRoot> {
        if self.custom.trim().is_empty() {
            browser_review::known_roots()
        } else {
            vec![BrowserRoot {
                browser: [
                    Browser::Chromium,
                    Browser::Edge,
                    Browser::Brave,
                    Browser::Firefox,
                ][self.browser],
                path: self.custom.clone().into(),
            }]
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, _host: &mut Host) {
        ui.label("Review local browser extension metadata. Nothing is enabled, disabled, removed or uploaded.");
        ui.weak("History, cookies, saved passwords and account databases are not read. Extension enabled state is shown only when Firefox metadata supplies it; check other browsers directly.");
        let before = (self.custom.clone(), self.browser);
        ui.add_enabled_ui(!self.task.running(),|ui|{
            let label=ui.label("Browser user-data folder (optional)");ui.text_edit_singleline(&mut self.custom).labelled_by(label.id);
            if ui.button("Choose browser user-data folder…").clicked()&&let Some(path)=crate::host::dialogs::file(ui.ctx()).pick_folder(){self.admit_folder(&path);}
            egui::ComboBox::from_label("Browser for selected folder").selected_text(["Chrome / Chromium","Edge","Brave","Firefox Profiles"][self.browser]).show_ui(ui,|ui|{for(index,name)in["Chrome / Chromium","Edge","Brave","Firefox Profiles"].into_iter().enumerate(){ui.selectable_value(&mut self.browser,index,name);}});
            ui.weak("Leave the folder blank to review installed browsers in their known per-OS profile locations. For a custom Chromium folder, select the parent of Default / Profile N; Firefox uses its Profiles directory.");
        });
        if before != (self.custom.clone(), self.browser) {
            self.task.abandon();
            if before.0 != self.custom {
                match crate::host::path_text(std::path::Path::new(&self.custom)) {
                    Ok(value) => {
                        self.custom = value;
                        self.error = None;
                    }
                    Err(error) => {
                        self.custom = before.0;
                        self.error = Some(error.to_string());
                    }
                }
            }
        }
        if let Some(error) = &self.error {
            crate::accessibility::alert(ui, error);
        }
        if ui
            .add_enabled(
                !self.task.running(),
                egui::Button::new("Review browser extensions"),
            )
            .clicked()
        {
            let roots = self.requested_roots();
            self.task.start(ui.ctx(), move |cancel, progress| {
                browser_review::scan_roots(&roots, &cancel, progress)
            });
        }
        self.task.show(ui);
        if let Some(report) = &self.task.result {
            ui.label(format!(
                "{} profiles · {} metadata files reviewed · {} skipped",
                report.profiles.len(),
                report.metadata_files,
                report.skipped
            ));
            if report.limited {
                ui.label("Review limits reached. This is a partial list.");
            }
            if report.profiles.is_empty() {
                ui.label("No supported browser profiles were found. You can select a local profile root above.");
            }
            for profile in &report.profiles {
                ui.push_id(&profile.path, |ui| {
                    ui.collapsing(
                        format!(
                            "{} · {} extension versions",
                            profile.browser.label(),
                            profile.extensions.len()
                        ),
                        |ui| {
                            ui.weak(profile.path.display().to_string());
                            ui.label(format!(
                                "Manage extensions in your browser at {}",
                                profile.browser.settings()
                            ));
                            if ui.button("Copy extensions settings address").clicked() {
                                ui.ctx().copy_text(profile.browser.settings().into());
                            }
                            if profile.extensions.is_empty() {
                                ui.label("No readable extension metadata in this profile.");
                            }
                            for (index, extension) in profile.extensions.iter().enumerate() {
                                ui.push_id(index, |ui| {
                                    ui.separator();
                                    ui.strong(&extension.name);
                                    ui.label(format!(
                                        "{} · {} · {}",
                                        extension.id,
                                        extension.version,
                                        match extension.enabled {
                                            Some(true) => "enabled",
                                            Some(false) => "disabled",
                                            None => "check enabled state in browser",
                                        }
                                    ));
                                    if !extension.permissions.is_empty() {
                                        ui.label(format!(
                                            "Declared permissions: {}",
                                            extension.permissions.join(", ")
                                        ));
                                    }
                                    let copy = ui.button("Copy extension ID");
                                    crate::accessibility::name(
                                        &copy,
                                        &format!(
                                            "Copy extension ID {} {}",
                                            extension.name, extension.version
                                        ),
                                    );
                                    if copy.clicked() {
                                        ui.ctx().copy_text(extension.id.clone());
                                    }
                                });
                            }
                        },
                    );
                });
            }
        }
    }
    pub fn tick(&mut self) {
        self.task.poll();
    }
    pub fn leave(&mut self) {
        self.task.abandon();
    }
    pub fn shutdown(&mut self) {
        self.task.shutdown();
    }
}

#[cfg(test)]
mod verification56_path_tests {
    use super::*;
    #[test]
    fn verification56_browser_request_preserves_selected_folder_edge_spaces() {
        let view = BrowserTools {
            custom: "/generated browser folder ".into(),
            ..Default::default()
        };
        let roots = view.requested_roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(
            roots[0].path,
            std::path::PathBuf::from("/generated browser folder ")
        );
    }
    #[test]
    fn verification56_browser_refused_folder_discards_queued_report_and_manual_edit_clears_error() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut view = BrowserTools {
            custom: "Generated prior browser folder".into(),
            ..Default::default()
        };
        view.task.result = Some(BrowserReport::default());
        view.admit_folder(std::path::Path::new("generated\nrefused folder"));
        assert_eq!(view.custom, "Generated prior browser folder");
        assert!(
            view.task.result.is_none(),
            "refused folder kept an unrelated queued report"
        );
        assert!(view.error.is_some());
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut first = context.run_ui(egui::RawInput::default(), |ui| view.show(ui, &mut host));
        first.textures_delta.clear();
        let nodes = first.platform_output.accesskit_update.unwrap().nodes;
        assert!(
            nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::Alert)
        );
        let field = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::TextInput
                    && node.value() == Some("Generated prior browser folder"))
                .then_some(id)
            })
            .unwrap();
        let mut edited = context.run_ui(
            egui::RawInput {
                events: vec![egui::Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::SetValue,
                        target_node: field,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        data: Some(egui::accesskit::ActionData::Value(
                            "Generated new browser folder".into(),
                        )),
                    },
                )],
                ..Default::default()
            },
            |ui| view.show(ui, &mut host),
        );
        edited.textures_delta.clear();
        assert_eq!(view.custom, "Generated new browser folder");
        assert!(view.error.is_none());
        view.shutdown();
        host.shutdown();
    }
}

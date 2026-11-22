use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::tools::browser_review::{self, Browser, BrowserReport, BrowserRoot};

#[derive(Default)]
pub(super) struct BrowserTools {
    custom: String,
    browser: usize,
    task: Task<BrowserReport>,
}
impl BrowserTools {
    pub fn show(&mut self, ui: &mut egui::Ui, _host: &mut Host) {
        ui.label("Review local browser extension metadata. Nothing is enabled, disabled, removed or uploaded.");
        ui.weak("History, cookies, saved passwords and account databases are not read. Extension enabled state is shown only when Firefox metadata supplies it; check other browsers directly.");
        let before = (self.custom.clone(), self.browser);
        ui.add_enabled_ui(!self.task.running(),|ui|{
            let label=ui.label("Browser user-data folder (optional)");ui.text_edit_singleline(&mut self.custom).labelled_by(label.id);
            if ui.button("Choose browser user-data folder…").clicked()&&let Some(path)=crate::host::dialogs::file(ui.ctx()).pick_folder(){self.custom=path.to_string_lossy().into();}
            egui::ComboBox::from_label("Browser for selected folder").selected_text(["Chrome / Chromium","Edge","Brave","Firefox Profiles"][self.browser]).show_ui(ui,|ui|{for(index,name)in["Chrome / Chromium","Edge","Brave","Firefox Profiles"].into_iter().enumerate(){ui.selectable_value(&mut self.browser,index,name);}});
            ui.weak("Leave the folder blank to review installed browsers in their known per-OS profile locations. For a custom Chromium folder, select the parent of Default / Profile N; Firefox uses its Profiles directory.");
        });
        if before != (self.custom.clone(), self.browser) {
            self.task.abandon();
        }
        if ui
            .add_enabled(
                !self.task.running(),
                egui::Button::new("Review browser extensions"),
            )
            .clicked()
        {
            let roots = if self.custom.trim().is_empty() {
                browser_review::known_roots()
            } else {
                vec![BrowserRoot {
                    browser: [
                        Browser::Chromium,
                        Browser::Edge,
                        Browser::Brave,
                        Browser::Firefox,
                    ][self.browser],
                    path: self.custom.trim().into(),
                }]
            };
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

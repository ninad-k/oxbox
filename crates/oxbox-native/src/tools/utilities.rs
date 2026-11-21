//! Native forms for offline text, file and time utilities.
mod currency;
mod files;
mod text;
mod time;
use crate::host::Host;
use eframe::egui::Ui;

#[derive(Default)]
pub struct UtilitiesTools {
    text: text::TextTools,
    files: files::FileTools,
    time: time::TimeTools,
    currency: currency::CurrencyTools,
}
impl UtilitiesTools {
    pub fn show(&mut self, ui: &mut Ui, id: &str, host: &mut Host) -> bool {
        match id {
            "transform-text" => self.text.transform(ui),
            "unit-converter" => {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.text.currency_mode, false, "Measurement units");
                    ui.selectable_value(
                        &mut self.text.currency_mode,
                        true,
                        "Currency reference rates",
                    );
                });
                if self.text.currency_mode {
                    self.currency.show(ui, host)
                } else {
                    self.text.units(ui)
                }
            }
            "currency-converter" => self.currency.show(ui, host),
            "verify-checksum" | "find-duplicates" | "archive" | "clean-drive" | "encrypt-files" => {
                self.files.show(ui, id)
            }
            "break-time" | "timer-stopwatch" | "world-time" | "alarm" | "date-countdown" => {
                self.time.show(ui, id, host)
            }
            _ => return false,
        }
        true
    }
    pub fn tick(&mut self, host: &mut Host) {
        self.files.poll();
        self.currency.tick(host);
        self.time.tick(host);
    }
    pub fn banners(&mut self, ui: &mut Ui, host: &mut Host) {
        self.time.banners(ui, host);
    }
    /// Clear transient credential drafts when navigation or hiding ends editing.
    pub fn deactivate(&mut self) {
        self.files.clear_secrets();
    }
    pub fn leave_tool(&mut self, id: &str) {
        self.files.leave_tool(id);
    }
    /// A synchronous tray Quit may signal reminders while a UI dialog is open.
    pub fn shutdown_request(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.time.shutdown_request()
    }
    pub fn shutdown(&mut self) {
        self.time.shutdown();
        self.files.shutdown();
        self.currency.shutdown();
    }
}
pub(super) fn field(ui: &mut Ui, label: &str, value: &mut String) {
    let label = ui.label(label);
    ui.text_edit_singleline(value).labelled_by(label.id);
}
pub(super) fn bytes(value: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut amount = value as f64;
    let mut index = 0;
    while amount >= 1024.0 && index + 1 < units.len() {
        amount /= 1024.0;
        index += 1;
    }
    format!("{amount:.1} {}", units[index])
}
pub(super) fn issue(ui: &mut Ui, error: impl std::fmt::Display) {
    ui.label(format!("Error: {error}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_utility_forms_render_without_effects_and_legacy_import_stays_offline() {
        let directory = tempfile::tempdir().unwrap();
        let settings = r#"{"preferences":{"worldZones":["UTC"]},"roadmapTime":{"alarms":[],"countdowns":[]},"breakTime":{"minutes":20,"eyeRest":false,"pauseFullscreen":true,"enabled":false}}"#;
        let currency = r#"{"currencyNetwork":true,"currencyRates":{"date":"2026-11-20","fetchedAt":1,"rates":{"EUR":1.0,"USD":2.0}}}"#;
        std::fs::write(directory.path().join("settings.json"), settings).unwrap();
        std::fs::write(directory.path().join("currency.json"), currency).unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut tools = UtilitiesTools::default();
        let context = eframe::egui::Context::default();
        for id in [
            "transform-text",
            "unit-converter",
            "currency-converter",
            "break-time",
            "timer-stopwatch",
            "world-time",
            "alarm",
            "date-countdown",
            "verify-checksum",
            "find-duplicates",
            "archive",
            "clean-drive",
            "encrypt-files",
        ] {
            let mut output = context.run_ui(eframe::egui::RawInput::default(), |ui| {
                assert!(tools.show(ui, id, &mut host), "missing native form: {id}");
            });
            // Headless rendering has no GPU painter to consume font-atlas updates.
            output.textures_delta.clear();
        }
        let mut output = context.run_ui(eframe::egui::RawInput::default(), |ui| {
            assert!(!tools.show(ui, "unknown-tool", &mut host))
        });
        output.textures_delta.clear();
        tools.shutdown();
        host.shutdown();
        assert_eq!(
            std::fs::read_to_string(directory.path().join("settings.json")).unwrap(),
            settings
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("currency.json")).unwrap(),
            currency
        );
        let imported: serde_json::Value = host.load("native-currency.json").unwrap().unwrap();
        assert_eq!(imported["network_enabled"], false);
        assert_eq!(imported["book"]["rates"]["USD"], 2.0);
        let imported: serde_json::Value = host.load("native-utilities.json").unwrap().unwrap();
        assert_eq!(imported["worldZones"][0], "UTC");
    }
}

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
    pub fn export_preset(
        &self,
        id: &str,
    ) -> oxbox_core::CoreResult<Option<oxbox_core::presets::PresetConfig>> {
        use oxbox_core::presets::PresetConfig as P;
        Ok(match id {
            "verify-checksum" => Some(P::VerifyChecksum(self.files.checksum_preset())),
            "timer-stopwatch" => Some(P::TimerStopwatch(self.time.timer_preset()?)),
            "unit-converter" if !self.text.currency_mode => {
                Some(P::UnitConverter(self.text.export_preset()))
            }
            _ => None,
        })
    }
    pub fn import_preset(
        &mut self,
        p: &oxbox_core::presets::PresetConfig,
    ) -> oxbox_core::CoreResult<()> {
        use oxbox_core::presets::PresetConfig as P;
        p.validate()?;
        match p {
            P::VerifyChecksum(p) => self.files.apply_checksum_preset(p),
            P::TimerStopwatch(p) => self.time.apply_timer_preset(p),
            P::UnitConverter(p) => {
                self.currency.leave();
                self.text.import_preset(p)
            }
            _ => Err(oxbox_core::CoreError::InvalidInput(
                "This preset belongs to another tool.".into(),
            )),
        }
    }
    pub fn configure_quick_action(
        &mut self,
        action: &oxbox_core::quick_actions::QuickAction,
    ) -> oxbox_core::CoreResult<()> {
        use oxbox_core::quick_actions::QuickAction as A;
        match action {
            A::Timer { seconds } => self
                .time
                .apply_timer_preset(&oxbox_core::presets::TimerPreset { seconds: *seconds }),
            A::Convert {
                value, from, to, ..
            } => {
                self.currency.leave();
                self.text.quick_conversion(*value, from, to)
            }
            _ => Ok(()),
        }
    }
    pub fn show(&mut self, ui: &mut Ui, id: &str, host: &mut Host) -> bool {
        match id {
            "transform-text" => self.text.transform(ui),
            "unit-converter" => {
                let was_currency = self.text.currency_mode;
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.text.currency_mode, false, "Measurement units");
                    ui.selectable_value(
                        &mut self.text.currency_mode,
                        true,
                        "Currency reference rates",
                    );
                });
                if was_currency && !self.text.currency_mode {
                    self.currency.leave();
                }
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
        if matches!(id, "currency-converter" | "unit-converter") {
            self.currency.leave();
        }
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
    crate::accessibility::alert(ui, error);
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
        context.enable_accesskit();
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
            let nodes = output
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .into_iter()
                .map(|(_, node)| node)
                .collect::<Vec<_>>();
            crate::accessibility::assert_named_controls(&nodes, id);
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

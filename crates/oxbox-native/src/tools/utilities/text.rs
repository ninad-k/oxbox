use super::{field, issue};
use eframe::egui::{self, Ui};
use oxbox_core::tools::{
    text_transform::{self, TextCase},
    unit_convert,
};
pub(super) struct TextTools {
    input: String,
    output: String,
    case: TextCase,
    value: String,
    category: String,
    from: String,
    to: String,
    pub currency_mode: bool,
}

impl Default for TextTools {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            case: TextCase::Upper,
            value: "1".into(),
            category: "length".into(),
            from: "m".into(),
            to: "km".into(),
            currency_mode: false,
        }
    }
}
impl TextTools {
    pub fn export_preset(&self) -> oxbox_core::presets::UnitPreset {
        oxbox_core::presets::UnitPreset {
            from: self.from.clone(),
            to: self.to.clone(),
        }
    }
    pub fn import_preset(
        &mut self,
        p: &oxbox_core::presets::UnitPreset,
    ) -> oxbox_core::CoreResult<()> {
        oxbox_core::tools::unit_convert::convert(1.0, &p.from, &p.to)?;
        let Some(unit) = unit_convert::units().iter().find(|u| u.id == p.from) else {
            return Err(oxbox_core::CoreError::InvalidInput(
                "Unknown unit preset.".into(),
            ));
        };
        self.category = unit.category.into();
        self.from = p.from.clone();
        self.to = p.to.clone();
        self.currency_mode = false;
        Ok(())
    }
    pub fn quick_conversion(
        &mut self,
        value: f64,
        from: &str,
        to: &str,
    ) -> oxbox_core::CoreResult<()> {
        unit_convert::convert(value, from, to)?;
        self.import_preset(&oxbox_core::presets::UnitPreset {
            from: from.into(),
            to: to.into(),
        })?;
        self.value = value.to_string();
        Ok(())
    }
    pub fn transform(&mut self, ui: &mut Ui) {
        let input_label = ui.label("Input text");
        let changed = ui
            .add(
                egui::TextEdit::multiline(&mut self.input)
                    .desired_rows(7)
                    .desired_width(f32::INFINITY)
                    .char_limit(1_000_000),
            )
            .labelled_by(input_label.id)
            .changed();
        let previous = self.case;
        egui::ComboBox::from_label("Text case")
            .selected_text(format!("{:?}", self.case))
            .show_ui(ui, |ui| {
                for (case, label) in [
                    (TextCase::Upper, "UPPER CASE"),
                    (TextCase::Lower, "lower case"),
                    (TextCase::Title, "Title Case"),
                    (TextCase::Sentence, "Sentence case"),
                    (TextCase::Camel, "camelCase"),
                    (TextCase::Pascal, "PascalCase"),
                    (TextCase::Snake, "snake_case"),
                    (TextCase::Kebab, "kebab-case"),
                    (TextCase::Constant, "CONSTANT_CASE"),
                ] {
                    ui.selectable_value(&mut self.case, case, label);
                }
            });
        if changed || previous != self.case {
            self.output = text_transform::transform(&self.input, self.case);
        }
        let output_label = ui.label("Result");
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_rows(7)
                .desired_width(f32::INFINITY)
                .interactive(false),
        )
        .labelled_by(output_label.id);
        if ui
            .add_enabled(!self.output.is_empty(), egui::Button::new("Copy result"))
            .clicked()
        {
            ui.ctx().copy_text(self.output.clone());
        }
        if self.input.is_empty() {
            ui.label("Enter text to transform it instantly.");
        }
    }
    pub fn units(&mut self, ui: &mut Ui) {
        let all = unit_convert::units();
        let categories = all
            .iter()
            .map(|unit| unit.category)
            .collect::<std::collections::BTreeSet<_>>();
        let old = self.category.clone();
        egui::ComboBox::from_label("Category")
            .selected_text(&self.category)
            .show_ui(ui, |ui| {
                for category in categories {
                    ui.selectable_value(&mut self.category, category.to_string(), category);
                }
            });
        let choices = all
            .iter()
            .filter(|unit| unit.category == self.category)
            .collect::<Vec<_>>();
        if self.category != old {
            self.from = choices.first().map_or("", |unit| unit.id).into();
            self.to = choices
                .get(1)
                .or(choices.first())
                .map_or("", |unit| unit.id)
                .into();
        }
        field(ui, "Value", &mut self.value);
        egui::ComboBox::from_label("From")
            .selected_text(
                choices
                    .iter()
                    .find(|unit| unit.id == self.from)
                    .map_or(self.from.as_str(), |unit| unit.label),
            )
            .show_ui(ui, |ui| {
                for unit in &choices {
                    ui.selectable_value(&mut self.from, unit.id.into(), unit.label);
                }
            });
        if ui.button("Swap units").clicked() {
            std::mem::swap(&mut self.from, &mut self.to);
        }
        egui::ComboBox::from_label("To")
            .selected_text(
                choices
                    .iter()
                    .find(|unit| unit.id == self.to)
                    .map_or(self.to.as_str(), |unit| unit.label),
            )
            .show_ui(ui, |ui| {
                for unit in &choices {
                    ui.selectable_value(&mut self.to, unit.id.into(), unit.label);
                }
            });
        if self.value.trim().is_empty() {
            ui.label("Enter a value to convert.");
            return;
        }
        match self
            .value
            .trim()
            .parse::<f64>()
            .map_err(|_| "Enter a finite number.".to_string())
            .and_then(|value| {
                unit_convert::convert(value, &self.from, &self.to)
                    .map_err(|error| error.to_string())
            }) {
            Ok(value) => {
                let output = value.to_string();
                ui.monospace(&output);
                if ui.button("Copy result").clicked() {
                    ui.ctx().copy_text(output);
                }
            }
            Err(error) => issue(ui, error),
        }
    }
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;
    use egui::accesskit::{Live, Role};

    #[test]
    fn invalid_conversion_is_an_announced_named_error() {
        let mut tools = TextTools {
            value: "not a number".into(),
            ..Default::default()
        };
        let nodes = crate::accessibility::test_tree(|ui| tools.units(ui));
        assert!(
            nodes.iter().any(|node| {
                node.role() == Role::Alert
                    && node.live() == Some(Live::Assertive)
                    && node
                        .label()
                        .is_some_and(|label| label.contains("finite number"))
            }),
            "conversion errors must be exposed as named live alerts"
        );
    }
}

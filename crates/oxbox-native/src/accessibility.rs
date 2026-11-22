//! Shared native accessibility semantics and headless tree checks.
use eframe::egui::{self, Color32};

pub(crate) fn control_border(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(99, 113, 135)
    } else {
        Color32::from_rgb(132, 143, 160)
    }
}

/// Preserve the widget's value/actions while supplying its explicit accessible name.
pub(crate) fn name(response: &egui::Response, name: &str) {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_label(name));
}

pub(crate) fn button(ui: &mut egui::Ui, visible: &str, name: &str) -> egui::Response {
    enabled_button(ui, true, visible, name)
}

pub(crate) fn enabled_button(
    ui: &mut egui::Ui,
    enabled: bool,
    visible: &str,
    name: &str,
) -> egui::Response {
    let response = ui.add_enabled(enabled, egui::Button::new(visible));
    self::name(&response, name);
    response
}

pub(crate) fn alert(ui: &mut egui::Ui, message: impl std::fmt::Display) -> egui::Response {
    let message = format!("Error: {message}");
    let response = ui.colored_label(ui.visuals().error_fg_color, &message);
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Alert);
        node.set_label(message);
        node.set_live(egui::accesskit::Live::Assertive);
        node.set_live_atomic();
    });
    response
}

pub(crate) fn status(ui: &mut egui::Ui, message: &str) -> egui::Response {
    let response = ui.label(message);
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::Status);
        node.set_label(message);
        node.set_live(egui::accesskit::Live::Polite);
        node.set_live_atomic();
    });
    response
}

pub(crate) fn search(ui: &mut egui::Ui, query: &mut String) -> egui::Response {
    let response = ui.add(
        egui::TextEdit::singleline(query)
            .id(egui::Id::unique("search-tools"))
            .hint_text("Find a tool…  ·  Ctrl/Cmd K")
            .desired_width(f32::INFINITY),
    );
    name(&response, "Find a tool");
    response
}

#[cfg(test)]
pub(crate) fn test_tree(
    mut render: impl FnMut(&mut eframe::egui::Ui),
) -> Vec<eframe::egui::accesskit::Node> {
    let context = eframe::egui::Context::default();
    context.enable_accesskit();
    let mut output = context.run_ui(eframe::egui::RawInput::default(), |ui| render(ui));
    output.textures_delta.clear();
    output
        .platform_output
        .accesskit_update
        .expect("AccessKit tree must be generated")
        .nodes
        .into_iter()
        .map(|(_, node)| node)
        .collect()
}

#[cfg(test)]
pub(crate) fn assert_named_controls(nodes: &[egui::accesskit::Node], scope: &str) {
    use egui::accesskit::Role;
    let unnamed = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.role(),
                Role::TextInput
                    | Role::MultilineTextInput
                    | Role::PasswordInput
                    | Role::SearchInput
                    | Role::ComboBox
                    | Role::EditableComboBox
                    | Role::SpinButton
                    | Role::Slider
                    | Role::CheckBox
                    | Role::Button
                    | Role::RadioButton
            ) && node.label().is_none_or(|label| label.trim().is_empty())
                && node.labelled_by().is_empty()
        })
        .map(|node| node.role())
        .collect::<Vec<_>>();
    assert!(
        unnamed.is_empty(),
        "unnamed interactive controls in {scope}: {unnamed:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color32) -> f64 {
        let linear = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
    }

    #[test]
    fn control_boundaries_have_three_to_one_contrast_in_both_themes() {
        for (dark, fill) in [
            (true, Color32::from_rgb(24, 33, 45)),
            (false, Color32::WHITE),
        ] {
            let border = luminance(control_border(dark));
            let fill = luminance(fill);
            let ratio = (border.max(fill) + 0.05) / (border.min(fill) + 0.05);
            assert!(
                ratio >= 3.0,
                "control border contrast is {ratio:.2}:1 in dark={dark}"
            );
        }
    }

    #[test]
    fn named_launcher_search_preserves_its_editable_value() {
        let mut query = "checksum".to_owned();
        let nodes = super::test_tree(|ui| {
            search(ui, &mut query);
        });
        assert!(nodes.iter().any(|node| node.label() == Some("Find a tool") && node.value() == Some("checksum")));
        assert_eq!(query, "checksum");
    }
}

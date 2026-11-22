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

fn expansion_request(response: &egui::Response) -> Option<bool> {
    use egui::accesskit::Action;
    let mut requested = None;
    response.ctx.input_mut(|input| {
        input.consume_accesskit_action_requests(response.id, |request| {
            match request.action {
                Action::Expand => requested = Some(true),
                Action::Collapse => requested = Some(false),
                _ => return false,
            }
            true
        });
    });
    requested.filter(|_| response.enabled())
}

fn expansion_state(response: &egui::Response, expanded: bool) {
    use egui::accesskit::Action;
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_expanded(expanded);
        node.remove_action(Action::Expand);
        node.remove_action(Action::Collapse);
        if response.enabled() {
            node.add_action(if expanded {
                Action::Collapse
            } else {
                Action::Expand
            });
        }
    });
}

/// Preserve combo value/actions and report actual popup expansion state.
pub(crate) fn combo_expansion(response: &egui::Response) {
    if let Some(requested) = expansion_request(response) {
        let popup = response.id.with("popup");
        if requested {
            egui::Popup::open_id(&response.ctx, popup);
        } else {
            egui::Popup::close_id(&response.ctx, popup);
        }
        // The popup body is drawn by the real combo on the next pass.
        response.ctx.request_repaint();
    }
    let expanded = egui::ComboBox::is_open(&response.ctx, response.id);
    if response.has_focus() {
        // Popup still receives Escape, but egui's next focus pass must not
        // discard its focused anchor before dismissing the chooser. Never
        // request focus here: pointer dismissal and another focused widget
        // keep their ordinary behavior.
        response.ctx.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    escape: expanded,
                    ..Default::default()
                },
            );
        });
    }
    expansion_state(response, expanded);
}

/// Handle assistive-technology actions on a real CollapsingHeader's persisted
/// state. Apply only to its header response, never an arbitrary button.
pub(crate) fn disclosure_expansion(response: &egui::Response) {
    let Some(mut state) =
        egui::collapsing_header::CollapsingState::load(&response.ctx, response.id)
    else {
        return;
    };
    if let Some(requested) = expansion_request(response) {
        state.set_open(requested);
        state.store(&response.ctx);
        response.ctx.request_repaint();
    }
    expansion_state(response, state.is_open());
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
    let message = message.to_string();
    if message == "Cancelled" {
        return status(ui, "Cancelled");
    }
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
mod cancellation_tests {
    #[test]
    fn requested_cancellation_is_a_neutral_polite_status_not_an_error_alert() {
        let nodes = super::test_tree(|ui| {
            super::alert(ui, oxbox_core::CoreError::Cancelled);
        });
        assert!(nodes.iter().any(|node| node.label() == Some("Cancelled")
            && node.role() == eframe::egui::accesskit::Role::Status
            && node.live() == Some(eframe::egui::accesskit::Live::Polite)));
        assert!(
            !nodes
                .iter()
                .any(|node| node.role() == eframe::egui::accesskit::Role::Alert)
        );
    }
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
    let missing_expansion = nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox && node.is_expanded().is_none())
        .map(|node| node.label().unwrap_or("labelled combo box"))
        .collect::<Vec<_>>();
    assert!(
        missing_expansion.is_empty(),
        "combo boxes lack expanded state in {scope}: {missing_expansion:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard_combo_frame(
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> (egui::Id, egui::Id, egui::Rect, egui::Id) {
        let mut ids = None;
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                let mut text = "Generated preceding text field".to_owned();
                let before = ui.text_edit_singleline(&mut text);
                let chooser = egui::ComboBox::from_label("Generated keyboard chooser")
                    .selected_text("First")
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "First");
                        let _ = ui.selectable_label(false, "Second");
                    });
                combo_expansion(&chooser.response);
                let after = ui.button("Generated following control");
                ids = Some((chooser.response.id, before.id, before.rect, after.id));
            },
        );
        output.textures_delta.clear();
        ids.unwrap()
    }

    fn keypress(key: egui::Key) -> Vec<egui::Event> {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]
    }

    #[test]
    fn keyboard_escape_closes_combo_and_preserves_its_focus_for_next_tab() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let frame = |events| keyboard_combo_frame(&context, events);
        let (chooser, _, _, following) = frame(vec![]);
        context.memory_mut(|memory| memory.request_focus(chooser));
        frame(vec![]);
        assert_eq!(context.memory(|memory| memory.focused()), Some(chooser));
        frame(keypress(egui::Key::Enter));
        assert!(
            egui::ComboBox::is_open(&context, chooser),
            "Enter must open the real chooser"
        );
        frame(keypress(egui::Key::Escape));
        assert!(
            !egui::ComboBox::is_open(&context, chooser),
            "Escape must close the real chooser"
        );
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(chooser),
            "Escape must return keyboard focus to the chooser instead of the window"
        );
        frame(keypress(egui::Key::Tab));
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(following),
            "Tab after Escape must continue after the chooser, not restart the page"
        );
    }

    #[test]
    fn combo_dismissal_does_not_steal_focus_from_another_field_or_pointer_click() {
        for pointer in [false, true] {
            let context = egui::Context::default();
            let frame = |events| keyboard_combo_frame(&context, events);
            let (chooser, other, other_rect, _) = frame(vec![]);
            context.memory_mut(|memory| memory.request_focus(chooser));
            frame(vec![]);
            frame(keypress(egui::Key::Enter));
            assert!(egui::ComboBox::is_open(&context, chooser));
            if pointer {
                let position = other_rect.center();
                frame(vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                frame(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }]);
                assert_eq!(
                    context.memory(|memory| memory.focused()),
                    Some(other),
                    "pointer dismissal must keep focus on the clicked text field"
                );
            } else {
                // Another widget can gain focus later in the same frame, after
                // the combo helper ran. A stale anchor snapshot must not steal it.
                context.memory_mut(|memory| memory.request_focus(other));
                frame(keypress(egui::Key::Escape));
                assert_eq!(
                    context.memory(|memory| memory.focused()),
                    None,
                    "Escape from another field must keep normal focus behavior"
                );
            }
            assert!(!egui::ComboBox::is_open(&context, chooser));
            assert_ne!(context.memory(|memory| memory.focused()), Some(chooser));
        }
    }

    #[test]
    fn combo_expansion_actions_operate_the_popup_and_respect_disabled_state() {
        use egui::accesskit::{Action, ActionRequest, Role, TreeId};
        for enabled in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let frame = |events| {
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        ui.add_enabled_ui(enabled, |ui| {
                            let chooser = egui::ComboBox::from_label("Generated action chooser")
                                .selected_text("First")
                                .show_ui(ui, |ui| {
                                    let _ = ui.selectable_label(true, "First");
                                    let _ = ui.selectable_label(false, "Second");
                                });
                            combo_expansion(&chooser.response);
                        });
                    },
                );
                output.textures_delta.clear();
                output.platform_output.accesskit_update.unwrap().nodes
            };
            let nodes = frame(Vec::new());
            let (chooser, advertises_expand) = nodes
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == Role::ComboBox)
                        .then_some((id, node.supports_action(Action::Expand)))
                })
                .unwrap();
            let request = |action| {
                egui::Event::AccessKitActionRequest(ActionRequest {
                    action,
                    target_node: chooser,
                    target_tree: TreeId::ROOT,
                    data: None,
                })
            };
            let nodes = frame(vec![request(Action::Expand)]);
            assert_eq!(
                egui::Popup::is_any_open(&context),
                enabled,
                "Expand must operate only an enabled chooser"
            );
            let advertises_collapse = nodes
                .into_iter()
                .find_map(|(id, node)| {
                    (id == chooser).then_some(node.supports_action(Action::Collapse))
                })
                .unwrap();
            frame(vec![request(Action::Expand)]);
            assert_eq!(
                egui::Popup::is_any_open(&context),
                enabled,
                "Expand must be idempotent rather than toggle closed"
            );
            frame(vec![request(Action::Collapse)]);
            assert!(
                !egui::Popup::is_any_open(&context),
                "Collapse must close the chooser"
            );
            assert_eq!(advertises_expand, enabled);
            assert_eq!(advertises_collapse, enabled);
            if enabled {
                frame(vec![request(Action::Click)]);
                assert!(
                    egui::Popup::is_any_open(&context),
                    "normal Click must keep working"
                );
                frame(vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]);
                assert!(
                    !egui::Popup::is_any_open(&context),
                    "keyboard Escape must keep working"
                );
            }
        }
    }

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

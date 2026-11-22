//! Explicit, validated safe-option presets; file contents and active services are excluded.
use crate::host::Host;
use eframe::egui;
use oxbox_core::{
    CoreResult,
    presets::{PresetBook, PresetConfig},
};

pub struct PresetUi {
    book: PresetBook,
    available: bool,
    tool: String,
    name: String,
    selected: Option<usize>,
    pub error: String,
    notice: String,
}
impl PresetUi {
    pub fn applied(&mut self) {
        self.error.clear();
        self.notice = "Selected options applied; no work was started.".into();
    }
    pub fn load(host: &Host) -> Self {
        let loaded = host
            .load::<PresetBook>("native-presets.json")
            .and_then(|book| {
                let book = book.unwrap_or_default();
                book.validate()?;
                Ok(book)
            });
        let (book, error, available) = match loaded {
            Ok(book) => (book, String::new(), true),
            Err(error) => (PresetBook::default(), error.to_string(), false),
        };
        Self {
            book,
            available,
            tool: String::new(),
            name: String::new(),
            selected: None,
            error,
            notice: String::new(),
        }
    }
    fn save(
        &mut self,
        context: &egui::Context,
        host: &Host,
        name: &str,
        config: PresetConfig,
    ) -> CoreResult<()> {
        if !self.available {
            return Err(oxbox_core::CoreError::InvalidInput(
                "Repair the original preset store before saving.".into(),
            ));
        }
        let mut candidate = self.book.clone();
        candidate.save(name, config)?;
        crate::jobs::register_context(context).check_open()?;
        host.save("native-presets.json", &candidate)?;
        self.book = candidate;
        Ok(())
    }
    fn remove(&mut self, context: &egui::Context, host: &Host, index: usize) -> CoreResult<()> {
        crate::jobs::register_context(context).check_open()?;
        if !self.available {
            return Err(oxbox_core::CoreError::InvalidInput(
                "Repair the original preset store before saving.".into(),
            ));
        }
        if index >= self.book.items.len() {
            return Err(oxbox_core::CoreError::InvalidInput(
                "The selected preset changed; choose it again.".into(),
            ));
        }
        let mut candidate = self.book.clone();
        candidate.items.remove(index);
        host.save("native-presets.json", &candidate)?;
        self.book = candidate;
        self.selected = None;
        Ok(())
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        host: &Host,
        current: CoreResult<Option<PresetConfig>>,
    ) -> Option<PresetConfig> {
        if !matches!(
            id,
            "resize-images" | "verify-checksum" | "timer-stopwatch" | "unit-converter"
        ) {
            return None;
        }
        if self.tool != id {
            self.tool = id.into();
            self.name.clear();
            self.selected = None;
            self.notice.clear();
            if self.available {
                self.error.clear();
            }
        }
        let current = match current {
            Ok(current) => current,
            Err(error) => {
                self.error = error.to_string();
                None
            }
        };
        let mut apply = None;
        ui.push_id(("safe-presets",id),|ui|{
            let section=egui::CollapsingHeader::new("Saved presets").show(ui,|ui|{
                ui.weak("Save tool options only. File paths, text, passphrases and running sessions are excluded.");
                let label=ui.label("Preset name");
                ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64)).labelled_by(label.id);
                if ui.add_enabled(self.available&&current.is_some(),egui::Button::new("Save current options")).clicked(){
                    let name=self.name.trim().to_owned();
                    if let Some(config)=current.clone(){match self.save(ui.ctx(),host,&name,config){Ok(())=>{self.selected=self.book.items.iter().position(|p|p.config.tool_id()==id&&p.name.to_lowercase()==name.to_lowercase());self.error.clear();self.notice="Preset saved on this computer.".into();},Err(error)=>self.error=error.to_string()}}
                }
                let selected=self.selected.and_then(|i|self.book.items.get(i)).filter(|p|p.config.tool_id()==id).map_or("Choose saved options",|p|p.name.as_str());
                let chooser=egui::ComboBox::from_label("Saved configuration").selected_text(selected).show_ui(ui,|ui|{
                    for (index,preset) in self.book.items.iter().enumerate().filter(|(_,p)|p.config.tool_id()==id){ui.selectable_value(&mut self.selected,Some(index),&preset.name);}
                });
                crate::accessibility::combo_expansion(&chooser.response);
                if let Some(index)=self.selected.filter(|i|self.book.items.get(*i).is_some_and(|p|p.config.tool_id()==id)){
                    ui.horizontal(|ui|{
                        if ui.button("Apply selected options").clicked(){apply=Some(self.book.items[index].config.clone());self.error.clear();self.notice.clear();}
                        if ui.add_enabled(self.available,egui::Button::new("Remove selected preset…")).clicked(){
                            let confirmed=crate::host::dialogs::message(ui.ctx()).set_title("Remove saved preset?").set_description(format!("Remove the saved options named {}? Input files and tool results are unaffected.",self.book.items[index].name)).set_buttons(rfd::MessageButtons::YesNo).show()==rfd::MessageDialogResult::Yes;
                            if confirmed {match self.remove(ui.ctx(),host,index){Ok(())=>{self.error.clear();self.notice="Saved preset removed.".into();},Err(error)=>self.error=error.to_string()}}
                        }
                    });
                }
                if !self.error.is_empty(){crate::accessibility::alert(ui,&self.error);}else if !self.notice.is_empty(){crate::accessibility::status(ui,&self.notice);}
            });
            crate::accessibility::disclosure_expansion(&section.header_response);
        });
        apply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_preset_disclosure_actions_operate_body_and_respect_disabled_state() {
        use egui::accesskit::{Action, ActionRequest, Role, TreeId};
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().into()).unwrap();
        for enabled in [false, true] {
            let mut presets = PresetUi::load(&host);
            let config =
                PresetConfig::TimerStopwatch(oxbox_core::presets::TimerPreset { seconds: 60 });
            presets
                .book
                .save("Generated timer", config.clone())
                .unwrap();
            let previous = presets.book.clone();
            let context = egui::Context::default();
            context.enable_accesskit();
            crate::jobs::register_context(&context).close();
            let mut time = 0.0;
            let mut frame = |events| {
                time += 1.0;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        time: Some(time),
                        ..Default::default()
                    },
                    |ui| {
                        ui.add_enabled_ui(enabled, |ui| {
                            assert!(
                                presets
                                    .show(ui, "timer-stopwatch", &host, Ok(Some(config.clone())))
                                    .is_none()
                            );
                        });
                    },
                );
                output.textures_delta.clear();
                output.platform_output.accesskit_update.unwrap().nodes
            };
            let nodes = frame(Vec::new());
            let header = nodes
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == Role::DisclosureTriangle
                        && node.label() == Some("Saved presets"))
                    .then_some(id)
                })
                .unwrap();
            let request = |action| {
                egui::Event::AccessKitActionRequest(ActionRequest {
                    action,
                    target_node: header,
                    target_tree: TreeId::ROOT,
                    data: None,
                })
            };
            frame(vec![request(Action::Expand)]);
            let nodes = frame(Vec::new());
            let expanded = nodes
                .iter()
                .find(|(id, _)| *id == header)
                .unwrap()
                .1
                .is_expanded();
            assert_eq!(
                expanded,
                Some(enabled),
                "Expand must operate the real enabled Saved presets header"
            );
            assert_eq!(
                nodes.iter().any(|(_, node)| node.role() == Role::TextInput),
                enabled,
                "the preset-name body must follow actual disclosure state"
            );
            frame(vec![request(Action::Expand)]);
            let nodes = frame(Vec::new());
            assert_eq!(
                nodes
                    .iter()
                    .find(|(id, _)| *id == header)
                    .unwrap()
                    .1
                    .is_expanded(),
                Some(enabled),
                "Expand must be idempotent"
            );
            frame(vec![request(Action::Collapse)]);
            let nodes = frame(Vec::new());
            assert_eq!(
                nodes
                    .iter()
                    .find(|(id, _)| *id == header)
                    .unwrap()
                    .1
                    .is_expanded(),
                Some(false)
            );
            assert!(!nodes.iter().any(|(_, node)| node.role() == Role::TextInput));
            assert_eq!(presets.book, previous);
            assert!(presets.selected.is_none());
        }
        host.shutdown();
    }
    #[test]
    fn saved_preset_combo_exposes_actual_expanded_and_collapsed_states() {
        use egui::accesskit::{Action, ActionRequest, Role, TreeId};
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().into()).unwrap();
        let mut presets = PresetUi::load(&host);
        let config = PresetConfig::TimerStopwatch(oxbox_core::presets::TimerPreset { seconds: 60 });
        presets
            .book
            .save("Generated timer", config.clone())
            .unwrap();
        let previous = presets.book.clone();
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut time = 0.0;
        let mut frame = |events| {
            time += 1.0;
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ui| {
                    assert!(
                        presets
                            .show(ui, "timer-stopwatch", &host, Ok(Some(config.clone())))
                            .is_none()
                    );
                },
            );
            output.textures_delta.clear();
            output.platform_output.accesskit_update.unwrap().nodes
        };
        let nodes = frame(Vec::new());
        let header = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == Role::DisclosureTriangle && node.label() == Some("Saved presets"))
                    .then_some(id)
            })
            .unwrap();
        let click = |target_node| {
            egui::Event::AccessKitActionRequest(ActionRequest {
                action: Action::Click,
                target_node,
                target_tree: TreeId::ROOT,
                data: None,
            })
        };
        frame(vec![click(header)]);
        let nodes = frame(Vec::new());
        let (chooser, closed) = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == Role::ComboBox).then_some((id, node.is_expanded()))
            })
            .unwrap();
        let nodes = frame(vec![click(chooser)]);
        assert!(
            egui::Popup::is_any_open(&context),
            "fixture must open the actual saved-configuration popup"
        );
        let open = nodes
            .into_iter()
            .find_map(|(id, node)| (id == chooser).then_some(node.is_expanded()))
            .unwrap();
        let nodes = frame(vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        assert!(!egui::Popup::is_any_open(&context));
        let closed_again = nodes
            .into_iter()
            .find_map(|(id, node)| (id == chooser).then_some(node.is_expanded()))
            .unwrap();
        assert_eq!(
            (closed, open, closed_again),
            (Some(false), Some(true), Some(false)),
            "saved chooser must expose its actual expansion state, not omit aria-expanded"
        );
        assert_eq!(presets.book, previous);
        assert!(presets.selected.is_none());
        host.shutdown();
    }
    #[test]
    fn preset_failed_commit_preserves_prior_book_and_invalid_store_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = Host::new(dir.path().into()).unwrap();
        let mut ui = PresetUi::load(&host);
        let config = PresetConfig::TimerStopwatch(oxbox_core::presets::TimerPreset { seconds: 60 });
        std::fs::create_dir(dir.path().join("native-presets.json")).unwrap();
        let context = egui::Context::default();
        assert!(ui.save(&context, &host, "Timer", config.clone()).is_err());
        assert!(ui.book.items.is_empty());
        std::fs::remove_dir(dir.path().join("native-presets.json")).unwrap();
        std::fs::write(dir.path().join("native-presets.json"), b"invalid").unwrap();
        let mut blocked = PresetUi::load(&host);
        assert!(!blocked.available);
        assert!(blocked.save(&context, &host, "Timer", config).is_err());
        assert_eq!(
            std::fs::read(dir.path().join("native-presets.json")).unwrap(),
            b"invalid"
        );
        host.shutdown();
    }
    #[test]
    fn preset_save_after_quit_rejects_without_changing_store_or_book() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = Host::new(dir.path().into()).unwrap();
        let mut ui = PresetUi::load(&host);
        let context = egui::Context::default();
        ui.save(
            &context,
            &host,
            "Timer",
            PresetConfig::TimerStopwatch(oxbox_core::presets::TimerPreset { seconds: 60 }),
        )
        .unwrap();
        let before = std::fs::read(dir.path().join("native-presets.json")).unwrap();
        let book = ui.book.clone();
        crate::jobs::register_context(&context).close();
        assert!(
            ui.save(
                &context,
                &host,
                "Timer",
                PresetConfig::TimerStopwatch(oxbox_core::presets::TimerPreset { seconds: 120 })
            )
            .is_err()
        );
        assert_eq!(ui.book, book);
        assert_eq!(
            std::fs::read(dir.path().join("native-presets.json")).unwrap(),
            before
        );
        host.shutdown();
    }
    #[test]
    fn preset_remove_after_dialog_quit_rejects_without_changing_store_or_book() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = Host::new(dir.path().into()).unwrap();
        let mut ui = PresetUi::load(&host);
        let context = egui::Context::default();
        ui.save(
            &context,
            &host,
            "Timer",
            PresetConfig::TimerStopwatch(oxbox_core::presets::TimerPreset { seconds: 60 }),
        )
        .unwrap();
        let before = std::fs::read(dir.path().join("native-presets.json")).unwrap();
        let book = ui.book.clone();
        // Model Quit arriving while the confirmation is open, before Yes returns.
        crate::jobs::register_context(&context).close();
        assert!(ui.remove(&context, &host, 0).is_err());
        assert_eq!(ui.book, book);
        assert_eq!(
            std::fs::read(dir.path().join("native-presets.json")).unwrap(),
            before
        );
        host.shutdown();
    }
}

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
            egui::CollapsingHeader::new("Saved presets").show(ui,|ui|{
                ui.weak("Save tool options only. File paths, text, passphrases and running sessions are excluded.");
                let label=ui.label("Preset name");
                ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64)).labelled_by(label.id);
                if ui.add_enabled(self.available&&current.is_some(),egui::Button::new("Save current options")).clicked(){
                    let name=self.name.trim().to_owned();
                    if let Some(config)=current.clone(){match self.save(ui.ctx(),host,&name,config){Ok(())=>{self.selected=self.book.items.iter().position(|p|p.config.tool_id()==id&&p.name.to_lowercase()==name.to_lowercase());self.error.clear();self.notice="Preset saved on this computer.".into();},Err(error)=>self.error=error.to_string()}}
                }
                let selected=self.selected.and_then(|i|self.book.items.get(i)).filter(|p|p.config.tool_id()==id).map_or("Choose saved options",|p|p.name.as_str());
                egui::ComboBox::from_label("Saved configuration").selected_text(selected).show_ui(ui,|ui|{
                    for (index,preset) in self.book.items.iter().enumerate().filter(|(_,p)|p.config.tool_id()==id){ui.selectable_value(&mut self.selected,Some(index),&preset.name);}
                });
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
        });
        apply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

use eframe::egui;
use oxbox_core::{
    CoreResult,
    tools::{resize, screenshot::RgbaImage},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub fn introduction(ui: &mut egui::Ui, text: &str) {
    ui.label(text);
    ui.add_space(ui.spacing().item_spacing.y);
}
pub fn note(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.group(|ui| {
        ui.label(egui::RichText::new(text.into()).weak());
    });
}
pub fn error(ui: &mut egui::Ui, text: &Option<String>) {
    if let Some(text) = text {
        ui.group(|ui| {
            crate::accessibility::alert(ui, text);
        });
    }
}
pub fn text(ui: &mut egui::Ui, label: &str, value: &mut String) {
    let label = ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY))
        .labelled_by(label.id);
}
pub fn choice(ui: &mut egui::Ui, label: &str, selected: &mut usize, options: &[&str]) {
    ui.horizontal(|ui| {
        let description = ui.label(label);
        egui::ComboBox::from_id_salt(label)
            .selected_text(options.get(*selected).copied().unwrap_or(options[0]))
            .show_ui(ui, |ui| {
                for (index, label) in options.iter().enumerate() {
                    ui.selectable_value(selected, index, *label);
                }
            })
            .response
            .labelled_by(description.id);
    });
}
pub fn number(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) {
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.add(egui::DragValue::new(value).range(range))
            .labelled_by(label.id);
    });
}
pub fn file(ui: &mut egui::Ui, label: &str, value: &mut String, filters: &[&str]) -> bool {
    let description = ui.label(label);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - 100.0))
            .labelled_by(description.id)
            .changed();
        if crate::accessibility::button(ui, "Choose…", &format!("Choose {label}")).clicked() {
            let mut dialog = crate::host::dialogs::file(ui.ctx()).set_title(label);
            if !filters.is_empty() {
                dialog = dialog.add_filter("Supported files", filters);
            }
            if let Some(path) = dialog.pick_file() {
                *value = path.display().to_string();
                changed = true;
            }
        }
    });
    changed
}
pub fn folder(ui: &mut egui::Ui, label: &str, value: &mut String) {
    let description = ui.label(label);
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - 100.0))
            .labelled_by(description.id);
        if crate::accessibility::button(ui, "Choose…", &format!("Choose {label}")).clicked()
            && let Some(path) = crate::host::dialogs::file(ui.ctx())
                .set_title(label)
                .pick_folder()
        {
            *value = path.display().to_string();
        }
    });
}
pub fn output(ui: &mut egui::Ui, label: &str, value: &mut String, filename: &str) {
    let description = ui.label(label);
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - 100.0))
            .labelled_by(description.id);
        if crate::accessibility::button(ui, "Save as…", &format!("Save {label} as")).clicked()
            && let Some(path) = crate::host::dialogs::file(ui.ctx())
                .set_title(label)
                .set_file_name(filename)
                .save_file()
        {
            *value = path.display().to_string();
        }
    });
}
pub fn confirm(ctx: &egui::Context, title: &str, description: &str) -> bool {
    crate::host::dialogs::message(ctx)
        .set_title(title)
        .set_description(description)
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;
    use egui::accesskit::Role;

    #[test]
    fn file_dialog_buttons_have_distinct_contextual_names() {
        let mut input = String::new();
        let mut directory = String::new();
        let mut destination = String::new();
        let nodes = crate::accessibility::test_tree(|ui| {
            file(ui, "Input image", &mut input, &["png"]);
            folder(ui, "Output folder", &mut directory);
            output(ui, "Output image", &mut destination, "image.png");
        });
        let names = nodes
            .iter()
            .filter(|node| node.role() == Role::Button)
            .filter_map(|node| node.label())
            .collect::<Vec<_>>();
        for name in [
            "Choose Input image",
            "Choose Output folder",
            "Save Output image as",
        ] {
            assert!(
                names.contains(&name),
                "missing contextual accessible button: {name}; got {names:?}"
            );
        }
    }
}
pub fn results(ui: &mut egui::Ui, paths: &[PathBuf]) {
    if !paths.is_empty() {
        ui.group(|ui| {
            crate::accessibility::status(ui, &format!("Saved {} file(s)", paths.len()));
            for path in paths {
                ui.label(path.display().to_string());
            }
        });
    }
}
pub fn open_folder(path: &Path) -> Result<(), String> {
    open::that(path).map_err(|error| error.to_string())
}

pub struct Asset {
    pub image: Arc<RgbaImage>,
    thumbnail: RgbaImage,
}
impl Asset {
    pub fn new(image: RgbaImage) -> Self {
        let thumbnail = image::imageops::thumbnail(&image, 1200, 800);
        Self {
            image: Arc::new(image),
            thumbnail,
        }
    }
    pub fn load(path: &Path) -> CoreResult<Self> {
        Ok(Self::new(resize::decode_oriented(path)?.to_rgba8()))
    }
}
#[derive(Default)]
pub struct Preview {
    pub image: Option<Arc<RgbaImage>>,
    texture: Option<egui::TextureHandle>,
}
impl Preview {
    pub fn set(&mut self, ctx: &egui::Context, asset: Asset) {
        let size = [
            asset.thumbnail.width() as usize,
            asset.thumbnail.height() as usize,
        ];
        self.texture = Some(ctx.load_texture(
            "local image preview",
            egui::ColorImage::from_rgba_unmultiplied(size, asset.thumbnail.as_raw()),
            egui::TextureOptions::LINEAR,
        ));
        self.image = Some(asset.image);
    }
    pub fn show(&self, ui: &mut egui::Ui) -> Option<egui::Response> {
        let texture = self.texture.as_ref()?;
        let size = texture.size_vec2();
        let factor = (ui.available_width() / size.x).min(340.0 / size.y).min(1.0);
        let response = ui.add(
            egui::Image::new((texture.id(), size))
                .fit_to_exact_size(size * factor)
                .sense(egui::Sense::click()),
        );
        crate::accessibility::name(&response, "Local image preview");
        Some(response)
    }
}

#[derive(Default)]
pub struct Providers {
    task: crate::jobs::Task<Vec<oxbox_core::tools::media_provider::ProviderStatus>>,
    statuses: Vec<oxbox_core::tools::media_provider::ProviderStatus>,
}
impl Providers {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui|{
            ui.label("Local media providers");
            ui.label("Oxbox never downloads providers. Install a maintained FFmpeg/Tesseract executable in PATH. Provider licensing belongs to the installed build.");
            if ui.add_enabled(!self.task.running(),egui::Button::new("Check installed providers")).clicked(){
                self.task.start(ui.ctx(),|cancel,_|{cancel.check()?;let ffmpeg=oxbox_core::tools::media_provider::provider_status_with_cancel("ffmpeg",&cancel);cancel.check()?;let tesseract=oxbox_core::tools::media_provider::provider_status_with_cancel("tesseract",&cancel);Ok(vec![ffmpeg,tesseract])});
            }
            for status in &self.statuses {
                ui.label(format!("{}: {}",status.name,status.version.as_deref().unwrap_or("Unavailable")));
                if let Some(path)=&status.path{ui.small(path.display().to_string());}
                if let Some(error)=&status.error{ui.label(error);}
            }
            self.task.show(ui);
        });
    }
    pub fn tick(&mut self) {
        self.task.poll();
        if let Some(value) = self.task.result.take() {
            self.statuses = value;
        }
    }
    pub fn cancel(&self) {
        self.task.cancel();
    }
    pub fn leave(&mut self) {
        self.task.abandon();
    }
    pub fn shutdown(&mut self) {
        self.task.shutdown();
    }
}

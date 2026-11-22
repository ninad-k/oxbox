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
    let context = ui.ctx().clone();
    path_field_using(
        ui,
        label,
        value,
        "Choose…",
        &format!("Choose {label}"),
        || {
            let mut dialog = crate::host::dialogs::file(&context).set_title(label);
            if !filters.is_empty() {
                dialog = dialog.add_filter("Supported files", filters);
            }
            dialog.pick_file()
        },
    )
}
pub fn folder(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    let context = ui.ctx().clone();
    path_field_using(
        ui,
        label,
        value,
        "Choose…",
        &format!("Choose {label}"),
        || {
            crate::host::dialogs::file(&context)
                .set_title(label)
                .pick_folder()
        },
    )
}
pub fn output(ui: &mut egui::Ui, label: &str, value: &mut String, filename: &str) -> bool {
    let context = ui.ctx().clone();
    path_field_using(
        ui,
        label,
        value,
        "Save as…",
        &format!("Save {label} as"),
        || {
            crate::host::dialogs::file(&context)
                .set_title(label)
                .set_file_name(filename)
                .save_file()
        },
    )
}
fn path_field_using(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    button: &str,
    accessible_button: &str,
    choose: impl FnOnce() -> Option<PathBuf>,
) -> bool {
    let error_id = ui.scope_id().with(("media path admission error", label));
    let enabled = ui.is_enabled();
    let description = ui.label(label);
    let mut changed = false;
    let mut candidate = value.clone();
    let mut replacement = None;
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::TextEdit::singleline(&mut candidate)
                    .interactive(enabled)
                    .desired_width(ui.available_width() - 100.0),
            )
            .labelled_by(description.id)
            .changed()
            && enabled
        {
            replacement = Some(PathBuf::from(&candidate));
            changed = true;
        }
        if crate::accessibility::button(ui, button, accessible_button).clicked()
            && enabled
            && let Some(path) = choose()
        {
            replacement = Some(path);
            changed = true;
        }
    });
    if let Some(path) = replacement {
        match crate::host::path_text(&path) {
            Ok(text) => {
                *value = text;
                ui.ctx().data_mut(|data| data.remove::<String>(error_id));
            }
            Err(error) => {
                // Keep the previous valid selection, but still return true so
                // the caller discards results belonging to the refused draft.
                ui.ctx()
                    .data_mut(|data| data.insert_temp(error_id, error.to_string()));
            }
        }
    }
    if let Some(error) = ui.ctx().data(|data| data.get_temp::<String>(error_id)) {
        crate::accessibility::alert(ui, &error);
    }
    changed
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

    #[test]
    fn consumed_provider_results_announce_completion_and_contextual_errors() {
        use oxbox_core::tools::media_provider::ProviderStatus;
        let mut providers = Providers {
            statuses: vec![
                ProviderStatus {
                    name: "ffmpeg".into(),
                    path: None,
                    version: Some("Generated fixture 1.0".into()),
                    error: None,
                },
                ProviderStatus {
                    name: "tesseract".into(),
                    path: None,
                    version: None,
                    error: Some("Generated provider failure".into()),
                },
            ],
            ..Default::default()
        };
        let nodes = crate::accessibility::test_tree(|ui| providers.show(ui));
        assert!(nodes.iter().any(|node| node.role() == Role::Status
            && node.label() == Some("Checked 2 local providers")
            && node.live() == Some(egui::accesskit::Live::Polite)));
        assert!(nodes.iter().any(|node| node.role() == Role::Alert
            && node.label() == Some("Error: tesseract: Generated provider failure")));
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
            if ui.add_enabled(!self.task.running(),egui::Button::new("Check installed providers")).clicked()
                && self.task.start(ui.ctx(),|cancel,_|{cancel.check()?;let ffmpeg=oxbox_core::tools::media_provider::provider_status_with_cancel("ffmpeg",&cancel);cancel.check()?;let tesseract=oxbox_core::tools::media_provider::provider_status_with_cancel("tesseract",&cancel);Ok(vec![ffmpeg,tesseract])}) {
                    self.statuses.clear();
            }
            if !self.statuses.is_empty() {
                crate::accessibility::status(ui, &format!("Checked {} local providers", self.statuses.len()));
            }
            for status in &self.statuses {
                ui.label(format!("{}: {}",status.name,status.version.as_deref().unwrap_or("Unavailable")));
                if let Some(path)=&status.path{ui.small(path.display().to_string());}
                if let Some(error)=&status.error{crate::accessibility::alert(ui, format!("{}: {error}", status.name));}
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

#[cfg(test)]
mod verification56_path_tests {
    use super::*;
    use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId};

    fn frame(
        context: &egui::Context,
        value: &mut String,
        selected: &Path,
        events: Vec<egui::Event>,
    ) -> (bool, Vec<(NodeId, Node)>) {
        let mut changed = false;
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                ui.push_id("first generated form", |ui| {
                    changed = path_field_using(
                        ui,
                        "Generated media path",
                        value,
                        "Choose…",
                        "Choose Generated media path",
                        || Some(selected.to_path_buf()),
                    );
                });
                ui.push_id("second generated form", |ui| {
                    path_field_using(
                        ui,
                        "Generated media path",
                        &mut "Other valid input".to_string(),
                        "Choose…",
                        "Choose Generated media path",
                        || None,
                    );
                });
            },
        );
        output.textures_delta.clear();
        (
            changed,
            output.platform_output.accesskit_update.unwrap().nodes,
        )
    }
    fn action(target_node: NodeId, action: Action, data: Option<ActionData>) -> egui::Event {
        egui::Event::AccessKitActionRequest(ActionRequest {
            action,
            target_node,
            target_tree: TreeId::ROOT,
            data,
        })
    }
    #[test]
    fn verification56_refused_picker_retains_selection_invalidates_and_persists_only_its_field_error()
     {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut value = "Generated valid input.mp4".to_string();
        let selected = Path::new("generated\nrefused.mp4");
        let (_, first) = frame(&context, &mut value, selected, vec![]);
        let button = first
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == Role::Button && node.label() == Some("Choose Generated media path"))
                    .then_some(id)
            })
            .unwrap();
        let (changed, _) = frame(
            &context,
            &mut value,
            selected,
            vec![action(button, Action::Click, None)],
        );
        assert_eq!(
            value, "Generated valid input.mp4",
            "a refused picker must retain the preceding valid selection"
        );
        assert!(
            changed,
            "refused replacement must signal invalidation of stale outcomes"
        );
        let (_, persistent) = frame(&context, &mut value, selected, vec![]);
        assert_eq!(
            persistent
                .iter()
                .filter(|(_, node)| node.role() == Role::Alert)
                .count(),
            1,
            "field error must persist without leaking to another same-named field"
        );
        let field = persistent
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == Role::TextInput && node.value() == Some(value.as_str()))
                    .then_some(id)
            })
            .unwrap();
        let (_, edited) = frame(
            &context,
            &mut value,
            selected,
            vec![action(
                field,
                Action::SetValue,
                Some(ActionData::Value(
                    "Generated accepted replacement.mp4".into(),
                )),
            )],
        );
        assert_eq!(
            value, "Generated accepted replacement.mp4",
            "fixture must edit the actual path control"
        );
        assert!(
            !edited.iter().any(|(_, node)| node.role() == Role::Alert),
            "accepted manual edit must clear the old admission error"
        );
    }
}

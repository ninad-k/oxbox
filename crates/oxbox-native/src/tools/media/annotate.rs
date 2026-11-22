//! Explicit screenshot/local-image annotation. No source pixels are captured here.
use super::{
    Shared,
    widgets::{self, Asset, Preview},
};
use crate::jobs::Task;
use eframe::egui;
use image::RgbaImage;
use oxbox_core::tools::image_annotate::{self, Edit, PixelRect};
use std::{path::PathBuf, sync::Arc};

pub(super) struct ImageEditor {
    active: bool,
    original: Option<Arc<RgbaImage>>,
    preview: Preview,
    load: Task<Asset>,
    edit: Task<Asset>,
    save: Task<PathBuf>,
    copy: Task<()>,
    ocr: Task<String>,
    recognized: String,
    tool: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    end_x: u32,
    end_y: u32,
    stroke: u32,
    scale: u32,
    color: usize,
    label: String,
    engine: usize,
    language: String,
    drag: Option<(u32, u32)>,
    applied: u32,
    resetting: bool,
    error: Option<String>,
}
impl Default for ImageEditor {
    fn default() -> Self {
        Self {
            active: false,
            original: None,
            preview: Preview::default(),
            load: Task::default(),
            edit: Task::default(),
            save: Task::default(),
            copy: Task::default(),
            ocr: Task::default(),
            recognized: String::new(),
            tool: 0,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            end_x: 1,
            end_y: 1,
            stroke: 3,
            scale: 3,
            color: 0,
            label: String::new(),
            engine: usize::from(!cfg!(target_os = "windows")),
            language: if cfg!(target_os = "windows") {
                "en-US".into()
            } else {
                "eng".into()
            },
            drag: None,
            applied: 0,
            resetting: false,
            error: None,
        }
    }
}
impl ImageEditor {
    pub fn active(&self) -> bool {
        self.active
    }
    fn busy(&self) -> bool {
        self.load.running()
            || self.edit.running()
            || self.save.running()
            || self.copy.running()
            || self.ocr.running()
    }
    fn export_image(&self) -> Option<Arc<RgbaImage>> {
        if self.load.running() || self.edit.running() {
            None
        } else {
            self.preview.image.clone()
        }
    }
    pub fn begin_asset(&mut self, ctx: &egui::Context, asset: Asset) {
        self.clear();
        self.active = true;
        self.original = Some(asset.image.clone());
        self.width = asset.image.width();
        self.height = asset.image.height();
        self.end_x = self.width.saturating_sub(1);
        self.end_y = self.height.saturating_sub(1);
        self.preview.set(ctx, asset);
    }
    pub fn begin(&mut self, ctx: &egui::Context, image: Arc<RgbaImage>) {
        if self.busy() {
            self.error =
                Some("An image operation is still stopping; try again when it finishes.".into());
            return;
        }
        self.clear();
        self.active = true;
        self.load.start(ctx, move |cancel, _| {
            let full = PixelRect {
                x: 0,
                y: 0,
                width: image.width(),
                height: image.height(),
            };
            Ok(Asset::new(image_annotate::apply(
                &image,
                &Edit::Crop(full),
                &cancel,
            )?))
        });
    }
    fn invalidate_outcomes(&mut self) {
        self.save.abandon();
        self.copy.abandon();
        self.ocr.abandon();
        self.recognized.clear();
        self.error = None;
    }
    fn color(&self) -> [u8; 3] {
        [
            [0, 0, 0],
            [220, 30, 30],
            [255, 255, 255],
            [30, 90, 220],
            [255, 220, 0],
        ][self.color.min(4)]
    }
    fn operation(&self) -> Edit {
        let rect = PixelRect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        };
        match self.tool {
            1 => Edit::Arrow {
                from: (self.x, self.y),
                to: (self.end_x, self.end_y),
                width: self.stroke,
                color: self.color(),
            },
            2 => Edit::Text {
                at: (self.x, self.y),
                text: self.label.clone(),
                scale: self.scale,
                color: self.color(),
            },
            3 => Edit::Redact {
                rect,
                color: self.color(),
            },
            _ => Edit::Crop(rect),
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, shared: &Shared) {
        ui.separator();
        ui.heading("Image annotation");
        widgets::note(
            ui,
            "Review and flatten edits before exporting. Crop, arrows, bitmap labels and solid redaction modify the preview. Originals are kept; exported PNGs contain only the current pixels, without layers.",
        );
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.busy(),
                    egui::Button::new("Open local image to annotate…"),
                )
                .clicked()
                && let Some(path) = crate::host::dialogs::file(ui.ctx())
                    .set_title("Open a local image for annotation")
                    .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
            {
                self.clear();
                self.active = true;
                self.load.start(ui.ctx(), move |cancel, _| {
                    cancel.check()?;
                    Asset::load(&path)
                });
            }
            if self.active && ui.button("Close editor").clicked() {
                self.clear();
            }
        });
        if !self.active {
            return;
        }
        self.load.show(ui);
        let dimensions = self.preview.image.as_ref().map(|image| image.dimensions());
        if let Some((width, height)) = dimensions {
            crate::accessibility::status(
                ui,
                &format!(
                    "Current flattened image: {width} × {height} pixels · {} applied edits",
                    self.applied
                ),
            );
            let busy = self.busy();
            ui.add_enabled_ui(!busy,|ui|{
                widgets::choice(ui,"Edit tool",&mut self.tool,&["Crop","Arrow","Text label","Solid redaction"]);
                widgets::number(ui,"Start X (pixels)",&mut self.x,0..=width.saturating_sub(1));
                widgets::number(ui,"Start Y (pixels)",&mut self.y,0..=height.saturating_sub(1));
                if self.tool==0 || self.tool==3{
                    widgets::number(ui,"Rectangle width (pixels)",&mut self.width,1..=width);
                    widgets::number(ui,"Rectangle height (pixels)",&mut self.height,1..=height);
                }else if self.tool==1{
                    widgets::number(ui,"Arrow end X (pixels)",&mut self.end_x,0..=width.saturating_sub(1));
                    widgets::number(ui,"Arrow end Y (pixels)",&mut self.end_y,0..=height.saturating_sub(1));
                    widgets::number(ui,"Arrow stroke (pixels)",&mut self.stroke,1..=32);
                }else{
                    let label=ui.label("ASCII text label");
                    ui.add(egui::TextEdit::multiline(&mut self.label).char_limit(256).desired_rows(2).desired_width(f32::INFINITY)).labelled_by(label.id);
                    widgets::number(ui,"Bitmap text scale",&mut self.scale,1..=8);
                    widgets::note(ui,"Original 5×7 bitmap font: printable ASCII letters, digits and punctuation only, at most 256 characters/12 lines. Unicode labels are not supported.");
                }
                if self.tool!=0{widgets::choice(ui,"Annotation color",&mut self.color,&["Black","Red","White","Blue","Yellow"]);}
                if ui.button("Apply edit to preview").clicked() && let Some(image)=self.export_image(){
                    let operation=self.operation();self.invalidate_outcomes();self.resetting=false;
                    self.edit.start(ui.ctx(),move|cancel,_|Ok(Asset::new(image_annotate::apply(&image,&operation,&cancel)?)));
                }
                if ui.button("Reset preview to original").clicked() && let Some(image)=self.original.clone(){
                    self.invalidate_outcomes();self.resetting=true;
                    self.edit.start(ui.ctx(),move|cancel,_|Ok(Asset::new(image_annotate::apply(&image,&Edit::Crop(PixelRect{x:0,y:0,width:image.width(),height:image.height()}),&cancel)?)));
                }
            });
            widgets::note(
                ui,
                "Drag on the preview to set crop/redaction bounds or arrow endpoints; click to position a text label. Numeric controls provide the same operations. Apply commits the edit. Only applied pixels are exported.",
            );
            if let Some(image_response) = self.preview.show(ui) {
                let response = ui.interact(
                    image_response.rect,
                    ui.scope_id().with("annotation selection"),
                    egui::Sense::click_and_drag(),
                );
                crate::accessibility::name(&response, "Annotation selection surface");
                let point = |position: egui::Pos2| {
                    (
                        ((((position.x - response.rect.min.x) / response.rect.width())
                            * width as f32)
                            .floor()
                            .max(0.0) as u32)
                            .min(width.saturating_sub(1)),
                        ((((position.y - response.rect.min.y) / response.rect.height())
                            * height as f32)
                            .floor()
                            .max(0.0) as u32)
                            .min(height.saturating_sub(1)),
                    )
                };
                if !busy {
                    if response.drag_started() {
                        self.drag = ui.input(|input| input.pointer.press_origin()).map(point);
                    }
                    if response.clicked()
                        && self.tool == 2
                        && let Some(position) = response.interact_pointer_pos()
                    {
                        (self.x, self.y) = point(position);
                    }
                    if response.drag_stopped()
                        && let (Some(start), Some(position)) =
                            (self.drag.take(), response.interact_pointer_pos())
                    {
                        let end = point(position);
                        if self.tool == 1 {
                            (self.x, self.y) = start;
                            (self.end_x, self.end_y) = end;
                        } else {
                            self.x = start.0.min(end.0);
                            self.y = start.1.min(end.1);
                            self.width = start.0.abs_diff(end.0) + 1;
                            self.height = start.1.abs_diff(end.1) + 1;
                        }
                    }
                }
            }
            self.edit.show(ui);
            let exportable = !self.busy() && self.export_image().is_some();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(exportable, egui::Button::new("Save flattened PNG…"))
                    .clicked()
                    && let Some(path) = crate::host::dialogs::file(ui.ctx())
                        .set_title("Save a new flattened PNG")
                        .set_file_name("Oxbox-edited.png")
                        .save_file()
                    && let Some(image) = self.export_image()
                {
                    let quit = shared.quit.clone();
                    self.save.start(ui.ctx(), move |cancel, _| {
                        quit.check()?;
                        image_annotate::save_png_exclusive(&image, &path, &cancel)?;
                        Ok(path)
                    });
                }
                if ui
                    .add_enabled(exportable, egui::Button::new("Copy flattened image"))
                    .clicked()
                    && let Some(image) = self.export_image()
                {
                    let clipboard = shared.clipboard.clone();
                    self.copy.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        clipboard.copy_image(&image)
                    });
                }
            });
            self.save.show(ui);
            self.copy.show(ui);
            if let Some(path) = &self.save.result {
                crate::accessibility::status(
                    ui,
                    &format!("Flattened PNG saved: {}", path.display()),
                );
            }
            if self.copy.result.is_some() {
                crate::accessibility::status(
                    ui,
                    "Flattened image copied. History monitoring remains a separate setting.",
                );
            }
            ui.separator();
            ui.strong("Recognize text in this preview");
            let old_engine = self.engine;
            let old_language = self.language.clone();
            widgets::choice(
                ui,
                "Annotation OCR engine",
                &mut self.engine,
                &["Windows native OCR", "Local Tesseract"],
            );
            if old_engine != self.engine {
                self.language = if self.engine == 0 {
                    "en-US".into()
                } else {
                    "eng".into()
                };
            }
            widgets::text(ui, "Annotation OCR language", &mut self.language);
            if old_engine != self.engine || old_language != self.language {
                self.ocr.abandon();
                self.recognized.clear();
            }
            let available = self.engine != 0 || cfg!(target_os = "windows");
            if !available {
                widgets::note(
                    ui,
                    "Native Windows OCR is unavailable here. Local Tesseract needs an installed provider and language data.",
                );
            }
            if ui
                .add_enabled(
                    available && !self.busy(),
                    egui::Button::new("Recognize flattened image text"),
                )
                .clicked()
                && let Some(image) = self.export_image()
            {
                let provider = if self.engine == 0 {
                    "native"
                } else {
                    "tesseract"
                };
                let language = self.language.clone();
                let quit = shared.quit.clone();
                self.recognized.clear();
                self.ocr.start(ui.ctx(), move |cancel, progress| {
                    quit.check()?;
                    image_annotate::recognize(&image, provider, &language, &cancel, progress)
                });
            }
            self.ocr.show(ui);
            if !self.recognized.is_empty() {
                let label = ui.label("Recognized annotation text");
                ui.add(
                    egui::TextEdit::multiline(&mut self.recognized)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label.id);
                if ui.button("Copy recognized annotation text").clicked() {
                    ui.ctx().copy_text(self.recognized.clone());
                }
            }
            widgets::note(
                ui,
                "OCR is explicit and offline, using installed Windows language packs or Tesseract. A private temporary flattened PNG is removed after OCR; cleanup failures are reported. No provider is downloaded.",
            );
        }
        widgets::error(ui, &self.error);
    }
    pub fn tick(&mut self, ctx: Option<&egui::Context>) {
        self.load.poll();
        self.edit.poll();
        self.save.poll();
        self.copy.poll();
        self.ocr.poll();
        if let Some(ctx) = ctx {
            if let Some(asset) = self.load.result.take() {
                self.begin_asset(ctx, asset);
            }
            if let Some(asset) = self.edit.result.take() {
                self.preview.set(ctx, asset);
                self.applied = if self.resetting {
                    0
                } else {
                    self.applied.saturating_add(1)
                };
                self.resetting = false;
                if let Some(image) = &self.preview.image {
                    self.x = self.x.min(image.width().saturating_sub(1));
                    self.y = self.y.min(image.height().saturating_sub(1));
                    self.width = self.width.min(image.width());
                    self.height = self.height.min(image.height());
                }
            }
        }
        if let Some(text) = self.ocr.result.take() {
            self.recognized = text;
        }
    }
    pub fn clear(&mut self) {
        self.load.abandon();
        self.edit.abandon();
        self.save.abandon();
        self.copy.abandon();
        self.ocr.abandon();
        self.active = false;
        self.original = None;
        self.preview = Preview::default();
        self.label.clear();
        self.recognized.clear();
        self.drag = None;
        self.applied = 0;
        self.resetting = false;
        self.x = 0;
        self.y = 0;
        self.error = None;
    }
    pub fn shutdown(&mut self) {
        self.clear();
        self.load.shutdown();
        self.edit.shutdown();
        self.save.shutdown();
        self.copy.shutdown();
        self.ocr.shutdown();
    }
}

#[cfg(test)]
mod editor_tests {
    use super::super::widgets::Asset;
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::sync::Arc;

    #[test]
    fn exports_use_only_the_current_flattened_pixels_not_the_retained_original() {
        let original = Arc::new(RgbaImage::from_pixel(3, 3, Rgba([99, 88, 77, 13])));
        let redacted = Arc::new(RgbaImage::from_pixel(3, 3, Rgba([0, 0, 0, 255])));
        let mut editor = ImageEditor {
            original: Some(original.clone()),
            ..Default::default()
        };
        editor.preview.image = Some(redacted.clone());
        assert_eq!(*editor.export_image().unwrap(), *redacted);
        assert_ne!(*editor.export_image().unwrap(), *original);
        editor.preview.image = None;
        assert!(
            editor.export_image().is_none(),
            "never fall back to the unredacted original for export"
        );
        editor.shutdown();
    }

    #[test]
    fn navigation_discards_draft_pixels_and_queued_edit_or_ocr_completions() {
        let pixels = Arc::new(RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 255])));
        let mut editor = ImageEditor {
            active: true,
            original: Some(pixels.clone()),
            ..Default::default()
        };
        editor.preview.image = Some(pixels.clone());
        editor.edit.result = Some(Asset::new((*pixels).clone()));
        editor.recognized = "Generated recognition draft".into();
        editor.ocr.result = Some("Generated late OCR".into());
        editor.label = "Generated annotation".into();
        editor.clear();
        assert!(!editor.active);
        assert!(editor.original.is_none() && editor.export_image().is_none());
        assert!(editor.edit.result.is_none() && editor.ocr.result.is_none());
        assert!(editor.label.is_empty() && editor.recognized.is_empty());
        editor.shutdown();
    }
}

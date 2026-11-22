use super::{
    Shared,
    widgets::{self, Asset, Preview, Providers},
};
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::tools::{
    barcode_reader::{self, BarcodeResult},
    color_picker::{self, ColorValue},
    ocr,
    screenshot::{self, CaptureKind},
};
use std::{path::PathBuf, time::Duration};

pub struct VisionUi {
    providers: Providers,
    ocr_path: String,
    ocr_provider: usize,
    ocr_language: String,
    ocr_job: Task<String>,
    ocr_text: String,
    barcode_path: String,
    barcode_job: Task<Vec<BarcodeResult>>,
    barcodes: Vec<BarcodeResult>,
    color_path: String,
    color_source: usize,
    x: u32,
    y: u32,
    targets: Task<Vec<screenshot::TargetInfo>>,
    monitors: Vec<screenshot::TargetInfo>,
    monitor: usize,
    color_job: Task<ColorValue>,
    color: Option<ColorValue>,
    hidden: bool,
    preview_job: Task<(usize, Asset)>,
    ocr_preview: Preview,
    barcode_preview: Preview,
    color_preview: Preview,
    error: Option<String>,
}

impl Default for VisionUi {
    fn default() -> Self {
        Self {
            providers: Providers::default(),
            ocr_path: String::new(),
            ocr_provider: usize::from(!cfg!(target_os = "windows")),
            ocr_language: if cfg!(target_os = "windows") {
                "en-US".into()
            } else {
                "eng".into()
            },
            ocr_job: Task::default(),
            ocr_text: String::new(),
            barcode_path: String::new(),
            barcode_job: Task::default(),
            barcodes: Vec::new(),
            color_path: String::new(),
            color_source: 0,
            x: 0,
            y: 0,
            targets: Task::default(),
            monitors: Vec::new(),
            monitor: 0,
            color_job: Task::default(),
            color: None,
            hidden: false,
            preview_job: Task::default(),
            ocr_preview: Preview::default(),
            barcode_preview: Preview::default(),
            color_preview: Preview::default(),
            error: None,
        }
    }
}
impl VisionUi {
    fn input_changed(&mut self, slot: usize) {
        // A preview/result belongs to the file that produced it. Discard even
        // an already-queued completion before another frame can publish it.
        self.preview_job.abandon();
        match slot {
            0 => {
                self.ocr_job.abandon();
                self.ocr_text.clear();
                self.ocr_preview = Preview::default();
            }
            1 => {
                self.barcode_job.abandon();
                self.barcodes.clear();
                self.barcode_preview = Preview::default();
            }
            _ => {
                self.color_job.abandon();
                self.color = None;
                self.color_preview = Preview::default();
                self.error = None;
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, host: &mut Host, shared: &Shared) {
        match id {
            "recognize-text" => self.ocr(ui),
            "barcode-reader" => self.barcode(ui),
            _ => self.color(ui, host, shared),
        }
    }
    fn preview_button(&mut self, ui: &mut egui::Ui, path: String, slot: usize) {
        if ui
            .add_enabled(
                !path.is_empty() && !self.preview_job.running(),
                egui::Button::new("Preview image"),
            )
            .clicked()
        {
            self.preview_job.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                Ok((slot, Asset::load(&PathBuf::from(path))?))
            });
        }
        self.preview_job.show(ui);
    }
    fn ocr(&mut self, ui: &mut egui::Ui) {
        widgets::introduction(
            ui,
            "Recognize text in a local image using an installed offline engine. The image is never uploaded.",
        );
        if widgets::file(
            ui,
            "Image to recognize",
            &mut self.ocr_path,
            &["png", "jpg", "jpeg", "webp"],
        ) {
            self.input_changed(0);
        }
        self.preview_button(ui, self.ocr_path.clone(), 0);
        self.ocr_preview.show(ui);
        let old = self.ocr_provider;
        let old_language = self.ocr_language.clone();
        widgets::choice(
            ui,
            "Recognition engine",
            &mut self.ocr_provider,
            &["Windows native OCR", "Local Tesseract"],
        );
        if old != self.ocr_provider {
            self.ocr_language = if self.ocr_provider == 0 {
                "en-US".into()
            } else {
                "eng".into()
            };
        }
        widgets::text(ui, "OCR language", &mut self.ocr_language);
        if old != self.ocr_provider || old_language != self.ocr_language {
            // The pixels are unchanged, but recognized text and any queued
            // completion belong to the previous engine/language parameters.
            self.ocr_job.abandon();
            self.ocr_text.clear();
        }
        if self.ocr_provider == 1 {
            self.providers.show(ui);
            widgets::note(
                ui,
                "Tesseract language codes use installed traineddata, for example eng or eng+deu. No language data is downloaded.",
            );
        } else {
            widgets::note(
                ui,
                "Windows language tags use BCP-47, for example en-US. Install the OCR language through Windows yourself if unavailable.",
            );
        }
        let available = self.ocr_provider != 0 || cfg!(target_os = "windows");
        if !available {
            widgets::note(
                ui,
                "Windows native OCR is unavailable on this platform. Select the local Tesseract provider.",
            );
        }
        if ui
            .add_enabled(
                available
                    && !self.ocr_path.is_empty()
                    && !self.ocr_language.is_empty()
                    && !self.ocr_job.running(),
                egui::Button::new("Recognize text"),
            )
            .clicked()
        {
            let path = PathBuf::from(&self.ocr_path);
            let provider = if self.ocr_provider == 0 {
                "native"
            } else {
                "tesseract"
            };
            let language = self.ocr_language.clone();
            self.ocr_text.clear();
            self.ocr_job.start(ui.ctx(), move |cancel, progress| {
                ocr::recognize(&path, provider, &language, &cancel, progress)
            });
        }
        self.ocr_job.show(ui);
        if !self.ocr_text.is_empty() {
            let label = ui.label("Recognized text");
            ui.add(
                egui::TextEdit::multiline(&mut self.ocr_text)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(label.id);
            if ui.button("Copy all text").clicked() {
                ui.ctx().copy_text(self.ocr_text.clone());
            }
        }
    }
    fn barcode(&mut self, ui: &mut egui::Ui) {
        widgets::introduction(
            ui,
            "Read QR, Code 128, EAN-13 and other supported barcode formats from a local image. Decoded links are displayed as text, never opened automatically.",
        );
        if widgets::file(
            ui,
            "Barcode image",
            &mut self.barcode_path,
            &["png", "jpg", "jpeg", "webp"],
        ) {
            self.input_changed(1);
        }
        self.preview_button(ui, self.barcode_path.clone(), 1);
        self.barcode_preview.show(ui);
        if ui
            .add_enabled(
                !self.barcode_path.is_empty() && !self.barcode_job.running(),
                egui::Button::new("Read barcodes"),
            )
            .clicked()
        {
            let path = PathBuf::from(&self.barcode_path);
            self.barcodes.clear();
            self.barcode_job.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                barcode_reader::read(&path)
            });
        }
        self.barcode_job.show(ui);
        for result in &self.barcodes {
            ui.group(|ui| {
                ui.strong(&result.format);
                ui.label(&result.text);
                if ui.button("Copy decoded text").clicked() {
                    ui.ctx().copy_text(result.text.clone());
                }
            });
        }
        if self.barcodes.is_empty() && !self.barcode_job.running() {
            widgets::note(
                ui,
                "Choose a sharp, high-contrast barcode image, then select Read barcodes.",
            );
        }
    }
    fn color(&mut self, ui: &mut egui::Ui, host: &mut Host, shared: &Shared) {
        widgets::introduction(
            ui,
            "Measure a pixel as HEX, RGB, HSL and CMYK. Screen capture happens only after your explicit sampling action.",
        );
        let old_source = self.color_source;
        widgets::choice(
            ui,
            "Pixel source",
            &mut self.color_source,
            &["Local image", "Display coordinates", "Cursor · Windows"],
        );
        if old_source != self.color_source {
            self.invalidate_sample();
        }
        if self.color_source == 0 {
            if widgets::file(
                ui,
                "Image to sample",
                &mut self.color_path,
                &["png", "jpg", "jpeg", "webp"],
            ) {
                self.input_changed(2);
            }
            self.preview_button(ui, self.color_path.clone(), 2);
            if let Some(response) = self.color_preview.show(ui)
                && response.clicked()
                && let Some(position) = response.interact_pointer_pos()
                && let Some(image) = &self.color_preview.image
            {
                self.x = (((position.x - response.rect.min.x) / response.rect.width())
                    * image.width() as f32)
                    .floor()
                    .max(0.0) as u32;
                self.y = (((position.y - response.rect.min.y) / response.rect.height())
                    * image.height() as f32)
                    .floor()
                    .max(0.0) as u32;
                self.x = self.x.min(image.width().saturating_sub(1));
                self.y = self.y.min(image.height().saturating_sub(1));
                let sample = color_picker::sample_image(image, self.x, self.y);
                self.invalidate_sample();
                self.apply_color(sample);
            }
            let coordinates = (self.x, self.y);
            widgets::number(ui, "Pixel X", &mut self.x, 0..=32767);
            widgets::number(ui, "Pixel Y", &mut self.y, 0..=32767);
            if coordinates != (self.x, self.y) {
                self.invalidate_sample();
            }
            if ui
                .add_enabled(
                    self.color_preview.image.is_some(),
                    egui::Button::new("Sample image pixel"),
                )
                .clicked()
                && let Some(image) = &self.color_preview.image
            {
                let sample = color_picker::sample_image(image, self.x, self.y);
                self.invalidate_sample();
                self.apply_color(sample);
            }
        } else if self.color_source == 1 {
            let selection = (self.monitor, self.x, self.y);
            if ui.button("Find displays").clicked() {
                self.targets.start(ui.ctx(), |cancel, _| {
                    cancel.check()?;
                    Ok(screenshot::targets()?
                        .into_iter()
                        .filter(|target| target.kind == CaptureKind::Monitor)
                        .collect())
                });
            }
            egui::ComboBox::from_label("Display to sample")
                .selected_text(
                    self.monitors
                        .get(self.monitor)
                        .map_or("Choose a display", |target| target.name.as_str()),
                )
                .show_ui(ui, |ui| {
                    for (index, target) in self.monitors.iter().enumerate() {
                        ui.selectable_value(&mut self.monitor, index, &target.name);
                    }
                });
            self.targets.show(ui);
            widgets::number(ui, "Physical pixel X", &mut self.x, 0..=32767);
            widgets::number(ui, "Physical pixel Y", &mut self.y, 0..=32767);
            if selection != (self.monitor, self.x, self.y) {
                self.invalidate_sample();
            }
            if ui
                .add_enabled(
                    self.monitors.get(self.monitor).is_some() && !self.color_job.running(),
                    egui::Button::new("Sample display pixel"),
                )
                .clicked()
                && let Some(target) = self.monitors.get(self.monitor)
            {
                let id = target.id;
                let x = self.x;
                let y = self.y;
                let quit = shared.quit.clone();
                self.invalidate_sample();
                host.hide(ui.ctx());
                self.hidden = true;
                self.color_job.start(ui.ctx(), move |cancel, _| {
                    screenshot::capture_after_delay(Duration::from_millis(250), &cancel, || {
                        quit.check()?;
                        color_picker::sample_monitor(id, x, y)
                    })
                });
            }
        } else {
            widgets::note(
                ui,
                "The launcher hides for three seconds so you can position the cursor. One screen pixel is sampled, then the launcher returns.",
            );
            if ui
                .add_enabled(
                    cfg!(target_os = "windows") && !self.color_job.running(),
                    egui::Button::new("Sample under cursor in 3 seconds"),
                )
                .clicked()
            {
                let quit = shared.quit.clone();
                self.invalidate_sample();
                host.hide(ui.ctx());
                self.hidden = true;
                self.color_job.start(ui.ctx(), move |cancel, _| {
                    screenshot::capture_after_delay(Duration::from_secs(3), &cancel, || {
                        quit.check()?;
                        color_picker::sample_cursor()
                    })
                });
            }
            if !cfg!(target_os = "windows") {
                widgets::note(
                    ui,
                    "Cursor sampling is unavailable on this platform. Choose a local image or display coordinates.",
                );
            }
        }
        self.color_job.show(ui);
        widgets::error(ui, &self.error);
        if let Some(color) = &self.color {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(color.rgb[0], color.rgb[1], color.rgb[2]),
                        egui::RichText::new("■").size(36.0),
                    );
                    ui.strong(&color.hex);
                });
                for (label, value) in [
                    ("HEX", color.hex.clone()),
                    (
                        "RGB",
                        format!("rgb({}, {}, {})", color.rgb[0], color.rgb[1], color.rgb[2]),
                    ),
                    (
                        "HSL",
                        format!(
                            "hsl({}°, {}%, {}%)",
                            color.hsl[0], color.hsl[1], color.hsl[2]
                        ),
                    ),
                    (
                        "CMYK",
                        format!(
                            "cmyk({}%, {}%, {}%, {}%)",
                            color.cmyk[0], color.cmyk[1], color.cmyk[2], color.cmyk[3]
                        ),
                    ),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(format!("{label}: {value}"));
                        if ui.button(format!("Copy {label}")).clicked() {
                            ui.ctx().copy_text(value);
                        }
                    });
                }
            });
        }
    }
    fn invalidate_sample(&mut self) {
        self.color_job.abandon();
        self.color = None;
        self.error = None;
    }
    fn apply_color(&mut self, value: oxbox_core::CoreResult<ColorValue>) {
        match value {
            Ok(value) => {
                self.color = Some(value);
                self.error = None;
            }
            Err(error) => {
                self.color = None;
                self.error = Some(error.to_string());
            }
        }
    }
    pub fn tick(&mut self, ctx: Option<&egui::Context>, host: &mut Host) {
        self.providers.tick();
        self.ocr_job.poll();
        self.barcode_job.poll();
        self.targets.poll();
        self.color_job.poll();
        self.preview_job.poll();
        if let Some(text) = self.ocr_job.result.take() {
            self.ocr_text = text;
        }
        if let Some(results) = self.barcode_job.result.take() {
            self.barcodes = results;
        }
        if let Some(targets) = self.targets.result.take() {
            if self.color_source == 1 {
                self.invalidate_sample();
            }
            self.monitors = targets;
            self.monitor = 0;
        }
        if let Some(value) = self.color_job.result.take() {
            self.apply_color(Ok(value));
        }
        if self.hidden && !self.color_job.running() {
            self.hidden = false;
            if let Some(ctx) = ctx {
                host.restore(ctx);
            }
        }
        if let Some((slot, asset)) = self.preview_job.result.take()
            && let Some(ctx) = ctx
        {
            match slot {
                0 => self.ocr_preview.set(ctx, asset),
                1 => self.barcode_preview.set(ctx, asset),
                _ => self.color_preview.set(ctx, asset),
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.providers.cancel();
        self.ocr_job.cancel();
        self.barcode_job.cancel();
        self.targets.cancel();
        self.color_job.cancel();
        self.preview_job.cancel();
        self.providers.shutdown();
        self.ocr_job.shutdown();
        self.barcode_job.shutdown();
        self.targets.shutdown();
        self.color_job.shutdown();
        self.preview_job.shutdown();
    }
    pub fn leave(&mut self, id: &str) {
        self.preview_job.abandon();
        self.error = None;
        match id {
            "recognize-text" => {
                self.ocr_job.abandon();
                self.ocr_text.clear();
                self.ocr_preview = Preview::default();
                self.providers.leave();
            }
            "barcode-reader" => {
                self.barcode_job.abandon();
                self.barcodes.clear();
                self.barcode_preview = Preview::default();
            }
            "measure-color" => {
                self.color_job.abandon();
                self.targets.abandon();
                self.color = None;
                self.color_preview = Preview::default();
            }
            _ => {}
        }
        // A hidden sampling operation restores the launcher from tick after
        // cancellation completes; Host retains authoritative visibility.
    }
}

#[cfg(test)]
mod community_tests {
    use super::*;
    use std::sync::Arc;

    fn generated_pixels() -> Arc<screenshot::RgbaImage> {
        Arc::new(screenshot::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([255, 0, 0, 255]),
        ))
    }

    fn edit_image_path(view: &mut VisionUi, tool_id: &str, previous: &str) {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let shared = Shared::default();
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut first = context.run_ui(egui::RawInput::default(), |ui| {
            view.show(ui, tool_id, &mut host, &shared)
        });
        first.textures_delta.clear();
        let field = first
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::TextInput && node.value() == Some(previous))
                    .then_some(id)
            })
            .unwrap();
        let mut second = context.run_ui(
            egui::RawInput {
                events: vec![egui::Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::SetValue,
                        target_node: field,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        data: Some(egui::accesskit::ActionData::Value(
                            "new-generated-input.png".into(),
                        )),
                    },
                )],
                ..Default::default()
            },
            |ui| view.show(ui, tool_id, &mut host, &shared),
        );
        second.textures_delta.clear();
        host.shutdown();
        assert!(shared.clipboard.snapshot("").unwrap().paused);
    }

    #[test]
    fn community_editing_color_image_discards_old_pixels_and_queued_preview() {
        let mut view = VisionUi {
            color_path: "old-generated-color.png".into(),
            color: Some(color_picker::color(255, 0, 0)),
            ..Default::default()
        };
        view.color_preview.image = Some(generated_pixels());
        view.preview_job.result = Some((2, Asset::new((*generated_pixels()).clone())));
        edit_image_path(&mut view, "measure-color", "old-generated-color.png");
        assert_eq!(view.color_path, "new-generated-input.png");
        assert!(
            view.color_preview.image.is_none(),
            "new image path kept old sampleable pixels"
        );
        assert!(view.color.is_none());
        assert!(
            view.preview_job.result.is_none(),
            "old queued preview can reappear"
        );
        view.shutdown();
    }

    #[test]
    fn community_editing_ocr_image_discards_old_result_and_queued_completion() {
        let mut view = VisionUi {
            ocr_path: "old-generated-ocr.png".into(),
            ocr_text: "Old generated recognition result".into(),
            ..Default::default()
        };
        view.ocr_job.result = Some("Queued result for old input".into());
        view.ocr_preview.image = Some(generated_pixels());
        edit_image_path(&mut view, "recognize-text", "old-generated-ocr.png");
        assert_eq!(view.ocr_path, "new-generated-input.png");
        assert!(view.ocr_text.is_empty(), "new OCR path kept an old result");
        assert!(view.ocr_job.result.is_none());
        assert!(view.ocr_preview.image.is_none());
        view.shutdown();
    }

    #[test]
    fn community_editing_barcode_image_discards_old_decoded_values() {
        let mut view = VisionUi {
            barcode_path: "old-generated-barcode.png".into(),
            barcodes: vec![BarcodeResult {
                text: "Old generated barcode".into(),
                format: "QR_CODE".into(),
            }],
            ..Default::default()
        };
        view.barcode_job.result = Some(vec![BarcodeResult {
            text: "Queued old barcode".into(),
            format: "QR_CODE".into(),
        }]);
        view.barcode_preview.image = Some(generated_pixels());
        edit_image_path(&mut view, "barcode-reader", "old-generated-barcode.png");
        assert_eq!(view.barcode_path, "new-generated-input.png");
        assert!(
            view.barcodes.is_empty(),
            "new barcode path kept old decoded values"
        );
        assert!(view.barcode_job.result.is_none());
        assert!(view.barcode_preview.image.is_none());
        view.shutdown();
    }

    #[test]
    fn community_successful_async_color_clears_an_earlier_validation_error() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut view = VisionUi::default();
        view.apply_color(Err(oxbox_core::CoreError::InvalidInput(
            "Generated earlier coordinate error".into(),
        )));
        view.color_job.result = Some(color_picker::color(0, 255, 0));
        view.tick(None, &mut host);
        assert_eq!(view.color.as_ref().unwrap().hex, "#00FF00");
        assert!(
            view.error.is_none(),
            "successful color result still shows an old error"
        );
        view.shutdown();
        host.shutdown();
    }

    fn ocr_frame(
        view: &mut VisionUi,
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Vec<(egui::accesskit::NodeId, egui::accesskit::Node)> {
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| view.ocr(ui),
        );
        output.textures_delta.clear();
        output.platform_output.accesskit_update.unwrap().nodes
    }

    fn ocr_action(
        target_node: egui::accesskit::NodeId,
        action: egui::accesskit::Action,
        data: Option<egui::accesskit::ActionData>,
    ) -> egui::Event {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action,
            target_node,
            target_tree: egui::accesskit::TreeId::ROOT,
            data,
        })
    }

    fn generated_ocr_view() -> VisionUi {
        let mut view = VisionUi {
            ocr_path: "generated-ocr.png".into(),
            ocr_provider: 0,
            ocr_language: "en-US".into(),
            ocr_text: "Generated previous recognition".into(),
            ..Default::default()
        };
        view.ocr_job.result = Some("Generated queued previous recognition".into());
        view.ocr_preview.image = Some(generated_pixels());
        view
    }

    fn assert_old_recognition_discarded(view: &VisionUi) {
        assert!(
            view.ocr_text.is_empty(),
            "changed OCR settings kept recognition for the old settings"
        );
        assert!(
            view.ocr_job.result.is_none(),
            "queued old recognition can reappear"
        );
        assert!(
            view.ocr_preview.image.is_some(),
            "engine/language edits should retain the same input image preview"
        );
    }

    #[test]
    fn verification2_ocr_language_edit_discards_old_text_and_queued_recognition() {
        let mut view = generated_ocr_view();
        let context = egui::Context::default();
        context.enable_accesskit();
        let nodes = ocr_frame(&mut view, &context, vec![]);
        let field = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::TextInput && node.value() == Some("en-US"))
                    .then_some(id)
            })
            .unwrap();
        ocr_frame(
            &mut view,
            &context,
            vec![ocr_action(
                field,
                egui::accesskit::Action::SetValue,
                Some(egui::accesskit::ActionData::Value("de-DE".into())),
            )],
        );
        assert_eq!(
            view.ocr_language, "de-DE",
            "fixture must edit the real language control"
        );
        assert_old_recognition_discarded(&view);
        view.shutdown();
    }

    #[test]
    fn verification2_ocr_engine_edit_discards_old_text_and_queued_recognition() {
        let mut view = generated_ocr_view();
        let context = egui::Context::default();
        context.enable_accesskit();
        let nodes = ocr_frame(&mut view, &context, vec![]);
        let field = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::ComboBox
                    && node.value() == Some("Windows native OCR"))
                .then_some(id)
            })
            .unwrap();
        ocr_frame(
            &mut view,
            &context,
            vec![ocr_action(field, egui::accesskit::Action::Click, None)],
        );
        // Let the popup's initial invisible sizing pass complete before
        // targeting the accessible engine option.
        let nodes = ocr_frame(&mut view, &context, vec![]);
        let option = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::Button
                    && node.label() == Some("Local Tesseract"))
                .then_some(id)
            })
            .unwrap();
        ocr_frame(
            &mut view,
            &context,
            vec![ocr_action(option, egui::accesskit::Action::Click, None)],
        );
        assert_eq!(
            view.ocr_provider, 1,
            "fixture must edit the real engine combo"
        );
        assert_eq!(view.ocr_language, "eng");
        assert_old_recognition_discarded(&view);
        view.shutdown();
    }

    fn color_frame(
        view: &mut VisionUi,
        host: &mut Host,
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> Vec<(egui::accesskit::NodeId, egui::accesskit::Node)> {
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                view.show(ui, "measure-color", host, &Shared::default());
            },
        );
        output.textures_delta.clear();
        output.platform_output.accesskit_update.unwrap().nodes
    }

    #[test]
    fn verification56_failed_image_color_sample_removes_previous_copyable_value() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut view = VisionUi {
            x: 99,
            color: Some(color_picker::color(255, 0, 0)),
            ..Default::default()
        };
        view.color_preview.image = Some(generated_pixels());
        let button = color_frame(&mut view, &mut host, &context, vec![])
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::Button
                    && node.label() == Some("Sample image pixel"))
                .then_some(id)
            })
            .unwrap();
        let nodes = color_frame(
            &mut view,
            &mut host,
            &context,
            vec![ocr_action(button, egui::accesskit::Action::Click, None)],
        );
        assert!(
            view.error.is_some(),
            "fixture must reach real out-of-bounds image sampling"
        );
        let old_value = view.color.is_some();
        let copyable = nodes.iter().any(|(_, node)| {
            node.role() == egui::accesskit::Role::Button && node.label() == Some("Copy HEX")
        });
        view.shutdown();
        host.shutdown();
        assert!(
            !old_value && !copyable,
            "failed sampling kept the previous copyable color"
        );
    }

    #[test]
    fn verification56_color_coordinate_edit_discards_previous_and_queued_sample() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut view = VisionUi {
            x: 1,
            color: Some(color_picker::color(255, 0, 0)),
            ..Default::default()
        };
        view.color_job.result = Some(color_picker::color(0, 255, 0));
        view.color_preview.image = Some(generated_pixels());
        let field = color_frame(&mut view, &mut host, &context, vec![])
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::SpinButton
                    && node.numeric_value() == Some(1.0))
                .then_some(id)
            })
            .unwrap();
        color_frame(
            &mut view,
            &mut host,
            &context,
            vec![ocr_action(
                field,
                egui::accesskit::Action::SetValue,
                Some(egui::accesskit::ActionData::NumericValue(2.0)),
            )],
        );
        assert_eq!(
            view.x, 2,
            "fixture must edit the real pixel coordinate control"
        );
        let discarded = view.color.is_none() && view.color_job.result.is_none();
        view.shutdown();
        host.shutdown();
        assert!(
            discarded,
            "edited coordinates retained a previous or queued copyable sample"
        );
    }

    #[test]
    fn verification56_display_refresh_discards_previous_and_queued_sample_for_old_monitor() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let target = |id| screenshot::TargetInfo {
            kind: CaptureKind::Monitor,
            id,
            name: format!("Generated display {id}"),
            x: 0,
            y: 0,
            width: 2,
            height: 2,
            scale_factor: 1.0,
        };
        let mut view = VisionUi {
            color_source: 1,
            monitors: vec![target(1)],
            color: Some(color_picker::color(255, 0, 0)),
            ..Default::default()
        };
        view.targets.result = Some(vec![target(2)]);
        view.color_job.result = Some(color_picker::color(0, 255, 0));
        view.tick(None, &mut host);
        assert_eq!(
            view.monitors[0].id, 2,
            "fixture must replace the actual display list"
        );
        let discarded = view.color.is_none() && view.color_job.result.is_none();
        view.shutdown();
        host.shutdown();
        assert!(
            discarded,
            "new display identity kept a sample from the old monitor"
        );
    }
}

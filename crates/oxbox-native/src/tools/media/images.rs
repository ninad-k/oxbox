use super::{
    Shared,
    widgets::{self, Asset, Preview, Providers},
};
use crate::jobs::Task;
use eframe::egui;
use oxbox_core::tools::{
    convert_media::{self, MediaFormat},
    make_gif::{self, GifOptions},
    qr::{self, CodeKind, GeneratedCode},
    resize::{self, OutputFormat, ResizeMode, ResizeRequest},
};
use std::path::PathBuf;

struct QrResult {
    code: GeneratedCode,
    asset: Asset,
}

pub struct ImageTools {
    providers: Providers,
    qr_text: String,
    qr_kind: usize,
    qr_ec: usize,
    qr_size: u32,
    qr_svg: bool,
    qr_job: Task<QrResult>,
    qr_code: Option<GeneratedCode>,
    qr_preview: Preview,
    qr_action: Task<PathBuf>,
    copy_job: Task<()>,
    resize_paths: Vec<PathBuf>,
    resize_folder: String,
    resize_mode: usize,
    percent: f64,
    width: u32,
    height: u32,
    format: usize,
    quality: u32,
    resize_job: Task<Vec<PathBuf>>,
    resize_results: Vec<PathBuf>,
    resize_preview: Preview,
    preview_job: Task<Asset>,
    gif_input: String,
    gif_output: String,
    gif_start: f64,
    gif_seconds: f64,
    gif_fps: u32,
    gif_width: u32,
    gif_job: Task<PathBuf>,
    gif_result: Option<PathBuf>,
    convert_input: String,
    convert_output: String,
    convert_format: usize,
    convert_job: Task<PathBuf>,
    convert_result: Option<PathBuf>,
}
impl Default for ImageTools {
    fn default() -> Self {
        Self {
            providers: Providers::default(),
            qr_text: String::new(),
            qr_kind: 0,
            qr_ec: 1,
            qr_size: 512,
            qr_svg: false,
            qr_job: Task::default(),
            qr_code: None,
            qr_preview: Preview::default(),
            qr_action: Task::default(),
            copy_job: Task::default(),
            resize_paths: Vec::new(),
            resize_folder: String::new(),
            resize_mode: 0,
            percent: 50.0,
            width: 1920,
            height: 1080,
            format: 0,
            quality: 90,
            resize_job: Task::default(),
            resize_results: Vec::new(),
            resize_preview: Preview::default(),
            preview_job: Task::default(),
            gif_input: String::new(),
            gif_output: String::new(),
            gif_start: 0.0,
            gif_seconds: 5.0,
            gif_fps: 12,
            gif_width: 640,
            gif_job: Task::default(),
            gif_result: None,
            convert_input: String::new(),
            convert_output: String::new(),
            convert_format: 0,
            convert_job: Task::default(),
            convert_result: None,
        }
    }
}
impl ImageTools {
    pub fn export_resize_preset(
        &self,
    ) -> oxbox_core::CoreResult<oxbox_core::presets::ResizePreset> {
        if self.resize_job.running() {
            return Err(oxbox_core::CoreError::InvalidInput(
                "wait for the current resize job before saving a preset".into(),
            ));
        }
        let mode = ["percent", "fit", "exact"]
            .get(self.resize_mode)
            .ok_or_else(|| {
                oxbox_core::CoreError::InvalidInput("choose a valid resize mode".into())
            })?;
        let format = ["png", "jpeg", "webp"].get(self.format).ok_or_else(|| {
            oxbox_core::CoreError::InvalidInput("choose a valid image output format".into())
        })?;
        let preset = oxbox_core::presets::ResizePreset {
            mode: (*mode).into(),
            percent: self.percent,
            width: self.width,
            height: self.height,
            format: (*format).into(),
            quality: self.quality,
        };
        preset.validate()?;
        Ok(preset)
    }

    pub fn import_resize_preset(
        &mut self,
        preset: &oxbox_core::presets::ResizePreset,
    ) -> oxbox_core::CoreResult<()> {
        preset.validate()?;
        if self.resize_job.running() {
            return Err(oxbox_core::CoreError::InvalidInput(
                "wait for the current resize job before applying a preset".into(),
            ));
        }
        self.resize_mode = match preset.mode.as_str() {
            "percent" => 0,
            "fit" => 1,
            _ => 2,
        };
        self.percent = preset.percent;
        self.width = preset.width;
        self.height = preset.height;
        self.format = match preset.format.as_str() {
            "png" => 0,
            "jpeg" => 1,
            _ => 2,
        };
        self.quality = preset.quality;
        self.resize_job.abandon();
        self.preview_job.abandon();
        self.resize_results.clear();
        self.resize_preview = Preview::default();
        Ok(())
    }

    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, shared: &Shared) {
        match id {
            "qr-code" => self.qr(ui, shared),
            "resize-images" => self.resize(ui),
            "make-gif" => self.gif(ui),
            _ => self.convert(ui),
        }
    }
    fn qr(&mut self, ui: &mut egui::Ui, shared: &Shared) {
        widgets::introduction(
            ui,
            "Create a QR code or barcode from text. Preview locally, save a new image, or copy it to the clipboard.",
        );
        let previous = (self.qr_kind, self.qr_ec, self.qr_size);
        let label = ui.label("Text or barcode value");
        let text_changed = ui
            .add(
                egui::TextEdit::multiline(&mut self.qr_text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .char_limit(4096),
            )
            .labelled_by(label.id)
            .changed();
        widgets::choice(
            ui,
            "Code type",
            &mut self.qr_kind,
            &["QR code", "Code 128", "EAN-13"],
        );
        if self.qr_kind == 0 {
            widgets::choice(
                ui,
                "Error correction",
                &mut self.qr_ec,
                &["L · 7%", "M · 15%", "Q · 25%", "H · 30%"],
            );
        }
        if self.qr_kind == 2 {
            widgets::note(
                ui,
                "EAN-13 needs 12 digits, or 13 digits with a valid check digit.",
            );
        }
        widgets::number(ui, "Image size (pixels)", &mut self.qr_size, 128..=2048);
        if text_changed || previous != (self.qr_kind, self.qr_ec, self.qr_size) {
            // Save and Copy must only use a generation matching this form.
            // Abandon also discards a completion already queued by the worker.
            self.qr_job.abandon();
            self.qr_code = None;
            self.qr_preview = Preview::default();
            self.qr_action.abandon();
            self.copy_job.abandon();
        }
        if ui
            .add_enabled(
                !self.qr_text.is_empty() && !self.qr_job.running(),
                egui::Button::new("Generate preview"),
            )
            .clicked()
        {
            let text = self.qr_text.clone();
            let kind = [CodeKind::Qr, CodeKind::Code128, CodeKind::Ean13][self.qr_kind];
            let correction = ["L", "M", "Q", "H"][self.qr_ec];
            let size = self.qr_size;
            self.qr_job.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                let code = qr::generate(&text, kind, correction, size)?;
                let asset = Asset::new(qr::decode_image(&code)?);
                Ok(QrResult { code, asset })
            });
        }
        self.qr_job.show(ui);
        self.qr_preview.show(ui);
        if let Some(code) = &self.qr_code {
            ui.checkbox(&mut self.qr_svg, "Save SVG (otherwise PNG)");
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.qr_action.running() && !self.qr_job.running(),
                        egui::Button::new("Save code…"),
                    )
                    .clicked()
                {
                    let svg = self.qr_svg;
                    if let Some(path) = crate::host::dialogs::file(ui.ctx())
                        .set_file_name(if svg {
                            "Oxbox-code.svg"
                        } else {
                            "Oxbox-code.png"
                        })
                        .save_file()
                    {
                        let code = GeneratedCode {
                            svg: code.svg.clone(),
                            png: code.png.clone(),
                        };
                        self.qr_action.start(ui.ctx(), move |cancel, _| {
                            cancel.check()?;
                            qr::save(&code, &path, svg)?;
                            Ok(path)
                        });
                    }
                }
                if ui
                    .add_enabled(
                        !self.copy_job.running() && !self.qr_job.running(),
                        egui::Button::new("Copy image"),
                    )
                    .clicked()
                    && let Some(image) = &self.qr_preview.image
                {
                    let image = image.clone();
                    let clipboard = shared.clipboard.clone();
                    self.copy_job.start(ui.ctx(), move |cancel, _| {
                        cancel.check()?;
                        clipboard.copy_image(&image)
                    });
                }
            });
        }
        self.qr_action.show(ui);
        self.copy_job.show(ui);
        if self.copy_job.result.is_some() {
            widgets::note(ui, "Image copied to the clipboard.");
        }
        if let Some(path) = &self.qr_action.result {
            widgets::note(ui, format!("Saved {}", path.display()));
        }
        widgets::note(
            ui,
            "Saved files never replace existing files. Copying a code is an explicit clipboard action; it does not enable clipboard history.",
        );
    }
    fn resize(&mut self, ui: &mut egui::Ui) {
        widgets::introduction(
            ui,
            "Resize and convert local images in a batch. EXIF orientation is applied and original files are preserved.",
        );
        let previous = (
            self.resize_paths.clone(),
            self.resize_folder.clone(),
            self.resize_mode,
            self.percent.to_bits(),
            self.width,
            self.height,
            self.format,
            self.quality,
        );
        let busy = self.resize_job.running();
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Add images…").clicked()
                    && let Some(paths) = crate::host::dialogs::file(ui.ctx())
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                        .pick_files()
                {
                    for path in paths {
                        if !self.resize_paths.contains(&path) {
                            self.resize_paths.push(path);
                        }
                    }
                }
                if ui.button("Clear selection").clicked() {
                    self.resize_paths.clear();
                }
            });
            let mut remove = None;
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    for (index, path) in self.resize_paths.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(path.display().to_string());
                            if ui.small_button("Remove").clicked() {
                                remove = Some(index);
                            }
                        });
                    }
                });
            if let Some(index) = remove {
                self.resize_paths.remove(index);
            }
            widgets::choice(
                ui,
                "Resize mode",
                &mut self.resize_mode,
                &["Percentage", "Fit within bounds", "Exact dimensions"],
            );
            if self.resize_mode == 0 {
                ui.horizontal(|ui| {
                    let label = ui.label("Percentage");
                    ui.add(
                        egui::DragValue::new(&mut self.percent)
                            .range(0.1..=1000.0)
                            .suffix("%"),
                    )
                    .labelled_by(label.id);
                });
            } else {
                widgets::number(ui, "Width (pixels)", &mut self.width, 1..=32768);
                widgets::number(ui, "Height (pixels)", &mut self.height, 1..=32768);
            }
            widgets::choice(
                ui,
                "Output format",
                &mut self.format,
                &["PNG", "JPEG", "WebP"],
            );
            if self.format == 1 {
                widgets::number(ui, "JPEG quality", &mut self.quality, 1..=100);
            }
            let path_changed = widgets::folder(ui, "Destination folder", &mut self.resize_folder);
            if busy {
                // AccessKit value actions can reach disabled text fields. Restore
                // the admitted model before any invalidation or new-job action.
                let (paths, folder, mode, percent, width, height, format, quality) = previous;
                self.resize_paths = paths;
                self.resize_folder = folder;
                self.resize_mode = mode;
                self.percent = f64::from_bits(percent);
                self.width = width;
                self.height = height;
                self.format = format;
                self.quality = quality;
                return;
            }
            if path_changed
                || previous
                    != (
                        self.resize_paths.clone(),
                        self.resize_folder.clone(),
                        self.resize_mode,
                        self.percent.to_bits(),
                        self.width,
                        self.height,
                        self.format,
                        self.quality,
                    )
            {
                self.resize_job.abandon();
                self.preview_job.abandon();
                self.resize_results.clear();
                self.resize_preview = Preview::default();
            }
            if ui
                .add_enabled(
                    !self.resize_paths.is_empty()
                        && !self.resize_folder.is_empty()
                        && !self.resize_job.running(),
                    egui::Button::new("Resize images"),
                )
                .clicked()
            {
                let request = ResizeRequest {
                    paths: self.resize_paths.clone(),
                    output_dir: PathBuf::from(&self.resize_folder),
                    mode: match self.resize_mode {
                        0 => ResizeMode::Percent {
                            percent: self.percent,
                        },
                        1 => ResizeMode::Fit {
                            width: self.width,
                            height: self.height,
                        },
                        _ => ResizeMode::Exact {
                            width: self.width,
                            height: self.height,
                        },
                    },
                    format: [OutputFormat::Png, OutputFormat::Jpeg, OutputFormat::Webp]
                        [self.format],
                    jpeg_quality: self.quality as u8,
                };
                self.resize_job.abandon();
                self.preview_job.abandon();
                self.resize_results.clear();
                self.resize_preview = Preview::default();
                self.resize_job.start(ui.ctx(), move |cancel, progress| {
                    resize::resize_batch(&request, &cancel, progress)
                });
            }
        });
        self.resize_job.show(ui);
        widgets::results(ui, &self.resize_results);
        if let Some(path) = self.resize_results.first()
            && ui.button("Preview first output").clicked()
        {
            let path = path.clone();
            self.preview_job.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                Asset::load(&path)
            });
        }
        self.preview_job.show(ui);
        self.resize_preview.show(ui);
        widgets::note(
            ui,
            "Each output adds -resized to its name. Existing outputs are refused, including duplicate destinations. Cancel preserves originals and completed outputs.",
        );
    }
    fn gif(&mut self, ui: &mut egui::Ui) {
        widgets::introduction(
            ui,
            "Create an animated GIF from a bounded segment of a local video.",
        );
        self.providers.show(ui);
        let previous = (
            self.gif_input.clone(),
            self.gif_output.clone(),
            self.gif_start.to_bits(),
            self.gif_seconds.to_bits(),
            self.gif_fps,
            self.gif_width,
        );
        let busy = self.gif_job.running();
        ui.add_enabled_ui(!busy, |ui| {
            let input_changed = widgets::file(
                ui,
                "Input video",
                &mut self.gif_input,
                &["mp4", "webm", "mov", "mkv", "avi"],
            );
            ui.horizontal(|ui| {
                let label = ui.label("Start (seconds)");
                ui.add(egui::DragValue::new(&mut self.gif_start).range(0.0..=604800.0))
                    .labelled_by(label.id);
            });
            ui.horizontal(|ui| {
                let label = ui.label("Duration (seconds)");
                ui.add(egui::DragValue::new(&mut self.gif_seconds).range(0.1..=60.0))
                    .labelled_by(label.id);
            });
            widgets::number(ui, "Frame rate", &mut self.gif_fps, 1..=30);
            widgets::number(ui, "Width (pixels)", &mut self.gif_width, 16..=1280);
            let output_changed = widgets::output(
                ui,
                "New GIF output",
                &mut self.gif_output,
                "Oxbox-animation.gif",
            );
            if busy {
                let (input, output, start, seconds, fps, width) = previous;
                self.gif_input = input;
                self.gif_output = output;
                self.gif_start = f64::from_bits(start);
                self.gif_seconds = f64::from_bits(seconds);
                self.gif_fps = fps;
                self.gif_width = width;
                return;
            }
            if input_changed
                || output_changed
                || previous
                    != (
                        self.gif_input.clone(),
                        self.gif_output.clone(),
                        self.gif_start.to_bits(),
                        self.gif_seconds.to_bits(),
                        self.gif_fps,
                        self.gif_width,
                    )
            {
                self.gif_job.abandon();
                self.gif_result = None;
            }
            if ui
                .add_enabled(
                    !self.gif_input.is_empty()
                        && !self.gif_output.is_empty()
                        && !self.gif_job.running(),
                    egui::Button::new("Make GIF"),
                )
                .clicked()
            {
                let input = PathBuf::from(&self.gif_input);
                let output = PathBuf::from(&self.gif_output);
                let options = GifOptions {
                    start_seconds: self.gif_start,
                    seconds: self.gif_seconds,
                    fps: self.gif_fps,
                    width: self.gif_width,
                };
                self.gif_job.abandon();
                self.gif_result = None;
                self.gif_job.start(ui.ctx(), move |cancel, progress| {
                    make_gif::make(&input, &output, &options, &cancel, progress)
                });
            }
        });
        self.gif_job.show(ui);
        if let Some(path) = &self.gif_result {
            widgets::note(ui, format!("GIF saved to {}", path.display()));
        }
        widgets::note(
            ui,
            "Requires your local FFmpeg provider. Inputs and outputs stay local, originals remain intact, and existing outputs are refused.",
        );
    }
    fn convert(&mut self, ui: &mut egui::Ui) {
        widgets::introduction(
            ui,
            "Convert a local audio or video file using a fixed, reviewed preset.",
        );
        self.providers.show(ui);
        let previous = (
            self.convert_input.clone(),
            self.convert_output.clone(),
            self.convert_format,
        );
        let busy = self.convert_job.running();
        ui.add_enabled_ui(!busy, |ui| {
            let input_changed = widgets::file(ui, "Input media", &mut self.convert_input, &[]);
            widgets::choice(
                ui,
                "Output format",
                &mut self.convert_format,
                &[
                    "MP4 · video + audio",
                    "WebM · video + audio",
                    "MP3 · audio",
                    "WAV · audio",
                    "FLAC · audio",
                ],
            );
            let format = [
                MediaFormat::Mp4,
                MediaFormat::Webm,
                MediaFormat::Mp3,
                MediaFormat::Wav,
                MediaFormat::Flac,
            ][self.convert_format];
            let output_changed = widgets::output(
                ui,
                "New converted file",
                &mut self.convert_output,
                &format!("Oxbox-converted.{}", format.extension()),
            );
            if busy {
                (self.convert_input, self.convert_output, self.convert_format) = previous;
                return;
            }
            if input_changed
                || output_changed
                || previous
                    != (
                        self.convert_input.clone(),
                        self.convert_output.clone(),
                        self.convert_format,
                    )
            {
                self.convert_job.abandon();
                self.convert_result = None;
            }
            if ui
                .add_enabled(
                    !self.convert_input.is_empty()
                        && !self.convert_output.is_empty()
                        && !self.convert_job.running(),
                    egui::Button::new("Convert media"),
                )
                .clicked()
            {
                let input = PathBuf::from(&self.convert_input);
                let output = PathBuf::from(&self.convert_output);
                self.convert_job.abandon();
                self.convert_result = None;
                self.convert_job.start(ui.ctx(), move |cancel, progress| {
                    convert_media::convert(&input, &output, format, &cancel, progress)
                });
            }
        });
        self.convert_job.show(ui);
        if let Some(path) = &self.convert_result {
            widgets::note(ui, format!("Converted file saved to {}", path.display()));
        }
        widgets::note(
            ui,
            "No URL inputs, remote playlists or downloads. Requires your installed FFmpeg build; codec support and its license depend on that build. Output publication never overwrites a user file.",
        );
    }
    pub fn tick(&mut self, ctx: Option<&egui::Context>) {
        self.providers.tick();
        self.qr_job.poll();
        self.qr_action.poll();
        self.copy_job.poll();
        self.resize_job.poll();
        self.preview_job.poll();
        self.gif_job.poll();
        self.convert_job.poll();
        if let Some(ctx) = ctx {
            if let Some(result) = self.qr_job.result.take() {
                self.qr_preview.set(ctx, result.asset);
                self.qr_code = Some(result.code);
            }
            if let Some(asset) = self.preview_job.result.take() {
                self.resize_preview.set(ctx, asset);
            }
        }
        if let Some(paths) = self.resize_job.result.take() {
            self.resize_results = paths;
        }
        if let Some(path) = self.gif_job.result.take() {
            self.gif_result = Some(path);
        }
        if let Some(path) = self.convert_job.result.take() {
            self.convert_result = Some(path);
        }
    }
    pub fn shutdown(&mut self) {
        self.providers.cancel();
        self.qr_job.cancel();
        self.qr_action.cancel();
        self.copy_job.cancel();
        self.resize_job.cancel();
        self.preview_job.cancel();
        self.gif_job.cancel();
        self.convert_job.cancel();
        self.providers.shutdown();
        self.qr_job.shutdown();
        self.qr_action.shutdown();
        self.copy_job.shutdown();
        self.resize_job.shutdown();
        self.preview_job.shutdown();
        self.gif_job.shutdown();
        self.convert_job.shutdown();
    }
    pub fn leave(&mut self, id: &str) {
        match id {
            "qr-code" => {
                self.qr_job.abandon();
                self.qr_action.abandon();
                self.copy_job.abandon();
                self.qr_text.clear();
                self.qr_code = None;
                self.qr_preview = Preview::default();
            }
            "resize-images" => {
                self.resize_job.abandon();
                self.preview_job.abandon();
                self.resize_paths.clear();
                self.resize_results.clear();
                self.resize_preview = Preview::default();
            }
            "make-gif" => {
                self.gif_job.abandon();
                self.gif_result = None;
                self.providers.leave();
            }
            "convert-audio-video" => {
                self.convert_job.abandon();
                self.convert_result = None;
                self.providers.leave();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod verification2_tests {
    use super::*;

    #[test]
    fn resize_preset_round_trip_contains_only_options_and_preserves_input_paths() {
        let mut view = ImageTools {
            resize_paths: vec![PathBuf::from("generated-input.png")],
            resize_folder: "generated-destination".into(),
            resize_mode: 2,
            percent: 25.0,
            width: 640,
            height: 480,
            format: 1,
            quality: 83,
            ..Default::default()
        };
        let preset = view.export_resize_preset().unwrap();
        let fields = serde_json::to_value(&preset).unwrap();
        assert_eq!(fields.as_object().unwrap().len(), 6);
        assert_eq!(fields["mode"], "exact");
        assert_eq!(fields["format"], "jpeg");
        view.resize_mode = 0;
        view.width = 1;
        view.import_resize_preset(&preset).unwrap();
        assert_eq!(
            (
                view.resize_mode,
                view.width,
                view.height,
                view.format,
                view.quality
            ),
            (2, 640, 480, 1, 83)
        );
        assert_eq!(
            view.resize_paths,
            vec![PathBuf::from("generated-input.png")]
        );
        assert_eq!(view.resize_folder, "generated-destination");
        let mut invalid = preset;
        invalid.quality = 0;
        assert!(view.import_resize_preset(&invalid).is_err());
        assert_eq!((view.resize_mode, view.width, view.quality), (2, 640, 83));
        view.shutdown();
    }

    fn frame(
        view: &mut ImageTools,
        context: &egui::Context,
        shared: &Shared,
        events: Vec<egui::Event>,
    ) -> Vec<(egui::accesskit::NodeId, egui::accesskit::Node)> {
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| view.show(ui, "qr-code", shared),
        );
        output.textures_delta.clear();
        output.platform_output.accesskit_update.unwrap().nodes
    }

    fn action(
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

    fn generated_view() -> ImageTools {
        let mut view = ImageTools {
            qr_text: "Generated original value".into(),
            ..Default::default()
        };
        view.qr_code = Some(qr::generate(&view.qr_text, CodeKind::Qr, "M", 128).unwrap());
        view.qr_preview.image = Some(std::sync::Arc::new(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([0, 0, 0, 255]),
        )));
        view.qr_job.result = Some(QrResult {
            code: qr::generate("Queued original value", CodeKind::Qr, "M", 128).unwrap(),
            asset: Asset::new(image::RgbaImage::from_pixel(
                1,
                1,
                image::Rgba([0, 0, 0, 255]),
            )),
        });
        view.qr_action.result = Some(PathBuf::from("generated-previous-code.png"));
        view.copy_job.result = Some(());
        view
    }

    fn assert_stale_exports_discarded(view: &ImageTools) {
        assert!(
            view.qr_code.is_none(),
            "edited QR parameters kept an exportable old code"
        );
        assert!(
            view.qr_preview.image.is_none(),
            "edited QR parameters kept the old preview"
        );
        assert!(
            view.qr_job.result.is_none(),
            "queued old generation can reappear"
        );
        assert!(
            view.qr_action.result.is_none(),
            "old save receipt remains attached to edited content"
        );
        assert!(
            view.copy_job.result.is_none(),
            "old copy receipt remains attached to edited content"
        );
    }

    #[test]
    fn verification2_qr_text_edit_discards_exportable_old_code_and_queued_generation() {
        let mut view = generated_view();
        let context = egui::Context::default();
        context.enable_accesskit();
        let shared = Shared::default();
        let nodes = frame(&mut view, &context, &shared, vec![]);
        let field = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::MultilineTextInput
                    && node.value() == Some("Generated original value"))
                .then_some(id)
            })
            .unwrap();
        frame(
            &mut view,
            &context,
            &shared,
            vec![action(
                field,
                egui::accesskit::Action::SetValue,
                Some(egui::accesskit::ActionData::Value(
                    "Generated changed value".into(),
                )),
            )],
        );
        assert_eq!(
            view.qr_text, "Generated changed value",
            "fixture must edit the real text control"
        );
        assert_stale_exports_discarded(&view);
        view.shutdown();
        assert!(shared.clipboard.snapshot("").unwrap().paused);
    }

    #[test]
    fn verification2_qr_type_edit_discards_exportable_old_code_and_queued_generation() {
        let mut view = generated_view();
        let context = egui::Context::default();
        context.enable_accesskit();
        let shared = Shared::default();
        let nodes = frame(&mut view, &context, &shared, vec![]);
        let field = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::ComboBox && node.value() == Some("QR code"))
                    .then_some(id)
            })
            .unwrap();
        frame(
            &mut view,
            &context,
            &shared,
            vec![action(field, egui::accesskit::Action::Click, None)],
        );
        // A newly opened popup first measures its contents in an invisible
        // sizing pass; its actual accessible options appear on the next frame.
        let nodes = frame(&mut view, &context, &shared, vec![]);
        let option = nodes
            .into_iter()
            .find_map(|(id, node)| {
                (node.role() == egui::accesskit::Role::Button && node.label() == Some("Code 128"))
                    .then_some(id)
            })
            .unwrap();
        frame(
            &mut view,
            &context,
            &shared,
            vec![action(option, egui::accesskit::Action::Click, None)],
        );
        assert_eq!(
            view.qr_kind, 1,
            "fixture must edit the real code-type combo"
        );
        assert_stale_exports_discarded(&view);
        view.shutdown();
        assert!(shared.clipboard.snapshot("").unwrap().paused);
    }
}

#[cfg(test)]
mod verification56_tests {
    use super::*;
    use egui::accesskit::{Action, ActionData, ActionRequest, Node, NodeId, Role, TreeId};
    use oxbox_core::{CoreError, jobs::CancelToken};
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    fn frame(
        view: &mut ImageTools,
        context: &egui::Context,
        id: &str,
        events: Vec<egui::Event>,
    ) -> Vec<(NodeId, Node)> {
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| view.show(ui, id, &Shared::default()),
        );
        output.textures_delta.clear();
        output.platform_output.accesskit_update.unwrap().nodes
    }
    fn action(target_node: NodeId, action: Action, data: Option<ActionData>) -> egui::Event {
        egui::Event::AccessKitActionRequest(ActionRequest {
            action,
            target_node,
            target_tree: TreeId::ROOT,
            data,
        })
    }
    fn input<'a>(view: &'a ImageTools, id: &str) -> &'a str {
        match id {
            "resize-images" => &view.resize_folder,
            "make-gif" => &view.gif_input,
            _ => &view.convert_input,
        }
    }
    fn generated(id: &str) -> ImageTools {
        let mut view = ImageTools {
            resize_folder: "Generated before folder".into(),
            gif_input: "Generated before video".into(),
            convert_input: "Generated before media".into(),
            ..Default::default()
        };
        let old = PathBuf::from("generated-previous-output");
        match id {
            "resize-images" => {
                view.resize_results = vec![old.clone()];
                view.resize_job.result = Some(vec![old]);
                view.resize_job.error = Some("Generated stale failure".into());
                view.resize_preview.image = Some(std::sync::Arc::new(image::RgbaImage::new(1, 1)));
                view.preview_job.result = Some(Asset::new(image::RgbaImage::new(1, 1)));
            }
            "make-gif" => {
                view.gif_result = Some(old.clone());
                view.gif_job.result = Some(old);
                view.gif_job.error = Some("Generated stale failure".into());
            }
            _ => {
                view.convert_result = Some(old.clone());
                view.convert_job.result = Some(old);
                view.convert_job.error = Some("Generated stale failure".into());
            }
        }
        view
    }
    fn outcomes_cleared(view: &ImageTools, id: &str) -> bool {
        match id {
            "resize-images" => {
                view.resize_results.is_empty()
                    && view.resize_job.result.is_none()
                    && view.resize_job.error.is_none()
                    && view.resize_preview.image.is_none()
                    && view.preview_job.result.is_none()
            }
            "make-gif" => {
                view.gif_result.is_none()
                    && view.gif_job.result.is_none()
                    && view.gif_job.error.is_none()
            }
            _ => {
                view.convert_result.is_none()
                    && view.convert_job.result.is_none()
                    && view.convert_job.error.is_none()
            }
        }
    }
    fn wait_job<T: Send + 'static>(job: &mut Task<T>, context: &egui::Context) {
        let (entered, receiver) = mpsc::sync_channel(1);
        assert!(job.start(context, move |cancel: CancelToken, _| {
            entered.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while cancel.check().is_ok() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(CoreError::Cancelled)
        }));
        receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    #[test]
    fn verification56_file_form_edits_discard_previous_and_queued_results() {
        let mut failures = Vec::new();
        for id in ["resize-images", "make-gif", "convert-audio-video"] {
            let mut view = generated(id);
            let context = egui::Context::default();
            context.enable_accesskit();
            let before = input(&view, id).to_string();
            let field = frame(&mut view, &context, id, vec![])
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == Role::TextInput && node.value() == Some(before.as_str()))
                        .then_some(id)
                })
                .unwrap();
            frame(
                &mut view,
                &context,
                id,
                vec![action(
                    field,
                    Action::SetValue,
                    Some(ActionData::Value("Generated after input".into())),
                )],
            );
            assert_eq!(
                input(&view, id),
                "Generated after input",
                "fixture must edit the actual form"
            );
            let cleared = outcomes_cleared(&view, id);
            view.shutdown();
            if !cleared {
                failures.push(id);
            }
        }
        assert!(
            failures.is_empty(),
            "edited forms retained old or queued outcomes: {failures:?}"
        );
    }
    #[test]
    fn verification56_busy_file_forms_preserve_admitted_inputs() {
        let mut failures = Vec::new();
        for id in ["resize-images", "make-gif", "convert-audio-video"] {
            let mut view = generated(id);
            let context = egui::Context::default();
            context.enable_accesskit();
            match id {
                "resize-images" => wait_job(&mut view.resize_job, &context),
                "make-gif" => wait_job(&mut view.gif_job, &context),
                _ => wait_job(&mut view.convert_job, &context),
            }
            let before = input(&view, id).to_string();
            let field = frame(&mut view, &context, id, vec![])
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == Role::TextInput && node.value() == Some(before.as_str()))
                        .then_some(id)
                })
                .unwrap();
            frame(
                &mut view,
                &context,
                id,
                vec![action(
                    field,
                    Action::SetValue,
                    Some(ActionData::Value("Generated different job".into())),
                )],
            );
            let after = input(&view, id).to_string();
            view.shutdown();
            if after != before {
                failures.push(id);
            }
        }
        assert!(
            failures.is_empty(),
            "busy forms relabeled admitted jobs: {failures:?}"
        );
    }
    #[test]
    fn verification56_new_failed_file_attempt_does_not_show_previous_success() {
        let mut failures = Vec::new();
        for id in ["resize-images", "make-gif", "convert-audio-video"] {
            let directory = tempfile::tempdir().unwrap();
            let missing = directory.path().join("generated-missing-input.mp4");
            let output = directory.path().join("generated-never-created-output");
            let mut view = ImageTools {
                resize_paths: vec![missing.clone()],
                resize_folder: directory.path().display().to_string(),
                resize_results: vec![PathBuf::from("generated-old-resize")],
                gif_input: missing.display().to_string(),
                gif_output: output.display().to_string(),
                gif_result: Some(PathBuf::from("generated-old.gif")),
                convert_input: missing.display().to_string(),
                convert_output: output.display().to_string(),
                convert_result: Some(PathBuf::from("generated-old.mp4")),
                ..Default::default()
            };
            let context = egui::Context::default();
            context.enable_accesskit();
            let label = match id {
                "resize-images" => "Resize images",
                "make-gif" => "Make GIF",
                _ => "Convert media",
            };
            let button = frame(&mut view, &context, id, vec![])
                .into_iter()
                .find_map(|(id, node)| {
                    (node.role() == Role::Button && node.label() == Some(label)).then_some(id)
                })
                .unwrap();
            frame(
                &mut view,
                &context,
                id,
                vec![action(button, Action::Click, None)],
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                view.tick(Some(&context));
                let running = match id {
                    "resize-images" => view.resize_job.running(),
                    "make-gif" => view.gif_job.running(),
                    _ => view.convert_job.running(),
                };
                if !running {
                    break;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            let failed = match id {
                "resize-images" => view.resize_job.error.is_some(),
                "make-gif" => view.gif_job.error.is_some(),
                _ => view.convert_job.error.is_some(),
            };
            let stale = match id {
                "resize-images" => !view.resize_results.is_empty(),
                "make-gif" => view.gif_result.is_some(),
                _ => view.convert_result.is_some(),
            };
            view.shutdown();
            assert!(
                failed,
                "fixture must fail on the generated missing input before any provider starts"
            );
            assert!(!output.exists());
            if stale {
                failures.push(id);
            }
        }
        assert!(
            failures.is_empty(),
            "new failed attempts still show old success: {failures:?}"
        );
    }
}

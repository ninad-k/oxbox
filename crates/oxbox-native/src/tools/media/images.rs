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
        let label = ui.label("Text or barcode value");
        ui.add(
            egui::TextEdit::multiline(&mut self.qr_text)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .char_limit(4096),
        )
        .labelled_by(label.id);
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
        widgets::folder(ui, "Destination folder", &mut self.resize_folder);
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
                format: [OutputFormat::Png, OutputFormat::Jpeg, OutputFormat::Webp][self.format],
                jpeg_quality: self.quality as u8,
            };
            self.resize_job.start(ui.ctx(), move |cancel, progress| {
                resize::resize_batch(&request, &cancel, progress)
            });
        }
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
        widgets::file(
            ui,
            "Input video",
            &mut self.gif_input,
            &["mp4", "webm", "mov", "mkv", "avi"],
        );
        ui.horizontal(|ui| {
            ui.label("Start (seconds)");
            ui.add(egui::DragValue::new(&mut self.gif_start).range(0.0..=604800.0));
        });
        ui.horizontal(|ui| {
            ui.label("Duration (seconds)");
            ui.add(egui::DragValue::new(&mut self.gif_seconds).range(0.1..=60.0));
        });
        widgets::number(ui, "Frame rate", &mut self.gif_fps, 1..=30);
        widgets::number(ui, "Width (pixels)", &mut self.gif_width, 16..=1280);
        widgets::output(
            ui,
            "New GIF output",
            &mut self.gif_output,
            "Oxbox-animation.gif",
        );
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
            self.gif_job.start(ui.ctx(), move |cancel, progress| {
                make_gif::make(&input, &output, &options, &cancel, progress)
            });
        }
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
        widgets::file(ui, "Input media", &mut self.convert_input, &[]);
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
        widgets::output(
            ui,
            "New converted file",
            &mut self.convert_output,
            &format!("Oxbox-converted.{}", format.extension()),
        );
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
            self.convert_job.start(ui.ctx(), move |cancel, progress| {
                convert_media::convert(&input, &output, format, &cancel, progress)
            });
        }
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

use super::{
    Shared,
    annotate::ImageEditor,
    widgets::{self, Asset, Preview},
};
use crate::{host::Host, jobs::Task};
use eframe::egui;
use oxbox_core::{
    CoreError, CoreResult,
    jobs::CancelToken,
    tools::screenshot::{self, CaptureKind, CaptureTarget, RgbaImage, Selection, TargetInfo},
};
use std::{path::PathBuf, sync::Arc, time::Duration};

struct Captured {
    path: Option<PathBuf>,
    asset: Asset,
    clipboard_error: Option<String>,
    review: bool,
}
struct Frozen {
    target: TargetInfo,
    monitor: usize,
    image: RgbaImage,
}
struct AreaFrame {
    id: egui::ViewportId,
    monitor: usize,
    image: Arc<RgbaImage>,
    texture: egui::TextureHandle,
    start: Option<egui::Pos2>,
}
struct AreaSession {
    frames: Vec<AreaFrame>,
    folder: Option<PathBuf>,
    copy: bool,
    review: bool,
}
pub struct CaptureUi {
    review: bool,
    editor: ImageEditor,
    mode: usize,
    delay: usize,
    selected: usize,
    folder: String,
    copy: bool,
    targets_job: Task<Vec<TargetInfo>>,
    targets: Vec<TargetInfo>,
    capture: Task<Captured>,
    freeze: Task<Vec<Frozen>>,
    copy_job: Task<()>,
    preview: Preview,
    path: Option<PathBuf>,
    clipboard_error: Option<String>,
    error: Option<String>,
    session: Option<AreaSession>,
    hidden: bool,
}
impl Default for CaptureUi {
    fn default() -> Self {
        Self {
            review: true,
            editor: ImageEditor::default(),
            mode: 0,
            delay: 0,
            selected: 0,
            folder: String::new(),
            copy: false,
            targets_job: Task::default(),
            targets: Vec::new(),
            capture: Task::default(),
            freeze: Task::default(),
            copy_job: Task::default(),
            preview: Preview::default(),
            path: None,
            clipboard_error: None,
            error: None,
            session: None,
            hidden: false,
        }
    }
}
fn finish(
    image: RgbaImage,
    folder: Option<PathBuf>,
    copy: bool,
    review: bool,
    shared_clipboard: Arc<oxbox_core::tools::clipboard_history::ClipboardService>,
    cancel: &CancelToken,
    quit: &CancelToken,
) -> CoreResult<Captured> {
    finish_capture_using(
        image,
        review,
        copy,
        cancel,
        quit,
        |image| {
            let folder = folder
                .map(Ok)
                .unwrap_or_else(screenshot::output_directory)?;
            screenshot::save_capture(image, &folder)
        },
        |image| shared_clipboard.copy_image(image),
    )
}

fn finish_capture_using(
    image: RgbaImage,
    review: bool,
    copy: bool,
    cancel: &CancelToken,
    quit: &CancelToken,
    save: impl FnOnce(&RgbaImage) -> CoreResult<PathBuf>,
    copy_image: impl FnOnce(&RgbaImage) -> CoreResult<()>,
) -> CoreResult<Captured> {
    cancel.check()?;
    quit.check()?;
    if review {
        return Ok(Captured {
            path: None,
            asset: Asset::new(image),
            clipboard_error: None,
            review: true,
        });
    }
    let path = save(&image)?;
    cancel.check()?;
    quit.check()?;
    let clipboard_error = if copy {
        copy_image(&image).err().map(|error| error.to_string())
    } else {
        None
    };
    Ok(Captured {
        path: Some(path),
        asset: Asset::new(image),
        clipboard_error,
        review: false,
    })
}
impl CaptureUi {
    pub fn show(&mut self, ui: &mut egui::Ui, host: &mut Host, shared: &Shared) {
        widgets::introduction(
            ui,
            "Capture a display, window or selected area. Review and annotate before exporting, or explicitly choose direct raw screenshot saving.",
        );
        let old = self.mode;
        widgets::choice(
            ui,
            "Capture type",
            &mut self.mode,
            &["Display", "Window", "Area selection"],
        );
        if old != self.mode {
            self.selected = 0;
        }
        if ui
            .add_enabled(
                !self.targets_job.running(),
                egui::Button::new("Find displays and windows"),
            )
            .clicked()
        {
            self.targets_job.start(ui.ctx(), |cancel, _| {
                cancel.check()?;
                screenshot::targets()
            });
        }
        self.targets_job.show(ui);
        let kind = if self.mode == 1 {
            CaptureKind::Window
        } else {
            CaptureKind::Monitor
        };
        let choices: Vec<_> = self
            .targets
            .iter()
            .filter(|target| target.kind == kind)
            .collect();
        if self.mode != 2 {
            egui::ComboBox::from_label("Capture target")
                .selected_text(
                    choices
                        .get(self.selected)
                        .map_or("Choose a capture target", |target| target.name.as_str()),
                )
                .show_ui(ui, |ui| {
                    for (index, target) in choices.iter().enumerate() {
                        ui.selectable_value(
                            &mut self.selected,
                            index,
                            format!("{} · {} × {}", target.name, target.width, target.height),
                        );
                    }
                });
        } else {
            widgets::note(
                ui,
                "A frozen preview opens on each display. Drag within one display; Escape cancels all selectors. Multi-monitor and HiDPI crop coordinates use the original pixels.",
            );
        }
        widgets::choice(
            ui,
            "Delay",
            &mut self.delay,
            &["None", "3 seconds", "5 seconds", "10 seconds"],
        );
        if ui
            .checkbox(
                &mut self.review,
                "Review and annotate before exporting (memory-only capture)",
            )
            .changed()
            && self.review
        {
            self.copy = false;
        }
        ui.add_enabled_ui(!self.review, |ui| {
            widgets::folder(
                ui,
                "Direct-save folder (blank uses Pictures/Oxbox)",
                &mut self.folder,
            );
            ui.checkbox(
                &mut self.copy,
                "Copy raw direct-save screenshot to clipboard",
            );
        });
        if !self.review {
            widgets::note(
                ui,
                "Direct-save mode exports the unedited screenshot immediately. Later redaction does not change earlier files or clipboard copies.",
            );
        }
        let busy = self.capture.running() || self.freeze.running() || self.session.is_some();
        if ui
            .add_enabled(
                !busy && !choices.is_empty(),
                egui::Button::new(match (self.mode == 2, self.review) {
                    (true, true) => "Select area for review…",
                    (true, false) => "Select area and save…",
                    (false, true) => "Capture for review",
                    (false, false) => "Save screenshot",
                }),
            )
            .clicked()
        {
            self.editor.clear();
            self.preview = Preview::default();
            self.path = None;
            self.clipboard_error = None;
            self.error = None;
            let delay = [0, 3, 5, 10][self.delay];
            let folder = if self.folder.trim().is_empty() {
                None
            } else {
                Some(PathBuf::from(&self.folder))
            };
            host.hide(ui.ctx());
            self.hidden = true;
            if self.mode == 2 {
                let targets: Vec<_> = choices.iter().map(|target| (*target).clone()).collect();
                let native: Vec<_> = host
                    .monitors
                    .iter()
                    .map(|monitor| TargetInfo {
                        kind: CaptureKind::Monitor,
                        id: monitor.index as u32,
                        name: monitor.name.clone(),
                        x: monitor.x,
                        y: monitor.y,
                        width: monitor.width,
                        height: monitor.height,
                        scale_factor: monitor.scale as f32,
                    })
                    .collect();
                let quit = shared.quit.clone();
                self.freeze.start(ui.ctx(),move|cancel,_|screenshot::capture_after_delay(Duration::from_millis(delay*1000+250),&cancel,||{
                    let mut frames=Vec::new();let mut bytes=0_u64;
                    for target in targets{
                        quit.check()?;cancel.check()?;
                        let monitor=screenshot::native_monitor_index(&target,&native,cfg!(any(target_os="macos",target_os="linux")))? as usize;
                        let expected=u64::from(target.width).saturating_mul(u64::from(target.height)).saturating_mul(4);
                        if expected>256*1024*1024{return Err(CoreError::InvalidInput("display exceeds the capture memory limit".into()));}
                        let image=screenshot::capture_image(CaptureTarget{kind:CaptureKind::Monitor,id:target.id})?;
                        quit.check()?;cancel.check()?;bytes=bytes.saturating_add(image.as_raw().len() as u64);
                        if bytes>256*1024*1024{return Err(CoreError::InvalidInput("area capture is limited to 256 MiB of display pixels; capture one display instead".into()));}
                        frames.push(Frozen{target,monitor,image});
                    }
                    if frames.is_empty(){return Err(CoreError::InvalidInput("no capturable display is available".into()));}Ok(frames)
                }));
                // The session options belong to this explicit capture, even if
                // the launcher is brought back and form fields are edited.
                self.session = Some(AreaSession {
                    frames: Vec::new(),
                    folder,
                    copy: self.copy,
                    review: self.review,
                });
            } else if let Some(target) = choices.get(self.selected) {
                let target = CaptureTarget {
                    kind: target.kind,
                    id: target.id,
                };
                let copy = self.copy;
                let review = self.review;
                let clipboard = shared.clipboard.clone();
                let quit = shared.quit.clone();
                self.capture.start(ui.ctx(), move |cancel, _| {
                    screenshot::capture_after_delay(
                        Duration::from_millis(delay * 1000 + 250),
                        &cancel,
                        || {
                            quit.check()?;
                            let image = screenshot::capture_image(target)?;
                            finish(image, folder, copy, review, clipboard, &cancel, &quit)
                        },
                    )
                });
            }
        }
        self.capture.show(ui);
        self.freeze.show(ui);
        self.copy_job.show(ui);
        if self.copy_job.result.is_some() {
            widgets::note(ui, "Image copied to the clipboard.");
        }
        widgets::error(ui, &self.error);
        if busy && ui.button("Cancel capture").clicked() {
            self.capture.cancel();
            self.freeze.cancel();
            self.close_session(ui.ctx());
            self.hidden = false;
            host.restore(ui.ctx());
        }
        if let Some(path) = &self.path {
            widgets::note(ui, format!("Saved {}", path.display()));
            if ui.button("Open output folder").clicked()
                && let Some(parent) = path.parent()
            {
                self.error = widgets::open_folder(parent).err();
            }
        }
        if let Some(error) = &self.clipboard_error {
            widgets::note(
                ui,
                format!("The PNG was saved. Clipboard copy failed: {error}"),
            );
        }
        if !self.editor.active() {
            self.preview.show(ui);
        }
        if let Some(image) = &self.preview.image
            && !self.editor.active()
            && ui
                .add_enabled(
                    !self.copy_job.running(),
                    egui::Button::new("Copy captured image"),
                )
                .clicked()
        {
            let image = image.clone();
            let clipboard = shared.clipboard.clone();
            self.copy_job.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                clipboard.copy_image(&image)
            });
        }
        if !self.editor.active()
            && let Some(image) = self.preview.image.clone()
            && ui.button("Annotate this captured image").clicked()
        {
            self.copy_job.abandon();
            self.editor.begin(ui.ctx(), image);
        }
        self.editor.show(ui, shared);
        widgets::note(
            ui,
            "Review captures stay in memory until an explicit flattened Save or Copy. Clipboard history stays paused unless separately enabled. Saved originals and earlier clipboard copies are not erased by redaction; existing files are never overwritten.",
        );
    }

    pub fn tick(&mut self, ctx: Option<&egui::Context>, host: &mut Host, shared: &Shared) {
        self.targets_job.poll();
        self.capture.poll();
        self.freeze.poll();
        self.copy_job.poll();
        self.editor.tick(ctx);
        if let Some(targets) = self.targets_job.result.take() {
            self.targets = targets;
            self.selected = 0;
        }
        if let Some(result) = self.capture.result.take() {
            self.path = result.path;
            self.clipboard_error = result.clipboard_error;
            if let Some(ctx) = ctx {
                if result.review {
                    self.preview = Preview::default();
                    self.editor.begin_asset(ctx, result.asset);
                } else {
                    self.editor.clear();
                    self.preview.set(ctx, result.asset);
                }
            }
        }
        if let Some(frames) = self.freeze.result.take()
            && let Some(ctx) = ctx
        {
            let maximum = ctx.input(|input| input.max_texture_side).max(1) as u32;
            let mut area_frames = Vec::new();
            for frozen in frames {
                let image = Arc::new(frozen.image);
                let thumbnail;
                let visible = if image.width() > maximum || image.height() > maximum {
                    thumbnail = image::imageops::thumbnail(image.as_ref(), maximum, maximum);
                    &thumbnail
                } else {
                    image.as_ref()
                };
                let texture = ctx.load_texture(
                    format!("frozen display {}", frozen.target.id),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [visible.width() as usize, visible.height() as usize],
                        visible.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                );
                area_frames.push(AreaFrame {
                    id: egui::ViewportId::from_hash_of(("oxbox capture", frozen.target.id)),
                    monitor: frozen.monitor,
                    image,
                    texture,
                    start: None,
                });
            }
            if let Some(session) = &mut self.session {
                session.frames = area_frames;
            }
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.frames.is_empty())
            && !self.freeze.running()
        {
            self.session = None;
        }
        if self.hidden
            && !self.capture.running()
            && !self.freeze.running()
            && self.session.is_none()
        {
            self.hidden = false;
            if shared.quit.check().is_ok()
                && let Some(ctx) = ctx
            {
                host.restore(ctx);
            }
        }
        if (self.capture.running() || self.freeze.running() || self.session.is_some())
            && let Some(ctx) = ctx
        {
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::RequestPaintWhileHidden,
            );
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }

    pub fn overlays(&mut self, ui: &mut egui::Ui, host: &mut Host, shared: &Shared) {
        let Some(session) = &mut self.session else {
            return;
        };
        let mut selected = None;
        let mut cancelled = false;
        for (index, frame) in session.frames.iter_mut().enumerate() {
            let context = ui.ctx().clone();
            let builder = egui::ViewportBuilder::default()
                .with_title("Oxbox area selection")
                .with_monitor(frame.monitor)
                .with_fullscreen(true)
                .with_decorations(false)
                .with_resizable(false)
                .with_taskbar(false)
                .with_window_level(egui::WindowLevel::AlwaysOnTop);
            context.show_viewport_immediate(frame.id, builder, |ui, class| {
                if class == egui::ViewportClass::EmbeddedWindow {
                    cancelled = true;
                    return;
                }
                if ui.input(|input| {
                    input.key_pressed(egui::Key::Escape) || input.viewport().close_requested()
                }) {
                    cancelled = true;
                    return;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        let rect = ui.max_rect();
                        ui.painter().image(
                            frame.texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        let response = ui.interact(
                            rect,
                            ui.scope_id().with("selection surface"),
                            egui::Sense::drag(),
                        );
                        if response.drag_started() {
                            frame.start = ui.input(|input| input.pointer.press_origin());
                        }
                        if let Some(start) = frame.start
                            && let Some(end) = response.interact_pointer_pos()
                        {
                            let area = egui::Rect::from_two_pos(start, end).intersect(rect);
                            ui.painter().rect_filled(
                                area,
                                egui::CornerRadius::ZERO,
                                ui.visuals().selection.bg_fill.linear_multiply(0.25),
                            );
                            ui.painter().rect_stroke(
                                area,
                                egui::CornerRadius::ZERO,
                                ui.visuals().selection.stroke,
                                egui::StrokeKind::Inside,
                            );
                            if response.drag_stopped()
                                && area.width() >= 1.0
                                && area.height() >= 1.0
                            {
                                // Both axes use the exact painted image bounds;
                                // the existing core crop applies outward rounding.
                                selected = Some((
                                    index,
                                    Selection {
                                        x: f64::from((area.min.x - rect.min.x) / rect.width())
                                            * f64::from(frame.image.width()),
                                        y: f64::from((area.min.y - rect.min.y) / rect.height())
                                            * f64::from(frame.image.height()),
                                        width: f64::from(area.width() / rect.width())
                                            * f64::from(frame.image.width()),
                                        height: f64::from(area.height() / rect.height())
                                            * f64::from(frame.image.height()),
                                    },
                                ));
                            }
                        }
                        let badge = egui::Rect::from_min_size(
                            rect.min + egui::vec2(16.0, 16.0),
                            egui::vec2(360.0, 32.0),
                        );
                        ui.painter().rect_filled(
                            badge,
                            ui.visuals().widgets.noninteractive.corner_radius,
                            ui.visuals().panel_fill,
                        );
                        ui.painter().text(
                            badge.center(),
                            egui::Align2::CENTER_CENTER,
                            "Drag an area · Escape cancels",
                            egui::FontId::proportional(16.0),
                            ui.visuals().text_color(),
                        );
                    });
            });
        }
        if cancelled {
            self.close_session(ui.ctx());
            self.freeze.cancel();
            self.hidden = false;
            host.restore(ui.ctx());
            return;
        }
        if let Some((index, selection)) = selected
            && let Some(session) = self.session.take()
        {
            for frame in &session.frames {
                ui.ctx()
                    .send_viewport_cmd_to(frame.id, egui::ViewportCommand::Close);
            }
            let image = session.frames[index].image.clone();
            let clipboard = shared.clipboard.clone();
            let quit = shared.quit.clone();
            let folder = session.folder;
            let copy = session.copy;
            let review = session.review;
            self.capture.start(ui.ctx(), move |cancel, _| {
                cancel.check()?;
                quit.check()?;
                let image = screenshot::crop_selection(&image, selection, 1.0)?;
                finish(image, folder, copy, review, clipboard, &cancel, &quit)
            });
        }
    }
    fn close_session(&mut self, ctx: &egui::Context) {
        if let Some(session) = self.session.take() {
            for frame in session.frames {
                ctx.send_viewport_cmd_to(frame.id, egui::ViewportCommand::Close);
            }
        }
    }
    pub fn shutdown(&mut self, ctx: Option<&egui::Context>) {
        self.editor.shutdown();
        self.targets_job.shutdown();
        self.capture.shutdown();
        self.freeze.shutdown();
        self.copy_job.shutdown();
        if let Some(ctx) = ctx {
            self.close_session(ctx);
        }
        self.preview = Preview::default();
    }
    pub fn leave(&mut self, ctx: Option<&egui::Context>) {
        self.editor.clear();
        self.targets_job.abandon();
        self.capture.abandon();
        self.freeze.abandon();
        self.copy_job.abandon();
        if let Some(ctx) = ctx {
            self.close_session(ctx);
        }
        self.preview = Preview::default();
        self.path = None;
        self.clipboard_error = None;
        self.error = None;
        // Tick will restore a hidden launcher after the cancelled worker joins.
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn screenshot_review_dispatch_never_saves_or_copies_unreviewed_pixels() {
        let saved = Cell::new(0);
        let copied = Cell::new(0);
        let result = finish_capture_using(
            RgbaImage::from_pixel(2, 2, image::Rgba([99, 88, 77, 255])),
            true,
            true,
            &CancelToken::default(),
            &CancelToken::default(),
            |_| {
                saved.set(saved.get() + 1);
                Ok(PathBuf::from("generated-original.png"))
            },
            |_| {
                copied.set(copied.get() + 1);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(saved.get(), 0);
        assert_eq!(copied.get(), 0);
        assert!(result.path.is_none());
        assert!(result.review);
        assert!(
            CaptureUi::default().review,
            "review before export must be the default"
        );
    }

    #[test]
    fn direct_capture_mode_keeps_a_saved_path_when_explicit_clipboard_copy_fails() {
        let saved = Cell::new(0);
        let copied = Cell::new(0);
        let result = finish_capture_using(
            RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255])),
            false,
            true,
            &CancelToken::default(),
            &CancelToken::default(),
            |_| {
                saved.set(saved.get() + 1);
                Ok(PathBuf::from("generated-direct.png"))
            },
            |_| {
                copied.set(copied.get() + 1);
                Err(CoreError::InvalidInput(
                    "Generated clipboard refusal".into(),
                ))
            },
        )
        .unwrap();
        assert_eq!((saved.get(), copied.get()), (1, 1));
        assert_eq!(result.path, Some(PathBuf::from("generated-direct.png")));
        assert!(
            result
                .clipboard_error
                .unwrap()
                .contains("Generated clipboard refusal")
        );
        assert!(!result.review);
    }
}

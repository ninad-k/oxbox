//! Native capture, media and clipboard views. No embedded browser or startup devices.
mod capture;
mod clipboard;
mod dispatch;
mod images;
mod recording;
mod vision;
mod widgets;

use crate::host::Host;
use eframe::egui;
use oxbox_core::{
    CoreResult,
    jobs::CancelToken,
    tools::{
        clipboard_history::ClipboardService,
        media_provider::{Recorder, RecordingStatus},
    },
};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(super) struct Shared {
    clipboard: Arc<ClipboardService>,
    recorder: Arc<Recorder>,
    recording_dispatch: dispatch::RecordingDispatch,
    quit: CancelToken,
}

/// Thread-safe tray recovery. No operation requires the main UI to be pumped.
#[derive(Clone)]
pub struct MediaRecovery {
    shared: Shared,
}
impl MediaRecovery {
    pub fn invalidate_pending(&self) -> u64 {
        self.shared.recording_dispatch.invalidate_pending()
    }
    pub fn request_shutdown(&self) -> u64 {
        self.shared.quit.cancel();
        let _ = self.shared.clipboard.set_paused(true);
        self.invalidate_pending()
    }
    pub fn stop_recording(&self) -> CoreResult<()> {
        self.shared.recording_dispatch.stop(|| {
            let status = self.shared.recorder.status()?;
            if status.active {
                self.shared.recorder.stop(&status.id)?;
            }
            Ok(())
        })
    }
    pub fn shutdown(&self) -> CoreResult<()> {
        self.request_shutdown();
        self.shared.clipboard.shutdown();
        self.shared.recorder.shutdown();
        Ok(())
    }
}

#[derive(Default)]
pub struct MediaTools {
    shared: Shared,
    capture: capture::CaptureUi,
    clipboard: clipboard::ClipboardUi,
    images: images::ImageTools,
    recording: recording::RecordingUi,
    vision: vision::VisionUi,
    context: Option<egui::Context>,
    stopped: bool,
}

impl MediaTools {
    pub fn recovery_handle(&self) -> MediaRecovery {
        MediaRecovery {
            shared: self.shared.clone(),
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, host: &mut Host) -> bool {
        self.context = Some(ui.ctx().clone());
        let handled = matches!(
            id,
            "screenshot"
                | "record-screen"
                | "take-photo"
                | "take-video"
                | "resize-images"
                | "qr-code"
                | "mute-microphone"
                | "record-audio"
                | "make-gif"
                | "convert-audio-video"
                | "barcode-reader"
                | "measure-color"
                | "clipboard-history"
                | "recognize-text"
        );
        if !handled {
            return false;
        }
        ui.push_id(id, |ui| match id {
            "screenshot" => self.capture.show(ui, host, &self.shared),
            "record-screen" | "take-video" | "record-audio" | "take-photo" | "mute-microphone" => {
                self.recording.show(ui, id, host, &self.shared)
            }
            "resize-images" | "qr-code" | "make-gif" | "convert-audio-video" => {
                self.images.show(ui, id, &self.shared)
            }
            "clipboard-history" => self.clipboard.show(ui, &self.shared),
            _ => self.vision.show(ui, id, host, &self.shared),
        });
        true
    }

    pub fn tick(&mut self, host: &mut Host) {
        if self.stopped {
            return;
        }
        self.capture.tick(self.context.as_ref(), host, &self.shared);
        self.recording
            .tick(self.context.as_ref(), host, &self.shared);
        self.images.tick(self.context.as_ref());
        self.vision.tick(self.context.as_ref(), host);
        self.clipboard.tick();
    }

    /// Render selectors independently of the currently selected tool.
    pub fn overlays(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        self.context = Some(ui.ctx().clone());
        self.capture.overlays(ui, host, &self.shared);
    }

    pub fn recording_status(&self) -> CoreResult<RecordingStatus> {
        self.shared.recorder.status()
    }
    pub fn stop_recording(&mut self, ctx: &egui::Context) {
        self.recording.stop(ctx, &self.shared);
    }
    pub fn recording_banner(&mut self, ui: &mut egui::Ui) {
        self.recording.banner(ui, &self.shared);
    }

    pub fn leave_tool(&mut self, id: &str) {
        match id {
            "screenshot" => self.capture.leave(self.context.as_ref()),
            "clipboard-history" => self.clipboard.leave(),
            "record-screen" | "record-audio" | "take-video" | "take-photo" | "mute-microphone" => {
                self.recording.leave(id)
            }
            "qr-code" | "resize-images" | "make-gif" | "convert-audio-video" => {
                self.images.leave(id)
            }
            "recognize-text" | "barcode-reader" | "measure-color" => self.vision.leave(id),
            _ => {}
        }
    }

    pub fn shutdown(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        self.recovery_handle().request_shutdown();
        // Block late starts/copies and stop privacy-sensitive controllers
        // before joining unrelated file conversion or image workers.
        self.shared.clipboard.shutdown();
        self.shared.recorder.shutdown();
        self.capture.shutdown(self.context.as_ref());
        self.recording.shutdown();
        self.images.shutdown();
        self.vision.shutdown();
        self.clipboard.shutdown();
    }
}
impl Drop for MediaTools {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_media_forms_render_without_devices_or_clipboard_monitoring() {
        let directory = tempfile::tempdir().unwrap();
        let mut host = Host::new(directory.path().to_path_buf()).unwrap();
        let mut tools = MediaTools::default();
        let context = egui::Context::default();
        for id in [
            "screenshot",
            "record-screen",
            "take-photo",
            "take-video",
            "resize-images",
            "qr-code",
            "mute-microphone",
            "record-audio",
            "make-gif",
            "convert-audio-video",
            "barcode-reader",
            "measure-color",
            "clipboard-history",
            "recognize-text",
        ] {
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                assert!(tools.show(ui, id, &mut host), "missing native form: {id}");
            });
            // No renderer is present in this headless test; explicitly consume
            // the texture delta that a real eframe renderer uploads each frame.
            output.textures_delta.clear();
            tools.leave_tool(id);
        }
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            assert!(!tools.show(ui, "unknown-tool", &mut host));
        });
        output.textures_delta.clear();
        assert!(!tools.recording_status().unwrap().active);
        assert!(tools.shared.clipboard.snapshot("").unwrap().paused);
        tools.shutdown();
        host.shutdown();
        assert!(
            std::fs::read_dir(directory.path())
                .unwrap()
                .next()
                .is_none()
        );
    }
}

//! Parent-owned synchronous native dialogs and focus restoration.
use eframe::egui;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::sync::Arc;

trait NativeWindow: HasWindowHandle + HasDisplayHandle + Send + Sync {}
impl<T: HasWindowHandle + HasDisplayHandle + Send + Sync> NativeWindow for T {}

#[derive(Clone)]
struct Owner(Arc<dyn NativeWindow>);

fn owner_id() -> egui::Id {
    egui::Id::unique("oxbox-native-dialog-owner")
}

/// Retain eframe's actual window, without reconstructing a raw native handle.
/// Called on the UI thread when eframe provides its current root window.
pub fn attach_parent<W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static>(
    context: &egui::Context,
    window: Arc<W>,
) {
    context.data_mut(|data| data.insert_temp(owner_id(), Owner(window)));
}

pub fn detach_parent(context: &egui::Context) {
    context.data_mut(|data| data.remove::<Owner>(owner_id()));
}

struct RestoreFocus(egui::Context);
impl Drop for RestoreFocus {
    fn drop(&mut self) {
        self.0
            .send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
        self.0.request_repaint();
    }
}

fn with_parent<R>(context: &egui::Context, show: impl FnOnce(Option<&dyn NativeWindow>) -> R) -> R {
    let owner = context.data(|data| data.get_temp::<Owner>(owner_id()));
    let _focus = RestoreFocus(context.clone());
    // rfd copies raw handles into its builder. This Arc remains alive across
    // the complete synchronous show, even if the Context owner is detached.
    // All calls stay on the UI thread; no raw handles are stored in workers.
    show(owner.as_ref().map(|owner| owner.0.as_ref()))
}

/// A file-dialog builder whose synchronous operation retains its native parent.
pub struct FileDialog {
    context: egui::Context,
    inner: rfd::FileDialog,
}
pub fn file(context: &egui::Context) -> FileDialog {
    FileDialog {
        context: context.clone(),
        inner: rfd::FileDialog::new(),
    }
}
impl FileDialog {
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.inner = self.inner.set_title(title);
        self
    }
    pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
        self.inner = self.inner.set_file_name(name);
        self
    }
    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[impl ToString]) -> Self {
        self.inner = self.inner.add_filter(name, extensions);
        self
    }
    fn run<R>(self, show: impl FnOnce(rfd::FileDialog) -> R) -> R {
        with_parent(&self.context, |owner| {
            let dialog = match owner {
                Some(owner) => self.inner.set_parent(owner),
                None => self.inner,
            };
            show(dialog)
        })
    }
    pub fn pick_file(self) -> Option<std::path::PathBuf> {
        self.run(rfd::FileDialog::pick_file)
    }
    pub fn pick_files(self) -> Option<Vec<std::path::PathBuf>> {
        self.run(rfd::FileDialog::pick_files)
    }
    pub fn pick_folder(self) -> Option<std::path::PathBuf> {
        self.run(rfd::FileDialog::pick_folder)
    }
    pub fn pick_folders(self) -> Option<Vec<std::path::PathBuf>> {
        self.run(rfd::FileDialog::pick_folders)
    }
    pub fn save_file(self) -> Option<std::path::PathBuf> {
        self.run(rfd::FileDialog::save_file)
    }
}

/// A confirmation builder with the same parent/focus lifetime as file dialogs.
pub struct MessageDialog {
    context: egui::Context,
    inner: rfd::MessageDialog,
}
pub fn message(context: &egui::Context) -> MessageDialog {
    MessageDialog {
        context: context.clone(),
        inner: rfd::MessageDialog::new(),
    }
}
impl MessageDialog {
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.inner = self.inner.set_title(title);
        self
    }
    pub fn set_description(mut self, description: impl Into<String>) -> Self {
        self.inner = self.inner.set_description(description);
        self
    }
    pub fn set_level(mut self, level: rfd::MessageLevel) -> Self {
        self.inner = self.inner.set_level(level);
        self
    }
    pub fn set_buttons(mut self, buttons: rfd::MessageButtons) -> Self {
        self.inner = self.inner.set_buttons(buttons);
        self
    }
    pub fn show(self) -> rfd::MessageDialogResult {
        with_parent(&self.context, |owner| {
            let dialog = match owner {
                Some(owner) => self.inner.set_parent(owner),
                None => self.inner,
            };
            dialog.show()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    struct FakeWindow(Arc<AtomicBool>);
    impl HasWindowHandle for FakeWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }
    impl HasDisplayHandle for FakeWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }
    impl Drop for FakeWindow {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    #[test]
    fn dialog_parent_remains_owned_until_synchronous_completion() {
        let context = egui::Context::default();
        let dropped = Arc::new(AtomicBool::new(false));
        let parent = Arc::new(FakeWindow(dropped.clone()));
        attach_parent(&context, parent.clone());
        drop(parent);
        let accepted = with_parent(&context, |owner| {
            assert!(owner.is_some());
            detach_parent(&context);
            assert!(!dropped.load(Ordering::Acquire));
            Some(7)
        });
        assert_eq!(accepted, Some(7));
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn both_accept_and_cancel_request_root_focus_without_opening_a_window() {
        for accepted in [true, false] {
            let context = egui::Context::default();
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                let value = with_parent(ui.ctx(), |owner| {
                    assert!(owner.is_none());
                    accepted.then_some(9)
                });
                assert_eq!(value, accepted.then_some(9));
            });
            assert!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .iter()
                    .any(|command| matches!(command, egui::ViewportCommand::Focus))
            );
            output.textures_delta.clear();
        }
    }
}

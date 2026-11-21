//! Native lifecycle, local persistence and OS notification adapters.
pub mod dialogs;
use eframe::egui;
use oxbox_core::{CoreError, CoreResult, preferences};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct NativeMonitor {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}
#[derive(Clone)]
pub struct Notifier {
    sender: mpsc::SyncSender<(String, String)>,
    errors: Arc<Mutex<VecDeque<String>>>,
    stopping: Arc<AtomicBool>,
}
impl Notifier {
    /// Stop new/queued notification delivery without waiting for the UI thread.
    pub fn request_stop(&self) {
        self.stopping.store(true, Ordering::Release);
    }
    pub fn notify(&self, title: impl Into<String>, body: impl Into<String>) {
        if self.stopping.load(Ordering::Acquire) {
            return;
        }
        if self.sender.try_send((title.into(), body.into())).is_err() {
            self.error("A notification could not be queued");
        }
    }
    fn error(&self, message: &str) {
        if let Ok(mut errors) = self.errors.lock() {
            if errors.len() >= 8 {
                errors.pop_front();
            }
            errors.push_back(message.into());
        }
    }
}
pub struct Host {
    directory: PathBuf,
    pub notifier: Notifier,
    pub main_window_id: u64,
    pub monitors: Vec<NativeMonitor>,
    pub visible: Arc<AtomicBool>,
    notification_worker: Option<JoinHandle<()>>,
}
impl Host {
    pub fn new(directory: PathBuf) -> CoreResult<Self> {
        std::fs::create_dir_all(&directory)?;
        let (sender, receiver) = mpsc::sync_channel::<(String, String)>(32);
        let notifier = Notifier {
            sender,
            errors: Arc::new(Mutex::new(VecDeque::new())),
            stopping: Arc::new(AtomicBool::new(false)),
        };
        let worker_notifier = notifier.clone();
        let worker = std::thread::Builder::new()
            .name("oxbox-notifications".into())
            .spawn(move || {
                while !worker_notifier.stopping.load(Ordering::Acquire) {
                    match receiver.recv_timeout(Duration::from_millis(100)) {
                        Ok((title, body)) => {
                            if !worker_notifier.stopping.load(Ordering::Acquire)
                                && let Err(error) = native_notification(&title, &body)
                            {
                                worker_notifier
                                    .error(&format!("Notification was not delivered: {error}"));
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            directory,
            notifier,
            main_window_id: 0,
            monitors: Vec::new(),
            visible: Arc::new(AtomicBool::new(true)),
            notification_worker: Some(worker),
        })
    }
    pub fn data_dir(&self) -> &Path {
        &self.directory
    }
    pub fn load<T: DeserializeOwned>(&self, name: &str) -> CoreResult<Option<T>> {
        preferences::load(&self.directory, name)
    }
    pub fn save<T: Serialize>(&self, name: &str, value: &T) -> CoreResult<()> {
        save_at(&self.directory, name, value)
    }
    pub fn load_key<T: DeserializeOwned>(&self, name: &str, key: &str) -> CoreResult<Option<T>> {
        let value: Option<serde_json::Value> = self.load(name)?;
        value
            .and_then(|value| value.get(key).cloned())
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| {
                CoreError::InvalidInput(
                    "Saved preferences are invalid; original data was preserved".into(),
                )
            })
    }
    pub fn save_key<T: Serialize>(&self, name: &str, key: &str, value: &T) -> CoreResult<()> {
        preferences::write_key(&self.directory, name, key, value)
    }
    pub fn hide(&self, ctx: &egui::Context) {
        self.visible.store(false, Ordering::Release);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }
    pub fn restore(&self, ctx: &egui::Context) {
        self.visible.store(true, Ordering::Release);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
    pub fn notification_errors(&self) -> Vec<String> {
        self.notifier
            .errors
            .lock()
            .map(|mut values| values.drain(..).collect())
            .unwrap_or_default()
    }
    pub fn shutdown(&mut self) {
        self.notifier.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.notification_worker.take() {
            let _ = worker.join();
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.shutdown();
    }
}
pub fn save_at<T: Serialize>(directory: &Path, name: &str, value: &T) -> CoreResult<()> {
    preferences::save(directory, name, value)
}
pub fn default_data_dir() -> CoreResult<PathBuf> {
    dirs::data_dir()
        .map(|path| path.join("dev.oxbox.desktop"))
        .ok_or_else(|| {
            CoreError::InvalidInput("The local app configuration directory is unavailable".into())
        })
}
fn native_notification(title: &str, body: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        tauri_winrt_notification::Toast::new("dev.oxbox.desktop")
            .title(title)
            .text1(body)
            .show()
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        mac_notification_sys::send_notification(title, None, body, None)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        notify_rust::Notification::new()
            .appname("Oxbox")
            .summary(title)
            .body(body)
            .show()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

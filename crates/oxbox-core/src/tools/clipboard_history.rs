//! Bounded, memory-only clipboard history.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::{CoreError, CoreResult};

pub const MAX_ITEMS: usize = 200;
pub const MAX_BYTES: usize = 50 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub enum Content {
    Text(String),
    Image {
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    },
}

impl Content {
    fn size(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Image { rgba, .. } => rgba.len(),
        }
    }

    pub fn validate(&self) -> CoreResult<()> {
        if let Self::Image {
            width,
            height,
            rgba,
        } = self
            && (*width == 0
                || *height == 0
                || width
                    .checked_mul(*height)
                    .and_then(|count| count.checked_mul(4))
                    != Some(rgba.len()))
        {
            return Err(CoreError::InvalidInput(
                "invalid clipboard image dimensions".into(),
            ));
        }
        Ok(())
    }

    fn preview(&self) -> String {
        match self {
            Self::Text(text) => text.chars().take(500).collect(),
            Self::Image { width, height, .. } => format!("Image · {width} × {height}"),
        }
    }

    fn fingerprint(&self) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        match self {
            Self::Text(text) => {
                hasher.update(&[0]);
                hasher.update(text.as_bytes());
            }
            Self::Image {
                width,
                height,
                rgba,
            } => {
                hasher.update(&[1]);
                hasher.update(&width.to_le_bytes());
                hasher.update(&height.to_le_bytes());
                hasher.update(rgba);
            }
        }
        hasher.finalize()
    }
}

pub struct Entry {
    pub id: u64,
    pub pinned: bool,
    pub captured_at: u64,
    content: Content,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: u64,
    pub pinned: bool,
    pub captured_at: u64,
    pub kind: &'static str,
    pub preview: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub paused: bool,
    pub bytes: usize,
    pub total_items: usize,
    pub items: Vec<EntrySummary>,
    pub error: Option<String>,
}

pub struct History {
    items: Vec<Entry>,
    max_items: usize,
    max_bytes: usize,
    bytes: usize,
    next_id: u64,
    paused: bool,
    last_observed: Option<blake3::Hash>,
}

impl Default for History {
    fn default() -> Self {
        let mut history = Self::new(MAX_ITEMS, MAX_BYTES);
        history.paused = true;
        history
    }
}

impl History {
    pub fn new(max_items: usize, max_bytes: usize) -> Self {
        Self {
            items: Vec::new(),
            max_items: max_items.min(MAX_ITEMS),
            max_bytes: max_bytes.min(MAX_BYTES),
            bytes: 0,
            next_id: 1,
            paused: false,
            last_observed: None,
        }
    }

    pub fn items(&self) -> &[Entry] {
        &self.items
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn push(&mut self, content: Content, captured_at: u64) -> CoreResult<bool> {
        content.validate()?;
        let bytes = content.size();
        if self.paused || bytes == 0 || bytes > self.max_bytes || self.max_items == 0 {
            return Ok(false);
        }
        let fingerprint = content.fingerprint();
        if self.last_observed == Some(fingerprint) {
            return Ok(false);
        }
        // Keep the last observed digest across removal/clear so the polling loop
        // cannot immediately recreate an entry the user explicitly removed.
        self.last_observed = Some(fingerprint);
        if self.items.iter().any(|entry| entry.content == content) {
            return Ok(false);
        }
        let pinned_count = self.items.iter().filter(|entry| entry.pinned).count();
        let pinned_bytes: usize = self
            .items
            .iter()
            .filter(|entry| entry.pinned)
            .map(|entry| entry.content.size())
            .sum();
        if pinned_count >= self.max_items || pinned_bytes + bytes > self.max_bytes {
            return Ok(false);
        }
        while self.items.len() >= self.max_items || self.bytes + bytes > self.max_bytes {
            let Some(index) = self.items.iter().rposition(|entry| !entry.pinned) else {
                return Ok(false);
            };
            let removed = self.items.remove(index);
            self.bytes -= removed.content.size();
        }
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or_else(|| {
            CoreError::InvalidInput("clipboard history identifier limit reached".into())
        })?;
        self.items.insert(
            0,
            Entry {
                id,
                pinned: false,
                captured_at,
                content,
            },
        );
        self.bytes += bytes;
        Ok(true)
    }

    pub fn pin(&mut self, id: u64, pinned: bool) -> CoreResult<()> {
        let item = self
            .items
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CoreError::InvalidInput("clipboard item no longer exists".into()))?;
        item.pinned = pinned;
        Ok(())
    }

    pub fn delete(&mut self, id: u64) -> CoreResult<()> {
        let index = self
            .items
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| CoreError::InvalidInput("clipboard item no longer exists".into()))?;
        self.bytes -= self.items.remove(index).content.size();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.bytes = 0;
    }

    pub fn snapshot(&self, query: &str) -> Snapshot {
        let query = query.to_lowercase();
        Snapshot {
            paused: self.paused,
            bytes: self.bytes,
            total_items: self.items.len(),
            error: None,
            items: self
                .items
                .iter()
                .filter(|entry| match &entry.content {
                    Content::Text(text) => query.is_empty() || text.to_lowercase().contains(&query),
                    Content::Image { .. } => query.is_empty() || "image".contains(&query),
                })
                .map(|entry| EntrySummary {
                    id: entry.id,
                    pinned: entry.pinned,
                    captured_at: entry.captured_at,
                    kind: match entry.content {
                        Content::Text(_) => "text",
                        Content::Image { .. } => "image",
                    },
                    preview: entry.content.preview(),
                    bytes: entry.content.size(),
                })
                .collect(),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> CoreResult<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| CoreError::InvalidInput("clipboard state is unavailable".into()))
}

/// All arboard access is serialized; the persistent clipboard owner also supports X11.
pub struct ClipboardService {
    history: Arc<Mutex<History>>,
    clipboard: Arc<Mutex<Option<arboard::Clipboard>>>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self {
            history: Arc::new(Mutex::new(History::default())),
            clipboard: Arc::new(Mutex::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
            last_error: Arc::new(Mutex::new(None)),
        }
    }
}

impl ClipboardService {
    pub fn start(&self) -> CoreResult<()> {
        let mut worker = lock(&self.worker)?;
        // Shutdown sets this before waiting for the same mutex. A Start that
        // was queued behind cleanup must not create an unowned replacement.
        if self.stop.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        if worker.is_some() {
            return Ok(());
        }
        let history = self.history.clone();
        let clipboard = self.clipboard.clone();
        let stop = self.stop.clone();
        let last_error = self.last_error.clone();
        *worker = Some(
            std::thread::Builder::new()
                .name("oxbox-clipboard".into())
                .spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        if let Ok(history_guard) = history.lock()
                            && history_guard.paused
                        {
                            drop(history_guard);
                            std::thread::sleep(Duration::from_millis(500));
                            continue;
                        }
                        let result = (|| -> CoreResult<()> {
                            if concealed_content() {
                                return Ok(());
                            }
                            let mut clipboard = lock(&clipboard)?;
                            if clipboard.is_none() {
                                *clipboard =
                                    Some(arboard::Clipboard::new().map_err(clipboard_error)?);
                            }
                            let Some(clipboard) = clipboard.as_mut() else {
                                return Ok(());
                            };
                            let before = clipboard_sequence();
                            let content = if let Ok(text) = clipboard.get_text() {
                                if text.len() > MAX_BYTES {
                                    return Ok(());
                                }
                                Some(Content::Text(text))
                            } else if let Ok(image) = clipboard.get_image() {
                                if image.bytes.len() > MAX_BYTES {
                                    return Ok(());
                                }
                                Some(Content::Image {
                                    width: image.width,
                                    height: image.height,
                                    rgba: image.bytes.into_owned(),
                                })
                            } else {
                                None
                            };
                            if concealed_content() || before != clipboard_sequence() {
                                return Ok(());
                            }
                            if let Some(content) = content {
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                                    .min(u128::from(u64::MAX))
                                    as u64;
                                lock(&history)?.push(content, now)?;
                            }
                            Ok(())
                        })();
                        if let Ok(mut last_error) = last_error.lock() {
                            *last_error = result.err().map(|error| error.to_string());
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    }
                })?,
        );
        Ok(())
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Release);
        if let Ok(mut worker) = self.worker.lock()
            && let Some(worker) = worker.take()
        {
            let _ = worker.join();
        }
        if let Ok(mut clipboard) = self.clipboard.lock() {
            *clipboard = None;
        }
        if let Ok(mut history) = self.history.lock() {
            history.clear();
        }
    }

    pub fn snapshot(&self, query: &str) -> CoreResult<Snapshot> {
        let mut snapshot = lock(&self.history)?.snapshot(query);
        snapshot.error = lock(&self.last_error)?.clone();
        Ok(snapshot)
    }

    pub fn set_paused(&self, paused: bool) -> CoreResult<()> {
        lock(&self.history)?.set_paused(paused);
        Ok(())
    }
    pub fn pin(&self, id: u64, pinned: bool) -> CoreResult<()> {
        lock(&self.history)?.pin(id, pinned)
    }
    pub fn delete(&self, id: u64) -> CoreResult<()> {
        lock(&self.history)?.delete(id)
    }
    pub fn clear(&self) -> CoreResult<()> {
        lock(&self.history)?.clear();
        Ok(())
    }

    /// Return an independent snapshot of one bounded, in-memory item. Callers
    /// should clone image contents off the UI thread; no clipboard/OS read occurs.
    pub fn content(&self, id: u64) -> CoreResult<Content> {
        lock(&self.history)?
            .items
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.content.clone())
            .ok_or_else(|| CoreError::InvalidInput("clipboard item no longer exists".into()))
    }

    pub fn copy(&self, id: u64) -> CoreResult<()> {
        self.write_content(self.content(id)?)
    }

    pub fn copy_image(&self, image: &image::RgbaImage) -> CoreResult<()> {
        self.write_content(Content::Image {
            width: image.width() as usize,
            height: image.height() as usize,
            rgba: image.as_raw().clone(),
        })
    }

    fn write_content(&self, content: Content) -> CoreResult<()> {
        if self.stop.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        content.validate()?;
        let mut clipboard = lock(&self.clipboard)?;
        if self.stop.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new().map_err(clipboard_error)?);
        }
        let clipboard = clipboard
            .as_mut()
            .ok_or_else(|| CoreError::InvalidInput("clipboard is unavailable".into()))?;
        match content {
            Content::Text(text) => clipboard.set_text(text).map_err(clipboard_error),
            Content::Image {
                width,
                height,
                rgba,
            } => clipboard
                .set_image(arboard::ImageData {
                    width,
                    height,
                    bytes: Cow::Owned(rgba),
                })
                .map_err(clipboard_error),
        }
    }
}

fn clipboard_error(error: arboard::Error) -> CoreError {
    CoreError::InvalidInput(format!("clipboard access failed: {error}"))
}

#[cfg(target_os = "windows")]
fn concealed_content() -> bool {
    use windows_sys::Win32::System::DataExchange::{
        IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };
    // Conservatively ignore marked formats, including password managers' history hint.
    [
        "ExcludeClipboardContentFromMonitorProcessing",
        "CanIncludeInClipboardHistory",
        "Clipboard Viewer Ignore",
    ]
    .iter()
    .any(|name| {
        let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: name is NUL-terminated and lives through the call; neither API dereferences user pointers.
        unsafe {
            let format = RegisterClipboardFormatW(name.as_ptr());
            format != 0 && IsClipboardFormatAvailable(format) != 0
        }
    })
}

#[cfg(not(target_os = "windows"))]
fn concealed_content() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn clipboard_sequence() -> u32 {
    // SAFETY: this API has no parameters or ownership requirements.
    unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

#[cfg(not(target_os = "windows"))]
fn clipboard_sequence() -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_refuses_a_late_start_without_creating_another_watcher() {
        let service = ClipboardService::default();
        service.shutdown();
        let result = service.start();
        let created_worker = service.worker.lock().unwrap().is_some();
        // Always collect a baseline worker before reporting the regression.
        service.shutdown();
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert!(!created_worker);
    }

    #[test]
    fn start_waiting_behind_a_shutdown_observes_the_stop_flag_under_its_lock() {
        let service = Arc::new(ClipboardService::default());
        let held = service.worker.lock().unwrap();
        let stopping = service.clone();
        let shutdown = std::thread::spawn(move || stopping.shutdown());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !service.stop.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "shutdown did not begin"
            );
            std::thread::yield_now();
        }
        drop(held);
        let result = service.start();
        shutdown.join().unwrap();
        service.shutdown();
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert!(service.worker.lock().unwrap().is_none());
    }

    #[test]
    fn readonly_content_snapshot_is_independent_and_rejects_missing_items() {
        let service = ClipboardService::default();
        let id = {
            let mut history = service.history.lock().unwrap();
            history.set_paused(false);
            history
                .push(
                    Content::Image {
                        width: 1,
                        height: 1,
                        rgba: vec![1, 2, 3, 255],
                    },
                    1,
                )
                .unwrap();
            history.items()[0].id
        };
        let Content::Image { mut rgba, .. } = service.content(id).unwrap() else {
            panic!("expected image")
        };
        rgba[0] = 200;
        assert!(
            matches!(service.content(id).unwrap(), Content::Image {rgba, ..} if rgba == [1,2,3,255])
        );
        assert!(service.content(u64::MAX).is_err());
        service.shutdown();
        assert!(service.content(id).is_err());
    }
    #[test]
    fn shutdown_rejects_writes_before_validation_or_clipboard_access() {
        let service = ClipboardService::default();
        service.shutdown();
        let error = service
            .write_content(Content::Image {
                width: 2,
                height: 2,
                rgba: vec![1],
            })
            .unwrap_err();
        assert_eq!(error.code(), "cancelled");
    }

    #[test]
    fn removing_or_clearing_does_not_recapture_unchanged_clipboard() {
        let mut history = History::new(10, 1000);
        history
            .push(Content::Text("current clipboard".into()), 1)
            .unwrap();
        history.clear();
        assert!(
            !history
                .push(Content::Text("current clipboard".into()), 2)
                .unwrap()
        );
        assert!(history.items().is_empty());
        history
            .push(Content::Text("new clipboard".into()), 3)
            .unwrap();
        history.delete(history.items()[0].id).unwrap();
        assert!(
            !history
                .push(Content::Text("new clipboard".into()), 4)
                .unwrap()
        );
    }

    #[test]
    fn deduplicates_and_evicts_oldest_unpinned() {
        let mut history = History::new(2, 100);
        assert!(history.push(Content::Text("one".into()), 1).unwrap());
        assert!(!history.push(Content::Text("one".into()), 2).unwrap());
        let id = history.items()[0].id;
        history.pin(id, true).unwrap();
        history.push(Content::Text("two".into()), 3).unwrap();
        history.push(Content::Text("three".into()), 4).unwrap();
        assert_eq!(history.items().len(), 2);
        assert!(history.items().iter().any(|item| item.id == id));
        assert_eq!(history.bytes(), 8);
    }

    #[test]
    fn pinned_items_never_bypass_byte_or_count_limit() {
        let mut history = History::new(1, 4);
        history.push(Content::Text("four".into()), 1).unwrap();
        history.pin(history.items()[0].id, true).unwrap();
        assert!(!history.push(Content::Text("five".into()), 2).unwrap());
        assert!(!history.push(Content::Text("too big".into()), 3).unwrap());
        assert_eq!(history.bytes(), 4);
    }

    #[test]
    fn pause_and_clear_do_not_capture() {
        let mut history = History::default();
        assert!(!history.push(Content::Text("secret".into()), 1).unwrap());
        history.set_paused(false);
        history.push(Content::Text("visible".into()), 2).unwrap();
        history.clear();
        assert!(history.items().is_empty());
        assert_eq!(history.bytes(), 0);
        assert!(
            Content::Image {
                width: 2,
                height: 2,
                rgba: vec![1]
            }
            .validate()
            .is_err()
        );
    }
}

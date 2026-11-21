//! A local file lock and activation marker; single-instance operation needs no socket.
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
const MARKER: &str = "native-activate.json";
static NEXT: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
    version: u8,
    generation: String,
}
fn guarded(path: &Path) -> std::io::Result<()> {
    oxbox_core::fs_safe::reject_symlink_components(path).map_err(std::io::Error::other)
}
fn read_marker(directory: &Path) -> std::io::Result<Option<Activation>> {
    let path = directory.join(MARKER);
    guarded(&path)?;
    if let Ok(metadata) = std::fs::symlink_metadata(&path)
        && (!metadata.is_file() || metadata.len() > 1024)
    {
        return Err(std::io::Error::other(
            "The reserved activation marker must be an ordinary file below 1 KiB. Back it up before repair.",
        ));
    }
    let value: Option<Activation> =
        oxbox_core::preferences::load(directory, MARKER).map_err(std::io::Error::other)?;
    if let Some(value) = &value
        && (value.version != 1
            || value.generation.is_empty()
            || value.generation.len() > 160
            || !value
                .generation
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-'))
    {
        return Err(std::io::Error::other(
            "The activation marker format is invalid. Its original contents were preserved.",
        ));
    }
    Ok(value)
}
fn write_marker(directory: &Path) -> std::io::Result<()> {
    // A bounded, atomically replaced generation avoids coarse filesystem timestamps
    // and readers observing a secondary process's partially written marker.
    guarded(&directory.join(MARKER))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let value = Activation {
        version: 1,
        generation: format!(
            "{:x}-{now:x}-{:x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ),
    };
    oxbox_core::preferences::save(directory, MARKER, &value).map_err(std::io::Error::other)
}
pub struct Instance {
    _lock: File,
    directory: PathBuf,
    baseline: Option<Activation>,
}

impl Instance {
    pub fn acquire(directory: &Path) -> std::io::Result<Option<Self>> {
        guarded(directory)?;
        std::fs::create_dir_all(directory)?;
        let directory = std::fs::canonicalize(directory)?;
        let path = directory.join("native-instance.lock");
        guarded(&path)?;
        if let Ok(metadata) = std::fs::symlink_metadata(&path)
            && !metadata.is_file()
        {
            return Err(std::io::Error::other(
                "The reserved instance lock must be an ordinary file.",
            ));
        }
        // Read before owning the lock: a second launch can signal immediately
        // after lock acquisition, well before the UI installs its watcher.
        let baseline = read_marker(&directory)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        match lock.try_lock() {
            Ok(()) => Ok(Some(Self {
                _lock: lock,
                directory,
                baseline,
            })),
            Err(std::fs::TryLockError::WouldBlock) => {
                write_marker(&directory)?;
                Ok(None)
            }
            Err(std::fs::TryLockError::Error(error)) => Err(error),
        }
    }
    pub fn watch(
        &self,
        ctx: eframe::egui::Context,
        requested: Arc<AtomicBool>,
        stop: Arc<AtomicBool>,
    ) -> std::io::Result<JoinHandle<()>> {
        let directory = self.directory.clone();
        let mut previous = self.baseline.clone();
        std::thread::Builder::new()
            .name("oxbox-native-events".into())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    if let Ok(current) = read_marker(&directory)
                        && current != previous
                    {
                        if current.is_some() && !stop.load(Ordering::Acquire) {
                            requested.store(true, Ordering::Release);
                        }
                        previous = current;
                    }
                    ctx.request_repaint();
                    std::thread::park_timeout(Duration::from_millis(200));
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn second_launch_before_watch_is_not_lost_and_stop_joins() {
        let directory = tempfile::tempdir().unwrap();
        let first = Instance::acquire(directory.path()).unwrap().unwrap();
        assert!(Instance::acquire(directory.path()).unwrap().is_none());
        let requested = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let watcher = first
            .watch(
                eframe::egui::Context::default(),
                requested.clone(),
                stop.clone(),
            )
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !requested.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        stop.store(true, Ordering::Release);
        watcher.thread().unpark();
        watcher.join().unwrap();
        assert!(
            requested.load(Ordering::Acquire),
            "activation written during startup was lost"
        );
        drop(first);
        assert!(Instance::acquire(directory.path()).unwrap().is_some());
    }
    #[test]
    fn release_permits_new_primary_without_treating_old_marker_as_activation() {
        let directory = tempfile::tempdir().unwrap();
        let first = Instance::acquire(directory.path()).unwrap().unwrap();
        assert!(Instance::acquire(directory.path()).unwrap().is_none());
        drop(first);
        let next = Instance::acquire(directory.path()).unwrap().unwrap();
        let requested = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let watcher = next
            .watch(
                eframe::egui::Context::default(),
                requested.clone(),
                stop.clone(),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(250));
        stop.store(true, Ordering::Release);
        watcher.thread().unpark();
        watcher.join().unwrap();
        assert!(!requested.load(Ordering::Acquire));
    }
    #[test]
    fn malformed_reserved_marker_is_preserved_instead_of_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join(MARKER);
        std::fs::write(&marker, b"existing recovery data").unwrap();
        assert!(Instance::acquire(directory.path()).is_err());
        assert_eq!(std::fs::read(marker).unwrap(), b"existing recovery data");
    }
    #[cfg(unix)]
    #[test]
    fn reserved_marker_symlink_never_writes_its_target() {
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("original.txt");
        std::fs::write(&target, b"preserve me").unwrap();
        std::os::unix::fs::symlink(&target, directory.path().join(MARKER)).unwrap();
        assert!(Instance::acquire(directory.path()).is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"preserve me");
    }
}

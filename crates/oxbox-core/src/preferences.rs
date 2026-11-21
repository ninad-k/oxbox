//! Bounded atomic local JSON storage, independent of a desktop toolkit.
use crate::{CoreError, CoreResult};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
static WRITES: Mutex<()> = Mutex::new(());
static NEXT: AtomicU64 = AtomicU64::new(1);
fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn target(directory: &Path, name: &str) -> CoreResult<PathBuf> {
    if name.is_empty()
        || name.len() > 80
        || name == "."
        || name == ".."
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(invalid("Invalid preference filename"));
    }
    Ok(directory.join(name))
}
pub fn load<T: DeserializeOwned>(directory: &Path, name: &str) -> CoreResult<Option<T>> {
    let path = target(directory, name)?;
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_BYTES as u64
    {
        return Err(invalid(
            "Preference file must be an ordinary file below 8 MiB. Back it up before repair.",
        ));
    }
    let file = File::open(&path)?;
    let value=serde_json::from_reader(file.take(MAX_BYTES as u64+1)).map_err(|_|invalid("Preference file could not be read. Its original contents were preserved; back it up before repair."))?;
    if fs::metadata(path)?.len() > MAX_BYTES as u64 {
        return Err(invalid("Preference file exceeds 8 MiB"));
    }
    Ok(Some(value))
}
struct Limited<W> {
    inner: W,
    written: usize,
}
impl<W: Write> Write for Limited<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_BYTES.saturating_sub(self.written) {
            return Err(std::io::Error::other("Preference data exceeds 8 MiB"));
        }
        let count = self.inner.write(bytes)?;
        self.written += count;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
fn save_unlocked<T: Serialize>(directory: &Path, name: &str, value: &T) -> CoreResult<()> {
    let destination = target(directory, name)?;
    fs::create_dir_all(directory)?;
    if let Ok(metadata) = fs::symlink_metadata(&destination)
        && (!metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err(invalid("Preference target is not an ordinary file"));
    }
    let temporary = directory.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut created = false;
    let result = (|| {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        created = true;
        let mut output = Limited {
            inner: file,
            written: 0,
        };
        serde_json::to_writer_pretty(&mut output, value).map_err(|_| {
            invalid("Preference data could not be saved; the previous file was preserved")
        })?;
        output.flush()?;
        output.inner.sync_all()?;
        drop(output);
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if result.is_err() && created {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub fn save<T: Serialize>(directory: &Path, name: &str, value: &T) -> CoreResult<()> {
    let _guard = WRITES
        .lock()
        .map_err(|_| invalid("Preference storage is unavailable"))?;
    save_unlocked(directory, name, value)
}
pub fn write_key<T: Serialize>(
    directory: &Path,
    name: &str,
    key: &str,
    value: &T,
) -> CoreResult<()> {
    let _guard = WRITES
        .lock()
        .map_err(|_| invalid("Preference storage is unavailable"))?;
    let mut stored: serde_json::Value =
        load(directory, name)?.unwrap_or_else(|| serde_json::json!({}));
    let object = stored
        .as_object_mut()
        .ok_or_else(|| invalid("Preference store is not an object; original data was preserved"))?;
    object.insert(
        key.into(),
        serde_json::to_value(value)
            .map_err(|_| invalid("Preference value cannot be serialized"))?,
    );
    save_unlocked(directory, name, &stored)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_existing_temporary_file_is_never_deleted_on_collision() {
        let _guard = WRITES.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sequence = NEXT.load(Ordering::Relaxed);
        let path = dir.path().join(format!(
            ".settings.json.{}.{sequence}.tmp",
            std::process::id()
        ));
        std::fs::write(&path, b"existing recovery data").unwrap();
        assert!(
            save_unlocked(
                dir.path(),
                "settings.json",
                &serde_json::json!({"theme":"dark"})
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"existing recovery data");
    }
    #[test]
    fn updates_preserve_unknown_keys_and_failed_writes_preserve_original() {
        let dir = tempfile::tempdir().unwrap();
        write_key(dir.path(), "settings.json", "unrelated", &vec![1, 2, 3]).unwrap();
        write_key(dir.path(), "settings.json", "theme", &"dark").unwrap();
        let value: serde_json::Value = load(dir.path(), "settings.json").unwrap().unwrap();
        assert_eq!(value["unrelated"], serde_json::json!([1, 2, 3]));
        let before = std::fs::read(dir.path().join("settings.json")).unwrap();
        assert!(save(dir.path(), "../settings.json", &value).is_err());
        assert_eq!(
            std::fs::read(dir.path().join("settings.json")).unwrap(),
            before
        );
    }
    #[test]
    fn rejects_oversized_nonobject_and_malformed_key_stores_without_replacing_data() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        std::fs::write(&file, b"not json").unwrap();
        assert!(write_key(dir.path(), "settings.json", "theme", &"dark").is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"not json");
        let huge = "x".repeat(MAX_BYTES + 1);
        assert!(save(dir.path(), "new.json", &huge).is_err());
        assert!(!dir.path().join("new.json").exists());
        std::fs::write(&file, b"[]").unwrap();
        assert!(write_key(dir.path(), "settings.json", "theme", &"dark").is_err());
    }
}

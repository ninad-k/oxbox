//! Explicit permanent file removal, with no claim of physical-media sanitization.
use super::batch_rename::FileSnapshot;
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
pub const CONFIRMATION: &str = "DELETE";
pub const CHUNK_BYTES: usize = 64 * 1024;
pub const LIMITATION: &str = "Overwriting cannot guarantee erasure on SSDs, flash wear-leveling, copy-on-write filesystems, snapshots or backups. Cancellation can leave a file partially overwritten. This is not certified secure erase.";
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub files: Vec<FileSnapshot>,
    pub total_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub removed_files: usize,
    pub overwritten_bytes: u64,
}
trait Handle: Write {
    fn verify(&mut self, expected: &FileSnapshot) -> CoreResult<()>;
    fn sync(&mut self) -> CoreResult<()>;
    fn unlink(self, expected: &FileSnapshot) -> CoreResult<()>;
}
trait Backend {
    type File: Handle;
    fn inspect(&mut self, path: &Path, cancel: &CancelToken) -> CoreResult<FileSnapshot>;
    fn open(&mut self, expected: &FileSnapshot, cancel: &CancelToken) -> CoreResult<Self::File>;
}
fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
#[cfg(test)]
fn pending() -> CoreError {
    invalid("injected file identity failure")
}
pub fn review(
    paths: &[PathBuf],
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Plan> {
    cancel.check()?;
    super::batch_rename::validate_paths(paths)?;
    let mut files = Vec::new();
    let mut total_bytes = 0_u64;
    for (index, path) in paths.iter().enumerate() {
        let snapshot = super::batch_rename::snapshot(path, cancel)?;
        total_bytes = total_bytes
            .checked_add(snapshot.bytes)
            .ok_or_else(|| invalid("total file size is too large"))?;
        files.push(snapshot);
        progress(Progress {
            done: index as u64 + 1,
            total: paths.len() as u64,
            message: "Reviewed exact local regular file identity and contents; nothing changed"
                .into(),
        });
    }
    Ok(Plan { files, total_bytes })
}
pub fn shred(
    plan: &Plan,
    confirmation: &str,
    acknowledged_limits: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Report> {
    shred_with(
        plan,
        confirmation,
        acknowledged_limits,
        cancel,
        progress,
        &mut SystemBackend,
    )
}
fn shred_with(
    plan: &Plan,
    confirmation: &str,
    acknowledged_limits: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    backend: &mut impl Backend,
) -> CoreResult<Report> {
    cancel.check()?;
    if confirmation != CONFIRMATION || !acknowledged_limits {
        return Err(invalid(
            "type DELETE and acknowledge the storage and cancellation limitations before permanent removal",
        ));
    }
    super::batch_rename::validate_paths(
        &plan
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>(),
    )?;
    let total = plan.files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(file.bytes)
            .ok_or_else(|| invalid("total file size overflow"))
    })?;
    if total != plan.total_bytes || plan.files.iter().any(|file| file.links != 1) {
        return Err(invalid(
            "reviewed file totals or hard-link state are invalid",
        ));
    }
    // Validate every selected file before opening any of them for writes.
    for file in &plan.files {
        cancel.check()?;
        if backend.inspect(&file.path, cancel)? != *file {
            return Err(invalid(
                "a reviewed file changed; review the exact files again before deleting",
            ));
        }
    }
    let buffer = [0_u8; CHUNK_BYTES];
    let mut report = Report {
        removed_files: 0,
        overwritten_bytes: 0,
    };
    for file in &plan.files {
        cancel.check()?;
        let mut handle = backend.open(file, cancel)?;
        handle.verify(file)?;
        let mut remaining = file.bytes;
        while remaining > 0 {
            cancel.check()?;
            handle.verify(file)?;
            let amount = remaining.min(CHUNK_BYTES as u64) as usize;
            handle.write_all(&buffer[..amount])?;
            remaining -= amount as u64;
            report.overwritten_bytes += amount as u64;
            progress(Progress {
                done: report.overwritten_bytes,
                total: plan.total_bytes,
                message: format!(
                    "Overwriting selected file · {} previously removed; cancellation cannot restore overwritten bytes",
                    report.removed_files
                ),
            });
        }
        cancel.check()?;
        handle.sync()?;
        handle.verify(file)?;
        cancel.check()?;
        handle.unlink(file)?;
        report.removed_files += 1;
        progress(Progress {
            done: report.overwritten_bytes,
            total: plan.total_bytes,
            message: format!(
                "Permanently removed {} selected file(s); physical erasure is not guaranteed",
                report.removed_files
            ),
        });
    }
    Ok(report)
}
struct SystemBackend;
struct OpenFile {
    file: File,
    path: PathBuf,
}
impl Backend for SystemBackend {
    type File = OpenFile;
    fn inspect(&mut self, path: &Path, cancel: &CancelToken) -> CoreResult<FileSnapshot> {
        super::batch_rename::snapshot(path, cancel)
    }
    fn open(&mut self, expected: &FileSnapshot, cancel: &CancelToken) -> CoreResult<OpenFile> {
        cancel.check()?;
        let canonical = super::batch_rename::local_path(&expected.path)?;
        if canonical != expected.path {
            return Err(invalid("reviewed file path is no longer canonical"));
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::{
                Foundation::{GENERIC_READ, GENERIC_WRITE},
                Storage::FileSystem::{
                    DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_WRITE_THROUGH,
                },
            };
            options
                .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
                .share_mode(0)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_WRITE_THROUGH);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_SYNC);
        }
        let mut file = options.open(&canonical)?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        if super::batch_rename::inspect_open(&mut file, &canonical, cancel)? != *expected {
            return Err(invalid(
                "file changed before exclusive overwrite; nothing was written",
            ));
        }
        file.seek(SeekFrom::Start(0))?;
        Ok(OpenFile {
            file,
            path: canonical,
        })
    }
}
impl Write for OpenFile {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.file.write(buffer)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
impl Handle for OpenFile {
    fn verify(&mut self, expected: &FileSnapshot) -> CoreResult<()> {
        let current = super::batch_rename::metadata_snapshot(&self.file, &self.path)?;
        if current.device != expected.device
            || current.file_id != expected.file_id
            || current.bytes != expected.bytes
            || current.links != 1
        {
            return Err(invalid(
                "open file identity, length or link count changed; overwrite stopped",
            ));
        }
        Ok(())
    }
    fn sync(&mut self) -> CoreResult<()> {
        self.file.flush()?;
        self.file.sync_all()?;
        Ok(())
    }
    fn unlink(mut self, expected: &FileSnapshot) -> CoreResult<()> {
        self.verify(expected)?;
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_DISPOSITION_INFO, FileDispositionInfo, SetFileInformationByHandle,
            };
            let information = FILE_DISPOSITION_INFO { DeleteFile: true };
            if unsafe {
                SetFileInformationByHandle(
                    self.file.as_raw_handle() as _,
                    FileDispositionInfo,
                    (&information as *const FILE_DISPOSITION_INFO).cast(),
                    std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            // Only the exact exclusively opened file is marked; closing this handle performs removal.
            drop(self);
            Ok(())
        }
        #[cfg(unix)]
        {
            use std::os::unix::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawFd};
            crate::fs_safe::reject_symlink_components(&self.path)?;
            let parent = self
                .path
                .parent()
                .ok_or_else(|| invalid("file parent unavailable"))?;
            let directory = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
                .open(parent)?;
            let name = std::ffi::CString::new(
                self.path
                    .file_name()
                    .ok_or_else(|| invalid("file name unavailable"))?
                    .as_bytes(),
            )
            .map_err(|_| invalid("invalid filename"))?;
            let mut information: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe {
                libc::fstatat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    &mut information,
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            #[cfg(target_os = "linux")]
            let device = information.st_dev;
            #[cfg(target_os = "macos")]
            let device = information.st_dev as u64;
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            let device = information.st_dev as u64;
            if device != expected.device
                || information.st_ino != expected.file_id
                || information.st_nlink != 1
                || information.st_mode & libc::S_IFMT != libc::S_IFREG
            {
                return Err(invalid(
                    "pathname identity changed; overwritten file was not unlinked",
                ));
            }
            // POSIX has no conditional unlink-by-inode: this fresh check is not an atomic defense
            // against an adversary concurrently changing directory entries. No broader path is used.
            if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if unsafe { libc::fsync(directory.as_raw_fd()) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(())
        }
        #[cfg(not(any(windows, unix)))]
        Err(CoreError::Unsupported("permanent removal on this platform"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Default)]
    struct Effects {
        calls: Vec<&'static str>,
        written: usize,
        max_chunk: usize,
        changed: bool,
    }
    struct Fake {
        expected: FileSnapshot,
        effects: Rc<RefCell<Effects>>,
        cancel: Option<CancelToken>,
    }
    struct FakeFile {
        effects: Rc<RefCell<Effects>>,
        cancel: Option<CancelToken>,
    }
    impl Write for FakeFile {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            let mut e = self.effects.borrow_mut();
            e.calls.push("write");
            e.written += buffer.len();
            e.max_chunk = e.max_chunk.max(buffer.len());
            assert!(buffer.iter().all(|byte| *byte == 0));
            if let Some(cancel) = &self.cancel {
                cancel.cancel();
            }
            Ok(buffer.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl Handle for FakeFile {
        fn verify(&mut self, _: &FileSnapshot) -> CoreResult<()> {
            if self.effects.borrow().changed {
                Err(pending())
            } else {
                Ok(())
            }
        }
        fn sync(&mut self) -> CoreResult<()> {
            self.effects.borrow_mut().calls.push("sync");
            Ok(())
        }
        fn unlink(self, _: &FileSnapshot) -> CoreResult<()> {
            self.effects.borrow_mut().calls.push("unlink");
            Ok(())
        }
    }
    impl Backend for Fake {
        type File = FakeFile;
        fn inspect(&mut self, _: &Path, _: &CancelToken) -> CoreResult<FileSnapshot> {
            let mut e = self.expected.clone();
            if self.effects.borrow().changed {
                e.file_id += 1;
            }
            Ok(e)
        }
        fn open(&mut self, _: &FileSnapshot, _: &CancelToken) -> CoreResult<FakeFile> {
            self.effects.borrow_mut().calls.push("open");
            Ok(FakeFile {
                effects: self.effects.clone(),
                cancel: self.cancel.clone(),
            })
        }
    }
    fn fixture(bytes: u64) -> (Plan, Fake) {
        let file = FileSnapshot {
            path: std::env::temp_dir().join("oxbox-fake-shred/only-selected.bin"),
            bytes,
            device: 1,
            file_id: 1,
            modified_secs: 1,
            modified_nanos: 0,
            links: 1,
            hash: [3; 32],
        };
        let fake = Fake {
            expected: file.clone(),
            effects: Rc::new(RefCell::new(Effects::default())),
            cancel: None,
        };
        (
            Plan {
                files: vec![file],
                total_bytes: bytes,
            },
            fake,
        )
    }
    #[test]
    fn permanent_removal_requires_typed_confirmation_and_limit_acknowledgement() {
        let (plan, mut fake) = fixture(10);
        for (text, ack) in [("", true), ("delete", true), ("DELETE", false)] {
            assert!(
                shred_with(
                    &plan,
                    text,
                    ack,
                    &CancelToken::default(),
                    &mut |_| {},
                    &mut fake
                )
                .is_err()
            );
        }
        assert!(fake.effects.borrow().calls.is_empty());
    }
    #[test]
    fn streaming_overwrite_syncs_before_exact_selected_unlink() {
        let (plan, mut fake) = fixture((CHUNK_BYTES * 3 + 7) as u64);
        let report = shred_with(
            &plan,
            "DELETE",
            true,
            &CancelToken::default(),
            &mut |_| {},
            &mut fake,
        )
        .unwrap();
        let e = fake.effects.borrow();
        assert_eq!(e.written, CHUNK_BYTES * 3 + 7);
        assert!(e.max_chunk <= CHUNK_BYTES);
        assert_eq!(&e.calls[e.calls.len() - 2..], &["sync", "unlink"]);
        assert_eq!(report.removed_files, 1);
        assert_eq!(report.overwritten_bytes, plan.total_bytes);
    }
    #[test]
    fn cancelled_mid_overwrite_never_unlinks_or_claims_erasure() {
        let (plan, mut fake) = fixture((CHUNK_BYTES * 3) as u64);
        let cancel = CancelToken::default();
        fake.cancel = Some(cancel.clone());
        assert_eq!(
            shred_with(&plan, "DELETE", true, &cancel, &mut |_| {}, &mut fake)
                .unwrap_err()
                .code(),
            "cancelled"
        );
        let e = fake.effects.borrow();
        assert_eq!(e.written, CHUNK_BYTES);
        assert!(!e.calls.contains(&"unlink"));
    }
    #[test]
    fn stale_identity_hardlinks_and_tampered_totals_fail_before_open() {
        let (mut plan, mut fake) = fixture(5);
        fake.effects.borrow_mut().changed = true;
        assert!(
            shred_with(
                &plan,
                "DELETE",
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        fake.effects.borrow_mut().changed = false;
        plan.files[0].links = 2;
        assert!(
            shred_with(
                &plan,
                "DELETE",
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        plan.files[0].links = 1;
        plan.total_bytes = 9;
        assert!(
            shred_with(
                &plan,
                "DELETE",
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        assert!(fake.effects.borrow().calls.is_empty());
    }
}

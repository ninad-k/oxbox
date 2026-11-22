//! Reviewed, bounded same-directory batch renaming and recoverable Undo.
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub const MAX_FILES: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counter {
    pub start: u64,
    pub width: u8,
    pub separator: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    pub replace_from: String,
    pub replace_to: String,
    pub prefix: String,
    pub suffix: String,
    pub counter: Option<Counter>,
    pub preserve_extension: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            replace_from: String::new(),
            replace_to: String::new(),
            prefix: String::new(),
            suffix: String::new(),
            counter: None,
            preserve_extension: true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub bytes: u64,
    pub(crate) device: u64,
    pub(crate) file_id: u64,
    pub(crate) modified_secs: u64,
    pub(crate) modified_nanos: u32,
    pub(crate) links: u64,
    pub(crate) hash: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub from: PathBuf,
    pub to: PathBuf,
    pub snapshot: FileSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub entries: Vec<Entry>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryState {
    Pending,
    Moved,
    Undone,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub entry: Entry,
    pub state: EntryState,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub entries: Vec<JournalEntry>,
}

trait Backend {
    fn inspect(&mut self, path: &Path, cancel: &CancelToken) -> CoreResult<FileSnapshot>;
    fn exists(&mut self, path: &Path) -> CoreResult<bool>;
    fn rename(
        &mut self,
        from: &Path,
        to: &Path,
        expected: &FileSnapshot,
        cancel: &CancelToken,
    ) -> CoreResult<()>;
}
pub fn preview(
    paths: &[PathBuf],
    options: &Options,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Plan> {
    preview_with(paths, options, cancel, progress, &mut SystemBackend)
}
pub fn apply(
    plan: &Plan,
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    save: &mut dyn FnMut(&Journal) -> CoreResult<()>,
) -> CoreResult<Journal> {
    apply_with(plan, confirmed, cancel, progress, save, &mut SystemBackend)
}
pub fn undo(
    journal: &Journal,
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    save: &mut dyn FnMut(&Journal) -> CoreResult<()>,
) -> CoreResult<Journal> {
    undo_with(
        journal,
        confirmed,
        cancel,
        progress,
        save,
        &mut SystemBackend,
    )
}
fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
#[cfg(test)]
fn pending() -> CoreError {
    invalid("injected file operation failure")
}
fn key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}
fn protected_file_location(path: &str, windows: bool, system_attribute: bool) -> bool {
    if system_attribute {
        return true;
    }
    let normalized = path.replace('\\', "/").to_lowercase();
    let normalized = normalized.strip_prefix("//?/").unwrap_or(&normalized);
    if windows {
        let bytes = normalized.as_bytes();
        if bytes.len() < 4
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'/'
        {
            return true;
        }
        let parts = normalized[3..]
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() < 2 {
            return true;
        }
        matches!(
            parts[0],
            "windows"
                | "program files"
                | "program files (x86)"
                | "programdata"
                | "$recycle.bin"
                | "system volume information"
                | "recovery"
                | "boot"
                | "efi"
                | "config.msi"
                | "perflogs"
                | "msocache"
        ) || (parts[0] == "users" && parts.len() < 3)
    } else {
        let parts = normalized
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        parts.len() < 2
            || parts.first().is_some_and(|first| {
                matches!(
                    *first,
                    "bin"
                        | "boot"
                        | "dev"
                        | "etc"
                        | "lib"
                        | "lib64"
                        | "opt"
                        | "proc"
                        | "root"
                        | "run"
                        | "sbin"
                        | "snap"
                        | "srv"
                        | "sys"
                        | "usr"
                        | "system"
                        | "library"
                        | "applications"
                        | "lost+found"
                )
            })
    }
}
pub(crate) fn validate_paths(paths: &[PathBuf]) -> CoreResult<()> {
    if paths.is_empty()
        || paths.len() > MAX_FILES
        || paths.iter().map(|p| p.as_os_str().len()).sum::<usize>() > 128 * 1024
    {
        return Err(invalid(
            "select between 1 and 256 files with at most 128 KiB of path text",
        ));
    }
    let mut seen = HashSet::new();
    for path in paths {
        if !path.is_absolute()
            || path.parent().is_none()
            || path
                .components()
                .any(|c| c == std::path::Component::ParentDir)
            || !seen.insert(key(path))
        {
            return Err(invalid(
                "select distinct absolute files without parent-directory components",
            ));
        }
    }
    Ok(())
}
fn new_name(path: &Path, options: &Options, index: usize) -> CoreResult<String> {
    if [
        &options.replace_from,
        &options.replace_to,
        &options.prefix,
        &options.suffix,
    ]
    .iter()
    .any(|text| text.len() > 255)
    {
        return Err(invalid("rename text fields must each be at most 255 bytes"));
    }
    let filename = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| invalid("rename requires Unicode filenames"))?;
    let (stem, extension) = if options.preserve_extension {
        (
            path.file_stem()
                .and_then(|v| v.to_str())
                .ok_or_else(|| invalid("invalid file stem"))?,
            path.extension().and_then(|v| v.to_str()),
        )
    } else {
        (filename, None)
    };
    let stem = if options.replace_from.is_empty() {
        stem.to_owned()
    } else {
        stem.replace(&options.replace_from, &options.replace_to)
    };
    let mut output = format!("{}{}{}", options.prefix, stem, options.suffix);
    if let Some(counter) = &options.counter {
        if !(1..=20).contains(&counter.width) || counter.separator.len() > 16 {
            return Err(invalid(
                "counter width must be 1–20 and its separator at most 16 bytes",
            ));
        }
        let number = counter
            .start
            .checked_add(index as u64)
            .ok_or_else(|| invalid("counter exceeds its maximum value"))?;
        output.push_str(&counter.separator);
        output.push_str(&format!(
            "{:0width$}",
            number,
            width = usize::from(counter.width)
        ));
    }
    if let Some(extension) = extension {
        output.push('.');
        output.push_str(extension);
    }
    crate::fs_safe::validate_portable_filename(&output)?;
    if output.len() > 255 {
        return Err(invalid("a resulting filename exceeds 255 UTF-8 bytes"));
    }
    Ok(output)
}
fn preview_with(
    paths: &[PathBuf],
    options: &Options,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    backend: &mut impl Backend,
) -> CoreResult<Plan> {
    cancel.check()?;
    validate_paths(paths)?;
    let mut entries = Vec::new();
    let mut targets = HashSet::new();
    for (index, path) in paths.iter().enumerate() {
        cancel.check()?;
        let snapshot = backend.inspect(path, cancel)?;
        let from = snapshot.path.clone();
        let to = from.with_file_name(new_name(&from, options, index)?);
        if key(&from) == key(&to) {
            return Err(invalid(
                "a proposed name is unchanged or changes only case; choose a distinct name",
            ));
        }
        if !targets.insert(key(&to)) || backend.exists(&to)? {
            return Err(invalid(
                "a proposed target already exists or collides with another selected filename",
            ));
        }
        entries.push(Entry { from, to, snapshot });
        progress(Progress {
            done: index as u64 + 1,
            total: paths.len() as u64,
            message: "Reviewed file identity, contents and proposed name".into(),
        });
    }
    let plan = Plan { entries };
    validate_plan(&plan)?;
    Ok(plan)
}
pub fn validate_journal(journal: &Journal) -> CoreResult<()> {
    if journal.entries.is_empty() {
        return Ok(());
    }
    validate_plan(&Plan {
        entries: journal
            .entries
            .iter()
            .map(|item| item.entry.clone())
            .collect(),
    })
}
fn validate_plan(plan: &Plan) -> CoreResult<()> {
    validate_paths(
        &plan
            .entries
            .iter()
            .map(|e| e.from.clone())
            .collect::<Vec<_>>(),
    )?;
    validate_paths(
        &plan
            .entries
            .iter()
            .map(|e| e.to.clone())
            .collect::<Vec<_>>(),
    )?;
    let mut destinations = HashSet::new();
    let sources: HashSet<_> = plan.entries.iter().map(|e| key(&e.from)).collect();
    for entry in &plan.entries {
        if entry.snapshot.path != entry.from
            || entry.from.parent() != entry.to.parent()
            || entry.snapshot.links != 1
            || sources.contains(&key(&entry.to))
            || !destinations.insert(key(&entry.to))
        {
            return Err(invalid(
                "rename plans must contain distinct single-link regular files and new names in the same folder",
            ));
        }
        let name = entry
            .to
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| invalid("invalid target filename"))?;
        crate::fs_safe::validate_portable_filename(name)?;
        if name.len() > 255 {
            return Err(invalid("target filename is too long"));
        }
    }
    Ok(())
}
fn ensure_source(
    backend: &mut impl Backend,
    entry: &Entry,
    cancel: &CancelToken,
) -> CoreResult<()> {
    cancel.check()?;
    if backend.inspect(&entry.from, cancel)? != entry.snapshot || backend.exists(&entry.to)? {
        return Err(invalid(
            "a reviewed source or destination changed; preview again before renaming",
        ));
    }
    Ok(())
}
fn apply_with(
    plan: &Plan,
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    save: &mut dyn FnMut(&Journal) -> CoreResult<()>,
    backend: &mut impl Backend,
) -> CoreResult<Journal> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid("confirm the exact previewed rename before Apply"));
    }
    validate_plan(plan)?;
    for entry in &plan.entries {
        ensure_source(backend, entry, cancel)?;
    }
    let mut journal = Journal {
        entries: plan
            .entries
            .iter()
            .cloned()
            .map(|entry| JournalEntry {
                entry,
                state: EntryState::Pending,
            })
            .collect(),
    };
    // Persist intent first: a crash after rename but before its status save can be reconciled by identity.
    save(&journal)?;
    for index in 0..journal.entries.len() {
        cancel.check()?;
        let entry = &journal.entries[index].entry;
        ensure_source(backend, entry, cancel)?;
        backend.rename(&entry.from, &entry.to, &entry.snapshot, cancel)?;
        journal.entries[index].state = EntryState::Moved;
        save(&journal)?;
        progress(Progress {
            done: index as u64 + 1,
            total: journal.entries.len() as u64,
            message: "Renamed a reviewed file; Undo journal saved".into(),
        });
    }
    Ok(journal)
}
fn undo_with(
    journal: &Journal,
    confirmed: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
    save: &mut dyn FnMut(&Journal) -> CoreResult<()>,
    backend: &mut impl Backend,
) -> CoreResult<Journal> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid("confirm the exact Undo paths first"));
    }
    validate_journal(journal)?;
    if journal.entries.is_empty() {
        return Err(invalid("there is no rename journal to undo"));
    }
    let mut output = journal.clone();
    let mut restore = Vec::new();
    for (index, item) in journal.entries.iter().enumerate() {
        cancel.check()?;
        let entry = &item.entry;
        let from_exists = backend.exists(&entry.from)?;
        let to_exists = backend.exists(&entry.to)?;
        if from_exists && !to_exists && backend.inspect(&entry.from, cancel)? == entry.snapshot {
            output.entries[index].state = EntryState::Undone;
        } else if !from_exists && to_exists && item.state != EntryState::Undone {
            let mut expected = entry.snapshot.clone();
            expected.path = entry.to.clone();
            if backend.inspect(&entry.to, cancel)? != expected {
                return Err(invalid(
                    "renamed contents or identity changed; Undo was refused",
                ));
            }
            restore.push((index, expected));
        } else {
            return Err(invalid(
                "Undo paths are missing, occupied or changed; no files were overwritten",
            ));
        }
    }
    save(&output)?;
    for (index, expected) in restore {
        cancel.check()?;
        let entry = &output.entries[index].entry;
        if backend.exists(&entry.from)? || backend.inspect(&entry.to, cancel)? != expected {
            return Err(invalid("an Undo path changed; remaining Undo was stopped"));
        }
        backend.rename(&entry.to, &entry.from, &expected, cancel)?;
        output.entries[index].state = EntryState::Undone;
        save(&output)?;
        progress(Progress {
            done: output
                .entries
                .iter()
                .filter(|e| e.state == EntryState::Undone)
                .count() as u64,
            total: output.entries.len() as u64,
            message: "Restored a matching file without overwriting".into(),
        });
    }
    Ok(output)
}

struct SystemBackend;
impl Backend for SystemBackend {
    fn inspect(&mut self, path: &Path, cancel: &CancelToken) -> CoreResult<FileSnapshot> {
        snapshot(path, cancel)
    }
    fn exists(&mut self, path: &Path) -> CoreResult<bool> {
        crate::fs_safe::reject_symlink_components(path)?;
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
    fn rename(
        &mut self,
        from: &Path,
        to: &Path,
        expected: &FileSnapshot,
        cancel: &CancelToken,
    ) -> CoreResult<()> {
        rename_no_replace(from, to, expected, cancel)
    }
}
pub(crate) fn local_path(path: &Path) -> CoreResult<PathBuf> {
    if !path.is_absolute() {
        return Err(invalid("select an absolute local file path"));
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let drive = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
                _ => return Err(invalid("network and device paths are not allowed")),
            },
            _ => return Err(invalid("choose a local drive path")),
        };
        for component in path.components() {
            if let Component::Normal(value) = component {
                crate::fs_safe::validate_output_filename(value)?;
            }
        }
        let root = [u16::from(drive), 58, 92, 0];
        let kind = unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) };
        // GetDriveTypeW's documented DRIVE_REMOVABLE/FIXED/RAMDISK values.
        // windows-sys exposes this function but not these SDK constants in this module.
        if !matches!(kind, 2 | 3 | 6) {
            return Err(invalid(
                "remote, optical and unavailable drives are not allowed",
            ));
        }
    }
    let canonical = crate::fs_safe::validate_trash_path(path)?;
    if protected_file_location(&canonical.to_string_lossy(), cfg!(windows), false) {
        return Err(invalid(
            "root-directory files and operating-system locations are protected",
        ));
    }
    crate::fs_safe::reject_symlink_components(&canonical)?;
    local_filesystem(&canonical)?;
    Ok(canonical)
}
fn local_filesystem(path: &Path) -> CoreResult<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| invalid("invalid file path"))?;
        let mut information: libc::statfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statfs(path.as_ptr(), &mut information) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        #[cfg(target_os = "linux")]
        let local = matches!(
            information.f_type as u64,
            0xef53
                | 0x58465342
                | 0x9123683e
                | 0x01021994
                | 0x794c7630
                | 0x2fc12fc1
                | 0xf2f52010
                | 0x52654973
        );
        #[cfg(target_os = "macos")]
        let local = information.f_flags & libc::MNT_LOCAL as u32 != 0;
        if !local {
            return Err(invalid(
                "this filesystem is remote or not in the conservative local-filesystem allow-list",
            ));
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let _ = path;
    Ok(())
}
pub(crate) fn open_read(path: &Path) -> CoreResult<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    Ok(options.open(path)?)
}
pub(crate) fn metadata_snapshot(file: &File, path: &Path) -> CoreResult<FileSnapshot> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || crate::fs_safe::is_link(&metadata) {
        return Err(invalid(
            "only regular files without links or reparse points are allowed",
        ));
    }
    #[cfg(windows)]
    let system_attribute = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 4 != 0
    };
    #[cfg(not(windows))]
    let system_attribute = false;
    if protected_file_location(&path.to_string_lossy(), cfg!(windows), system_attribute) {
        return Err(invalid(
            "system files and operating-system locations are protected",
        ));
    }
    #[cfg(windows)]
    let (device, file_id, links) = {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut information) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        (
            u64::from(information.dwVolumeSerialNumber),
            (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
            u64::from(information.nNumberOfLinks),
        )
    };
    #[cfg(unix)]
    let (device, file_id, links) = {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino(), metadata.nlink())
    };
    #[cfg(not(any(windows, unix)))]
    return Err(CoreError::Unsupported("this file-identity platform"));
    #[cfg(any(windows, unix))]
    {
        if links != 1 {
            return Err(invalid("hard-linked files are not allowed"));
        }
        if device == 0 || file_id == 0 {
            return Err(invalid(
                "this filesystem does not provide a usable file identity",
            ));
        }
        let modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| invalid("file modification time is not supported"))?;
        Ok(FileSnapshot {
            path: path.to_owned(),
            bytes: metadata.len(),
            device,
            file_id,
            modified_secs: modified.as_secs(),
            modified_nanos: modified.subsec_nanos(),
            links,
            hash: [0; 32],
        })
    }
}
pub(crate) fn inspect_open(
    file: &mut File,
    path: &Path,
    cancel: &CancelToken,
) -> CoreResult<FileSnapshot> {
    let mut snapshot = metadata_snapshot(file, path)?;
    file.seek(SeekFrom::Start(0))?;
    let mut hash = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut count = 0_u64;
    loop {
        cancel.check()?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        count = count
            .checked_add(read as u64)
            .ok_or_else(|| invalid("file size overflow"))?;
        if count > snapshot.bytes {
            return Err(invalid("file grew while it was reviewed"));
        }
        hash.update(&buffer[..read]);
    }
    if count != snapshot.bytes || metadata_snapshot(file, path)? != snapshot {
        return Err(invalid("file changed during review; preview again"));
    }
    snapshot.hash = *hash.finalize().as_bytes();
    Ok(snapshot)
}
pub(crate) fn snapshot(path: &Path, cancel: &CancelToken) -> CoreResult<FileSnapshot> {
    cancel.check()?;
    let path = local_path(path)?;
    let mut file = open_read(&path)?;
    inspect_open(&mut file, &path, cancel)
}
fn rename_no_replace(
    from: &Path,
    to: &Path,
    expected: &FileSnapshot,
    cancel: &CancelToken,
) -> CoreResult<()> {
    cancel.check()?;
    if from.parent() != to.parent() {
        return Err(invalid(
            "renames must stay in the reviewed parent directory",
        ));
    }
    let checked = local_path(from)?;
    if checked != from {
        return Err(invalid("rename source must remain canonical"));
    }
    crate::fs_safe::reject_symlink_components(to)?;
    #[cfg(windows)]
    {
        use std::{
            mem::{offset_of, size_of},
            os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
        };
        use windows_sys::Win32::Storage::FileSystem::*;
        let parent = from
            .parent()
            .ok_or_else(|| invalid("file parent is unavailable"))?;
        let directory = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(parent)?;
        let mut file = OpenOptions::new()
            .access_mode(windows_sys::Win32::Foundation::GENERIC_READ | DELETE)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(from)?;
        if inspect_open(&mut file, from, cancel)? != *expected {
            return Err(invalid("source identity or contents changed before rename"));
        }
        let name = to
            .file_name()
            .ok_or_else(|| invalid("target filename missing"))?
            .encode_wide()
            .collect::<Vec<_>>();
        let bytes = offset_of!(FILE_RENAME_INFO, FileName) + (name.len() + 1) * 2;
        let mut allocation = vec![0_usize; bytes.div_ceil(size_of::<usize>())];
        let info = allocation.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        // Aligned storage includes the complete variable UTF-16 filename and NUL.
        unsafe {
            (*info).Anonymous.ReplaceIfExists = false;
            (*info).RootDirectory = directory.as_raw_handle() as _;
            (*info).FileNameLength = (name.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
        }
        cancel.check()?;
        if unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle() as _,
                FileRenameInfo,
                info.cast(),
                bytes as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::{
            ffi::OsStrExt,
            fs::OpenOptionsExt,
            io::{AsRawFd, FromRawFd},
        };
        let parent = from
            .parent()
            .ok_or_else(|| invalid("file parent missing"))?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(parent)?;
        let source = std::ffi::CString::new(
            from.file_name()
                .ok_or_else(|| invalid("source name missing"))?
                .as_bytes(),
        )
        .map_err(|_| invalid("invalid source name"))?;
        let target = std::ffi::CString::new(
            to.file_name()
                .ok_or_else(|| invalid("target name missing"))?
                .as_bytes(),
        )
        .map_err(|_| invalid("invalid target name"))?;
        // Inspect the source relative to the same held directory used by rename;
        // a renamed parent must not redirect the check to a different namespace.
        let source_fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                source.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if source_fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let mut file = unsafe { File::from_raw_fd(source_fd) };
        if inspect_open(&mut file, from, cancel)? != *expected {
            return Err(invalid("source changed before rename"));
        }
        let mut current: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                source.as_ptr(),
                &mut current,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        #[cfg(target_os = "linux")]
        let device = current.st_dev;
        #[cfg(target_os = "macos")]
        let device = current.st_dev as u64;
        if device != expected.device
            || current.st_ino != expected.file_id
            || current.st_nlink != 1
            || current.st_mode & libc::S_IFMT != libc::S_IFREG
        {
            return Err(invalid("source directory entry changed before rename"));
        }
        cancel.check()?;
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::renameat2(
                directory.as_raw_fd(),
                source.as_ptr(),
                directory.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        #[cfg(target_os = "macos")]
        let result = unsafe {
            libc::renameatx_np(
                directory.as_raw_fd(),
                source.as_ptr(),
                directory.as_raw_fd(),
                target.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = (to, expected);
        Err(CoreError::Unsupported(
            "atomic no-replace rename on this platform",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct Fake {
        files: BTreeMap<PathBuf, FileSnapshot>,
        moves: usize,
        fail_after: Option<usize>,
    }
    fn path(name: &str) -> PathBuf {
        std::env::temp_dir().join("oxbox-fake-rename").join(name)
    }
    fn stamp(name: &str, id: u64) -> FileSnapshot {
        FileSnapshot {
            path: path(name),
            bytes: 12,
            device: 1,
            file_id: id,
            modified_secs: 10,
            modified_nanos: 1,
            links: 1,
            hash: [id as u8; 32],
        }
    }
    impl Fake {
        fn add(&mut self, name: &str, id: u64) {
            self.files.insert(path(name), stamp(name, id));
        }
    }
    impl Backend for Fake {
        fn inspect(&mut self, path: &Path, _: &CancelToken) -> CoreResult<FileSnapshot> {
            self.files.get(path).cloned().ok_or_else(pending)
        }
        fn exists(&mut self, path: &Path) -> CoreResult<bool> {
            Ok(self.files.contains_key(path))
        }
        fn rename(
            &mut self,
            from: &Path,
            to: &Path,
            expected: &FileSnapshot,
            _: &CancelToken,
        ) -> CoreResult<()> {
            assert_eq!(self.files.get(from), Some(expected));
            assert!(
                !self.files.contains_key(to),
                "never overwrite even in a fake effect"
            );
            if self.fail_after == Some(self.moves) {
                return Err(pending());
            }
            let mut moved = self.files.remove(from).unwrap();
            moved.path = to.to_owned();
            self.files.insert(to.to_owned(), moved);
            self.moves += 1;
            Ok(())
        }
    }
    fn options() -> Options {
        Options {
            prefix: "trip_".into(),
            suffix: "_edited".into(),
            replace_from: "photo".into(),
            replace_to: "image".into(),
            counter: Some(Counter {
                start: 7,
                width: 3,
                separator: "_".into(),
            }),
            ..Options::default()
        }
    }
    #[test]
    fn preview_literal_rules_preserve_extension_and_apply_nothing() {
        let mut fake = Fake::default();
        fake.add("photo.txt", 1);
        fake.add("photo.png", 2);
        let plan = preview_with(
            &[path("photo.txt"), path("photo.png")],
            &options(),
            &CancelToken::default(),
            &mut |_| {},
            &mut fake,
        )
        .unwrap();
        assert_eq!(plan.entries[0].to, path("trip_image_edited_007.txt"));
        assert_eq!(plan.entries[1].to, path("trip_image_edited_008.png"));
        assert_eq!(fake.moves, 0);
    }
    #[test]
    fn apply_requires_confirmation_and_revalidates_all_sources_before_any_effect() {
        let mut fake = Fake::default();
        fake.add("a.txt", 1);
        fake.add("b.txt", 2);
        let entries = vec![
            Entry {
                from: path("a.txt"),
                to: path("new_a.txt"),
                snapshot: stamp("a.txt", 1),
            },
            Entry {
                from: path("b.txt"),
                to: path("new_b.txt"),
                snapshot: stamp("b.txt", 2),
            },
        ];
        let plan = Plan { entries };
        assert!(
            apply_with(
                &plan,
                false,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| Ok(()),
                &mut fake
            )
            .is_err()
        );
        fake.files.get_mut(&path("b.txt")).unwrap().file_id = 99;
        assert!(
            apply_with(
                &plan,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| Ok(()),
                &mut fake
            )
            .is_err()
        );
        assert_eq!(fake.moves, 0);
    }
    #[test]
    fn write_ahead_journal_recovers_partial_failure_and_undo_never_overwrites() {
        let mut fake = Fake::default();
        fake.add("a.txt", 1);
        fake.add("b.txt", 2);
        fake.fail_after = Some(1);
        let plan = Plan {
            entries: vec![
                Entry {
                    from: path("a.txt"),
                    to: path("new_a.txt"),
                    snapshot: stamp("a.txt", 1),
                },
                Entry {
                    from: path("b.txt"),
                    to: path("new_b.txt"),
                    snapshot: stamp("b.txt", 2),
                },
            ],
        };
        let mut durable = Journal::default();
        assert!(
            apply_with(
                &plan,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |journal| {
                    durable = journal.clone();
                    Ok(())
                },
                &mut fake
            )
            .is_err()
        );
        assert_eq!(fake.moves, 1);
        assert_eq!(durable.entries.len(), 2);
        fake.fail_after = None;
        // Simulate a crash between the first move and its post-move journal save.
        durable.entries[0].state = EntryState::Pending;
        let restored = undo_with(
            &durable,
            true,
            &CancelToken::default(),
            &mut |_| {},
            &mut |_| Ok(()),
            &mut fake,
        )
        .unwrap();
        assert!(
            restored
                .entries
                .iter()
                .all(|entry| entry.state == EntryState::Undone)
        );
        assert!(fake.files.contains_key(&path("a.txt")));
        assert!(fake.files.contains_key(&path("b.txt")));
        fake.add("new_a.txt", 10);
        assert!(
            undo_with(
                &durable,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| Ok(()),
                &mut fake
            )
            .is_err()
        );
    }
    #[test]
    fn collisions_invalid_names_limits_and_counter_overflow_are_refused() {
        let mut fake = Fake::default();
        fake.add("a.txt", 1);
        fake.add("new_a.txt", 2);
        let mut options = Options {
            prefix: "new_".into(),
            ..Options::default()
        };
        assert!(
            preview_with(
                &[path("a.txt")],
                &options,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        options.prefix = "../".into();
        assert!(
            preview_with(
                &[path("a.txt")],
                &options,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        assert!(
            preview_with(
                &vec![path("a.txt"); MAX_FILES + 1],
                &Options::default(),
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
        options = Options {
            counter: Some(Counter {
                start: u64::MAX,
                width: 2,
                separator: "_".into(),
            }),
            ..Options::default()
        };
        assert!(
            preview_with(
                &[path("a.txt"), path("new_a.txt")],
                &options,
                &CancelToken::default(),
                &mut |_| {},
                &mut fake
            )
            .is_err()
        );
    }
    #[test]
    fn cancelled_or_failed_journal_never_mutates_a_file() {
        let mut fake = Fake::default();
        fake.add("a.txt", 1);
        let plan = Plan {
            entries: vec![Entry {
                from: path("a.txt"),
                to: path("new_a.txt"),
                snapshot: stamp("a.txt", 1),
            }],
        };
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            apply_with(
                &plan,
                true,
                &cancel,
                &mut |_| {},
                &mut |_| Ok(()),
                &mut fake
            )
            .unwrap_err()
            .code(),
            "cancelled"
        );
        assert!(
            apply_with(
                &plan,
                true,
                &CancelToken::default(),
                &mut |_| {},
                &mut |_| Err(pending()),
                &mut fake
            )
            .is_err()
        );
        assert_eq!(fake.moves, 0);
    }
    #[test]
    fn system_file_policy_refuses_drive_root_files_other_os_volumes_and_system_attributes() {
        for (path, windows, system) in [
            (r"C:\bootmgr", true, false),
            (r"D:\Windows\system32\service.exe", true, false),
            (r"D:\Program Files\app\binary.exe", true, false),
            (r"C:\System Volume Information\index", true, false),
            (r"C:\Users\person\marked.dat", true, true),
            ("/opt/app/binary", false, false),
            ("/run/service/state", false, false),
        ] {
            assert!(protected_file_location(path, windows, system), "{path}");
        }
        for (path, windows) in [
            (r"D:\Documents\draft.txt", true),
            (r"C:\Users\person\Desktop\draft.txt", true),
            ("/home/person/draft.txt", false),
            ("/tmp/oxbox-fixture.bin", false),
        ] {
            assert!(!protected_file_location(path, windows, false), "{path}");
        }
    }
}

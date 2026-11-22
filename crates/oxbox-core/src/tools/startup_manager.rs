//! Reviewed, reversible current-user startup configuration only.

use super::maintenance_support::{
    self as support, MAX_CONFIG_BYTES, NativeRegistry, RUN_KEY, RegistryArea, RegistryBackend,
    RegistryValue, invalid,
};
use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
    platform::Os,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum FileStyle {
    LaunchAgent,
    DesktopEntry,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct FileConfig {
    style: FileStyle,
    root: PathBuf,
    path: PathBuf,
    original: Vec<u8>,
    disabled: Vec<u8>,
    parked: Option<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Source {
    Run(RegistryValue),
    File(FileConfig),
}
#[derive(Clone, Debug)]
pub struct StartupEntry {
    pub name: String,
    pub location: String,
    pub command: String,
    pub enabled: bool,
    pub note: String,
    source: Source,
}
impl StartupEntry {
    pub fn belongs_to_application(
        &self,
        name: &str,
        executable: &Path,
        home: &Path,
        os: Os,
    ) -> bool {
        owned_source(&self.source, name, executable, home, os)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Backup {
    schema: u8,
    purpose: String,
    source: Source,
}
#[derive(Clone, Debug)]
pub struct RecoveryReview {
    pub description: String,
    path: PathBuf,
    saved: Backup,
}
impl RecoveryReview {
    pub fn belongs_to_application(
        &self,
        name: &str,
        executable: &Path,
        home: &Path,
        os: Os,
    ) -> bool {
        owned_source(&self.saved.source, name, executable, home, os)
    }
    /// Read-only identity for a host that delegates its own login preference to
    /// Settings. The reviewed source remains private and cannot be replaced.
    pub fn run_entry_identity(&self) -> Option<(String, String)> {
        match &self.saved.source {
            Source::Run(value) => support::registry_text(value)
                .ok()
                .map(|command| (value.name.clone(), command)),
            Source::File(_) => None,
        }
    }
}

fn compact_agent_xml(text: &str) -> Option<String> {
    if text.contains('\0')
        || text.contains("<!--")
        || text.contains("<![CDATA[")
        || text.contains("<!ENTITY")
    {
        return None;
    }
    let start = text.find("<plist ")?;
    let mut remaining = &text[start..];
    let mut result = String::new();
    while let Some(open) = remaining.find('<') {
        let plain = &remaining[..open];
        // Remove formatting whitespace between tags, never whitespace inside a
        // non-empty value such as an executable path containing spaces.
        if !plain.trim().is_empty() {
            result.push_str(plain);
        }
        let tag = &remaining[open..];
        let close = tag.find('>')?;
        result.push_str(&tag[..=close]);
        remaining = &tag[close + 1..];
    }
    if !remaining.trim().is_empty() {
        return None;
    }
    Some(
        result
            .replace("<array />", "<array/>")
            .replace("<array/>", "<array></array>")
            .replace("<true />", "<true/>"),
    )
}
fn owned_source(source: &Source, name: &str, executable: &Path, home: &Path, os: Os) -> bool {
    if crate::fs_safe::validate_portable_filename(name).is_err() {
        return false;
    }
    let Some(path) = executable.to_str() else {
        return false;
    };
    let Ok(argument) = crate::startup::executable_path(path, os == Os::Windows) else {
        return false;
    };
    match source {
        Source::Run(value)
            if os == Os::Windows
                && value.key == RUN_KEY
                && value.name.eq_ignore_ascii_case(name) =>
        {
            let Ok(command) = support::registry_text(value) else {
                return false;
            };
            crate::startup::remove_owned_command(
                &command,
                &format!("{argument} --autostart"),
                &mut || Ok(()),
            )
            .unwrap_or(false)
        }
        Source::File(config) => {
            let Ok(expected_root) = folder(os, home) else {
                return false;
            };
            let extension = if os == Os::Macos { "plist" } else { "desktop" };
            if config.root != expected_root
                || config.path != expected_root.join(format!("{name}.{extension}"))
            {
                return false;
            }
            match (&config.style, os) {
                (FileStyle::DesktopEntry, Os::Linux) => {
                    desktop_edit(&config.original).is_ok_and(|(label, command, _, _)| {
                        label == name && command == format!("{argument} --autostart")
                    })
                }
                (FileStyle::LaunchAgent, Os::Macos) => {
                    let Ok(text) = std::str::from_utf8(&config.original) else {
                        return false;
                    };
                    let Some(actual) = compact_agent_xml(text) else {
                        return false;
                    };
                    // The pinned, approved auto-launch adapter writes this
                    // minimal Label/ProgramArguments/RunAtLoad structure. Match
                    // its exact executable and sole argument without interpreting
                    // arbitrary plists, resolving entities or invoking launchctl.
                    ["", "<key>AssociatedBundleIdentifiers</key><array></array>"].into_iter().any(|associated| {
                        actual == format!("<plist version=\"1.0\"><dict><key>Label</key><string>{name}</string>{associated}<key>ProgramArguments</key><array><string>{argument}</string><string>--autostart</string></array><key>RunAtLoad</key><true/></dict></plist>")
                    })
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn folder(os: Os, home: &Path) -> CoreResult<PathBuf> {
    match os {
        Os::Macos => Ok(home.join("Library/LaunchAgents")),
        Os::Linux => Ok(home.join(".config/autostart")),
        _ => Err(CoreError::Unsupported(os.display_name())),
    }
}
fn current_home() -> CoreResult<PathBuf> {
    dirs::home_dir().ok_or_else(|| invalid("Your current-user home folder is unavailable."))
}
fn current_folder(os: Os, home: &Path) -> CoreResult<PathBuf> {
    if os == Os::Linux {
        return dirs::config_dir()
            .map(|p| p.join("autostart"))
            .ok_or_else(|| invalid("Your current-user XDG configuration folder is unavailable."));
    }
    folder(os, home)
}
fn desktop_edit(bytes: &[u8]) -> CoreResult<(String, String, bool, Vec<u8>)> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| invalid("The startup desktop file is not UTF-8."))?;
    if text.contains('\0') {
        return Err(invalid("The startup file contains a NUL character."));
    }
    let mut in_entry = false;
    let mut existing_hidden = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_entry = line == "[Desktop Entry]";
        } else if in_entry
            && line
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "Hidden")
        {
            existing_hidden = true;
        }
    }
    let mut active = false;
    let mut groups = 0;
    let mut name = None;
    let mut command = None;
    let mut kind = None;
    let mut hidden = None;
    let mut output = String::with_capacity(text.len() + 32);
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            active = trimmed == "[Desktop Entry]";
            if active {
                groups += 1;
            }
            output.push_str(line);
            if active && !existing_hidden {
                if !line.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("Hidden=true\n");
            }
            continue;
        }
        let pair = active.then(|| trimmed.split_once('=')).flatten();
        if let Some((key, value)) = pair {
            match key.trim() {
                "Name" if name.replace(value.trim().to_owned()).is_some() => {
                    return Err(invalid("Duplicate startup Name field."));
                }
                "Exec" if command.replace(value.trim().to_owned()).is_some() => {
                    return Err(invalid("Duplicate startup Exec field."));
                }
                "Type" if kind.replace(value.trim().to_owned()).is_some() => {
                    return Err(invalid("Duplicate startup Type field."));
                }
                "Hidden" => {
                    let value = match value.trim() {
                        "true" => true,
                        "false" => false,
                        _ => return Err(invalid("Unsupported startup Hidden value.")),
                    };
                    if hidden.replace(value).is_some() {
                        return Err(invalid("Duplicate startup Hidden field."));
                    }
                    output.push_str("Hidden=true");
                    if line.ends_with("\r\n") {
                        output.push_str("\r\n");
                    } else if line.ends_with('\n') {
                        output.push('\n');
                    }
                    continue;
                }
                _ => {}
            }
        }
        output.push_str(line);
    }
    if groups != 1
        || kind.as_deref() != Some("Application")
        || command
            .as_ref()
            .is_none_or(|s| s.is_empty() || s.len() > 32_768)
    {
        return Err(invalid(
            "Only one ordinary Application desktop entry with an Exec field is supported.",
        ));
    }
    Ok((
        name.unwrap_or_else(|| "Unnamed startup application".into()),
        command.unwrap_or_default(),
        !hidden.unwrap_or(false),
        output.into_bytes(),
    ))
}
fn list_directory(os: Os, directory: &Path, cancel: &CancelToken) -> CoreResult<Vec<StartupEntry>> {
    cancel.check()?;
    support::local_path(directory)?;
    let reader = match fs::read_dir(directory) {
        Ok(reader) => reader,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    let mut entries = Vec::new();
    let mut seen = 0;
    let mut total_bytes = 0_usize;
    for child in reader {
        cancel.check()?;
        seen += 1;
        if seen > 1024 {
            return Err(invalid("Startup review exceeds 1,024 directory entries."));
        }
        let child = child?;
        let path = child.path();
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file() || crate::fs_safe::is_link(&metadata) {
            continue;
        }
        let expected_extension = if os == Os::Linux { "desktop" } else { "plist" };
        if path
            .extension()
            .is_none_or(|extension| extension != expected_extension)
        {
            continue;
        }
        let original = support::read_bounded(&path, MAX_CONFIG_BYTES)?;
        total_bytes = total_bytes.saturating_add(original.len());
        if total_bytes > 16 * 1024 * 1024 {
            return Err(invalid("Startup configuration review exceeds 16 MiB."));
        }
        let (name, command, enabled, disabled, style, parked, note) = if os == Os::Linux {
            let (name, command, enabled, disabled) = desktop_edit(&original)?;
            (
                name,
                command,
                enabled,
                disabled,
                FileStyle::DesktopEntry,
                None,
                "Current-user XDG autostart only. Hidden=true keeps this override in place; active programs are not stopped. Desktop conditions and OS controls can also disable it.",
            )
        } else {
            let name = child.file_name().to_string_lossy().into_owned();
            let parked = directory.join(format!(
                ".{name}.oxbox-disabled-{}",
                &blake3::hash(&original).to_hex()[..16]
            ));
            (
                name,
                "LaunchAgent configuration; the program is not executed or probed.".into(),
                true,
                Vec::new(),
                FileStyle::LaunchAgent,
                Some(parked),
                "Only this user's legacy LaunchAgent file is managed. Disable parks it for future logins; loaded jobs and modern Login Items/OS overrides are unchanged.",
            )
        };
        entries.push(StartupEntry {
            name,
            command,
            enabled,
            location: path.display().to_string(),
            note: note.into(),
            source: Source::File(FileConfig {
                style,
                root: directory.to_path_buf(),
                path,
                original,
                disabled,
                parked,
            }),
        });
    }
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    Ok(entries)
}
#[cfg(test)]
fn list_files(os: Os, home: &Path, cancel: &CancelToken) -> CoreResult<Vec<StartupEntry>> {
    list_directory(os, &folder(os, home)?, cancel)
}
pub fn list(
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<StartupEntry>> {
    cancel.check()?;
    let os = Os::current();
    let mut entries = if os == Os::Windows {
        NativeRegistry.list(RegistryArea::Run)?.into_iter().map(|value| {
            let command = support::registry_text(&value)?;
            Ok(StartupEntry { name: value.name.clone(), location: format!("HKCU\\{}\\{}", value.key, value.name), command, enabled: true,
                note: "Current-user Run registration only. Windows may separately disable/delay it. Disable removes this exact value; running programs and StartupApproved are untouched.".into(), source: Source::Run(value) })
        }).collect::<CoreResult<Vec<_>>>()?
    } else {
        let home = current_home()?;
        list_directory(os, &current_folder(os, &home)?, cancel)?
    };
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    progress(Progress {
        done: entries.len() as u64,
        total: entries.len() as u64,
        message: "Current-user startup review complete; no settings changed".into(),
    });
    Ok(entries)
}
fn validate_config(config: &FileConfig) -> CoreResult<()> {
    support::local_path(&config.root)?;
    support::local_path(&config.path)?;
    let filename = config
        .path
        .file_name()
        .ok_or_else(|| invalid("Invalid startup filename."))?;
    crate::fs_safe::validate_output_filename(filename)?;
    if config.path.parent() != Some(config.root.as_path())
        || config.original.len() > MAX_CONFIG_BYTES
        || config.disabled.len() > MAX_CONFIG_BYTES
    {
        return Err(invalid(
            "The startup configuration is outside its reviewed user folder.",
        ));
    }
    match config.style {
        FileStyle::DesktopEntry => {
            if config
                .path
                .extension()
                .is_none_or(|extension| extension != "desktop")
                || config.parked.is_some()
                || desktop_edit(&config.original)?.3 != config.disabled
            {
                return Err(invalid(
                    "The desktop-entry backup does not match its exact disabled form.",
                ));
            }
        }
        FileStyle::LaunchAgent => {
            let expected = config.root.join(format!(
                ".{}.oxbox-disabled-{}",
                filename.to_string_lossy(),
                &blake3::hash(&config.original).to_hex()[..16]
            ));
            if config
                .path
                .extension()
                .is_none_or(|extension| extension != "plist")
                || config.parked.as_ref() != Some(&expected)
                || !config.disabled.is_empty()
            {
                return Err(invalid("The LaunchAgent recovery location is invalid."));
            }
        }
    }
    Ok(())
}
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
fn replace_exact(path: &Path, expected: &[u8], replacement: &[u8]) -> CoreResult<()> {
    if support::read_bounded(path, MAX_CONFIG_BYTES)? != expected {
        return Err(invalid("The startup file changed; review it again."));
    }
    let permissions = fs::metadata(path)?.permissions();
    let temporary = path.with_file_name(format!(
        ".oxbox-startup-{}-{}.tmp",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    support::write_new(&temporary, replacement)?;
    fs::set_permissions(&temporary, permissions)?;
    if support::read_bounded(path, MAX_CONFIG_BYTES)? != expected {
        return Err(invalid(
            "The startup file changed; the temporary replacement was kept for inspection.",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // Explicit reviewed replacement; no unreviewed destination is selected.
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    fs::rename(&temporary, path)?;
    if support::read_bounded(path, MAX_CONFIG_BYTES)? != replacement {
        return Err(invalid(
            "Startup readback differs; keep the recovery backup and inspect the file.",
        ));
    }
    Ok(())
}
fn move_exact_no_replace(source: &Path, destination: &Path, expected: &[u8]) -> CoreResult<()> {
    if support::read_bounded(source, MAX_CONFIG_BYTES)? != expected {
        return Err(invalid("The startup file changed; review it again."));
    }
    support::local_path(destination)?;
    // hard_link creates the new name exclusively, unlike Unix rename which can
    // overwrite an unrelated destination. Both names remain in the same folder.
    fs::hard_link(source, destination)?;
    if support::read_bounded(source, MAX_CONFIG_BYTES)? != expected
        || support::read_bounded(destination, MAX_CONFIG_BYTES)? != expected
    {
        return Err(invalid(
            "The startup file changed during parking. Both names were kept; inspect the backup.",
        ));
    }
    fs::remove_file(source)?;
    Ok(())
}
fn disable_file(
    entry: &StartupEntry,
    backup: &Path,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    cancel.check()?;
    if !confirmed || !entry.enabled {
        return Err(invalid("Confirm an enabled current-user startup entry."));
    }
    let Source::File(config) = &entry.source else {
        return Err(invalid("This is not a startup file."));
    };
    validate_config(config)?;
    if support::read_bounded(&config.path, MAX_CONFIG_BYTES)? != config.original {
        return Err(invalid(
            "The startup configuration changed; refresh and review again.",
        ));
    }
    support::write_backup(
        backup,
        &Backup {
            schema: 1,
            purpose: "oxbox-startup".into(),
            source: entry.source.clone(),
        },
    )?;
    cancel.check()?;
    match config.style {
        FileStyle::DesktopEntry => replace_exact(&config.path, &config.original, &config.disabled),
        FileStyle::LaunchAgent => move_exact_no_replace(
            &config.path,
            config
                .parked
                .as_ref()
                .ok_or_else(|| invalid("Missing LaunchAgent parking path."))?,
            &config.original,
        ),
    }
}
fn disable_run(
    entry: &StartupEntry,
    backup: &Path,
    confirmed: bool,
    cancel: &CancelToken,
    backend: &mut dyn RegistryBackend,
) -> CoreResult<()> {
    cancel.check()?;
    let Source::Run(value) = &entry.source else {
        return Err(invalid("This is not a current-user Run entry."));
    };
    support::validate_registry_value(value)?;
    if !confirmed || value.key != RUN_KEY || backend.read(value)?.as_ref() != Some(value) {
        return Err(invalid("Confirm the unchanged current-user Run value."));
    }
    support::write_backup(
        backup,
        &Backup {
            schema: 1,
            purpose: "oxbox-startup".into(),
            source: entry.source.clone(),
        },
    )?;
    cancel.check()?;
    backend.change(Some(value), None)
}
pub fn disable(
    entry: &StartupEntry,
    backup: &Path,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    match &entry.source {
        Source::Run(_) if Os::current() == Os::Windows => {
            disable_run(entry, backup, confirmed, cancel, &mut NativeRegistry)
        }
        Source::File(config) => {
            let home = current_home()?;
            let platform_matches = matches!(
                (&config.style, Os::current()),
                (FileStyle::LaunchAgent, Os::Macos) | (FileStyle::DesktopEntry, Os::Linux)
            );
            if !platform_matches || config.root != current_folder(Os::current(), &home)? {
                return Err(invalid(
                    "The entry is not in this user's platform startup folder.",
                ));
            }
            disable_file(entry, backup, confirmed, cancel)
        }
        _ => Err(CoreError::Unsupported(Os::current().display_name())),
    }
}
fn load_recovery(path: &Path) -> CoreResult<Backup> {
    let backup: Backup = support::read_backup(path)?;
    if backup.schema != 1 || backup.purpose != "oxbox-startup" {
        return Err(invalid("Choose an Oxbox startup recovery backup."));
    }
    match &backup.source {
        Source::Run(value) => {
            support::validate_registry_value(value)?;
            if value.key != RUN_KEY {
                return Err(invalid("The backup is not a current-user Run value."));
            }
        }
        Source::File(config) => validate_config(config)?,
    }
    Ok(backup)
}
pub fn review_recovery(path: &Path) -> CoreResult<RecoveryReview> {
    let backup = load_recovery(path)?;
    let description = match &backup.source {
        Source::Run(value) => format!(
            "Restore current-user Run value {} with exact command:\n{}\nOnly an absent value can be restored. Other registry data is kept.",
            value.name,
            support::registry_text(value)?
        ),
        Source::File(config) => format!(
            "Restore the original current-user startup configuration at {}? Only the exact Oxbox-disabled file/parked configuration is accepted. This affects future logins and does not start a process.",
            config.path.display()
        ),
    };
    Ok(RecoveryReview {
        description,
        path: path.to_path_buf(),
        saved: backup,
    })
}
pub fn recovery_description(path: &Path) -> CoreResult<String> {
    Ok(review_recovery(path)?.description)
}
#[cfg(test)]
fn restore_file_at(
    backup: &Path,
    directory: &Path,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    restore_reviewed_file_at(&review_recovery(backup)?, directory, confirmed, cancel)
}
fn restore_reviewed_file_at(
    review: &RecoveryReview,
    directory: &Path,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid(
            "Confirm the exact startup backup before restoring.",
        ));
    }
    if load_recovery(&review.path)? != review.saved {
        return Err(invalid(
            "The recovery backup changed after review. Review it again.",
        ));
    }
    let Source::File(config) = &review.saved.source else {
        return Err(invalid("Choose a startup-file backup."));
    };
    if config.root != directory {
        return Err(invalid(
            "The backup does not belong to this user's current startup folder.",
        ));
    }
    cancel.check()?;
    match config.style {
        FileStyle::DesktopEntry => replace_exact(&config.path, &config.disabled, &config.original),
        FileStyle::LaunchAgent => move_exact_no_replace(
            config
                .parked
                .as_ref()
                .ok_or_else(|| invalid("Missing parked LaunchAgent."))?,
            &config.path,
            &config.original,
        ),
    }
}
#[cfg(test)]
fn restore_file(
    backup: &Path,
    os: Os,
    home: &Path,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    restore_file_at(backup, &folder(os, home)?, confirmed, cancel)
}
pub fn restore(backup: &Path, confirmed: bool, cancel: &CancelToken) -> CoreResult<()> {
    restore_reviewed(&review_recovery(backup)?, confirmed, cancel)
}
pub fn restore_reviewed(
    review: &RecoveryReview,
    confirmed: bool,
    cancel: &CancelToken,
) -> CoreResult<()> {
    cancel.check()?;
    if !confirmed {
        return Err(invalid(
            "Confirm the exact startup backup before restoring.",
        ));
    }
    if load_recovery(&review.path)? != review.saved {
        return Err(invalid(
            "The recovery backup changed after review. Review it again.",
        ));
    }
    match &review.saved.source {
        Source::Run(value) if Os::current() == Os::Windows => {
            if NativeRegistry.read(value)?.is_some() {
                return Err(invalid(
                    "The startup value already exists; it will not be overwritten.",
                ));
            }
            cancel.check()?;
            NativeRegistry.change(None, Some(value))
        }
        Source::File(config)
            if matches!(
                (&config.style, Os::current()),
                (FileStyle::LaunchAgent, Os::Macos) | (FileStyle::DesktopEntry, Os::Linux)
            ) =>
        {
            let home = current_home()?;
            restore_reviewed_file_at(review, &current_folder(Os::current(), &home)?, true, cancel)
        }
        _ => Err(CoreError::Unsupported(Os::current().display_name())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{jobs::CancelToken, platform::Os};

    fn owned_fixture(
        os: Os,
        home: &Path,
        filename: &str,
        label: &str,
        executable: &Path,
        extra: bool,
    ) -> StartupEntry {
        let directory = folder(os, home).unwrap();
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(filename);
        let executable = executable.to_str().unwrap();
        let text = if os == Os::Linux {
            format!(
                "[Desktop Entry]\nType=Application\nName={label}\nExec={executable} --autostart{}\nStartupNotify=false\nTerminal=false",
                if extra { " --custom" } else { "" }
            )
        } else {
            format!(
                "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>Label</key><string>{label}</string><key>ProgramArguments</key><array><string>{executable}</string><string>--autostart</string>{}</array><key>RunAtLoad</key><true/></dict></plist>",
                if extra {
                    "<string>--custom</string>"
                } else {
                    ""
                }
            )
        };
        std::fs::write(&path, text).unwrap();
        list_files(os, home, &CancelToken::default())
            .unwrap()
            .into_iter()
            .find(|entry| entry.location == path.display().to_string())
            .unwrap()
    }

    #[test]
    fn startup_exact_current_app_files_and_backups_delegate_to_settings_on_macos_and_linux() {
        for os in [Os::Macos, Os::Linux] {
            let home = tempfile::tempdir().unwrap();
            let executable = home.path().join("bin/oxbox-native");
            let filename = if os == Os::Macos {
                "Oxbox.plist"
            } else {
                "Oxbox.desktop"
            };
            let entry = owned_fixture(os, home.path(), filename, "Oxbox", &executable, false);
            assert!(entry.belongs_to_application("Oxbox", &executable, home.path(), os));
            assert!(!entry.belongs_to_application(
                "Oxbox",
                &executable,
                &home.path().join("other-user"),
                os
            ));
            let backup = home.path().join("owned-backup.json");
            disable_file(&entry, &backup, true, &CancelToken::default()).unwrap();
            let review = review_recovery(&backup).unwrap();
            assert!(review.belongs_to_application("Oxbox", &executable, home.path(), os));
            for (index, (name, label, target, extra)) in [
                (filename, "Oxbox", home.path().join("different-app"), false),
                (filename, "Oxbox", executable.clone(), true),
                (filename, "Different label", executable.clone(), false),
                (
                    if os == Os::Macos {
                        "Other.plist"
                    } else {
                        "Other.desktop"
                    },
                    "Oxbox",
                    executable.clone(),
                    false,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let case_home = tempfile::tempdir().unwrap();
                let entry = owned_fixture(os, case_home.path(), name, label, &target, extra);
                assert!(
                    !entry.belongs_to_application("Oxbox", &executable, case_home.path(), os),
                    "repurposed startup fixture {index} was misidentified"
                );
                let case_backup = case_home.path().join("other-backup.json");
                disable_file(&entry, &case_backup, true, &CancelToken::default()).unwrap();
                let review = review_recovery(&case_backup).unwrap();
                assert!(!review.belongs_to_application("Oxbox", &executable, case_home.path(), os));
            }
        }
    }

    #[test]
    fn startup_linux_disable_preserves_override_and_restores_only_its_exact_edit() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join(".config/autostart");
        std::fs::create_dir_all(&directory).unwrap();
        let filename = directory.join("example.desktop");
        let original = b"[Desktop Entry]\nType=Application\nName=Example\nExec=/usr/bin/example --literal\nHidden=false\n[Other Group]\nHidden=false\n";
        std::fs::write(&filename, original).unwrap();
        let entries = list_files(Os::Linux, root.path(), &CancelToken::default()).unwrap();
        assert_eq!(entries.len(), 1);
        let backup = root.path().join("startup.json");
        disable_file(&entries[0], &backup, true, &CancelToken::default()).unwrap();
        let changed = std::fs::read_to_string(&filename).unwrap();
        assert!(changed.contains("Hidden=true\n[Other Group]\nHidden=false"));
        assert!(
            filename.exists(),
            "deleting a user override could re-enable a system entry"
        );
        restore_file(
            &backup,
            Os::Linux,
            root.path(),
            true,
            &CancelToken::default(),
        )
        .unwrap();
        assert_eq!(std::fs::read(&filename).unwrap(), original);
        assert!(disable_file(&entries[0], &backup, true, &CancelToken::default()).is_err());
    }

    #[test]
    fn startup_review_rejects_changed_file_cancel_and_unconfirmed_mutation() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("Library/LaunchAgents");
        std::fs::create_dir_all(&directory).unwrap();
        let filename = directory.join("example.plist");
        std::fs::write(&filename, b"generated plist fixture").unwrap();
        let entries = list_files(Os::Macos, root.path(), &CancelToken::default()).unwrap();
        let backup = root.path().join("startup.json");
        assert!(disable_file(&entries[0], &backup, false, &CancelToken::default()).is_err());
        let cancel = CancelToken::default();
        cancel.cancel();
        assert_eq!(
            disable_file(&entries[0], &backup, true, &cancel)
                .unwrap_err()
                .code(),
            "cancelled"
        );
        std::fs::write(&filename, b"new config from another app").unwrap();
        assert!(disable_file(&entries[0], &backup, true, &CancelToken::default()).is_err());
        assert!(!backup.exists());
    }
    #[test]
    fn startup_restore_review_rejects_backup_changed_after_confirmation_preview() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join(".config/autostart");
        std::fs::create_dir_all(&directory).unwrap();
        let filename = directory.join("example.desktop");
        std::fs::write(
            &filename,
            b"[Desktop Entry]\nType=Application\nName=Example\nExec=/usr/bin/example\n",
        )
        .unwrap();
        let entries = list_files(Os::Linux, root.path(), &CancelToken::default()).unwrap();
        let backup = root.path().join("backup.json");
        disable_file(&entries[0], &backup, true, &CancelToken::default()).unwrap();
        let review = review_recovery(&backup).unwrap();
        let disabled = std::fs::read(&filename).unwrap();
        std::fs::write(&backup, b"{}").unwrap();
        assert!(
            restore_reviewed_file_at(&review, &directory, true, &CancelToken::default()).is_err()
        );
        assert_eq!(std::fs::read(&filename).unwrap(), disabled);
    }
}

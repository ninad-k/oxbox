//! Bounded exact snapshots and exclusive recovery backups for maintenance tools.

use crate::{CoreError, CoreResult, fs_safe};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub(crate) const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
pub(crate) const APP_PATHS_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths";
pub(crate) const MAX_CONFIG_BYTES: usize = 262_144;
const MAX_BACKUP_BYTES: usize = 2_097_152;
pub(crate) fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

pub(crate) fn local_path(path: &Path) -> CoreResult<()> {
    if !path.is_absolute() {
        return Err(invalid("Choose an absolute local path."));
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let letter = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
                _ => return Err(invalid("Network and device paths are not allowed.")),
            },
            _ => return Err(invalid("Choose a local drive path.")),
        };
        let drive = [u16::from(letter), b':' as u16, b'\\' as u16, 0];
        // SAFETY: the local drive prefix is a terminated UTF-16 string.
        if !matches!(
            unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(drive.as_ptr()) },
            2 | 3 | 6
        ) {
            return Err(invalid(
                "Remote, unavailable and unknown drives are not allowed.",
            ));
        }
    }
    fs_safe::reject_symlink_components(path)
}

pub(crate) fn read_bounded(path: &Path, limit: usize) -> CoreResult<Vec<u8>> {
    local_path(path)?;
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || fs_safe::is_link(&metadata) || metadata.len() > limit as u64 {
        return Err(invalid(
            "The selected configuration is not a bounded regular file.",
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut buffer = [0_u8; 8192];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        if bytes.len().saturating_add(size) > limit {
            return Err(invalid(
                "The selected configuration exceeds its size limit.",
            ));
        }
        bytes.extend_from_slice(&buffer[..size]);
    }
    Ok(bytes)
}

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    local_path(path)?;
    fs_safe::validate_output_filename(
        path.file_name()
            .ok_or_else(|| invalid("Choose a backup filename."))?,
    )?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Choose a backup folder."))?;
    if !parent.is_dir() {
        return Err(invalid("The backup folder must already exist."));
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    if read_bounded(path, bytes.len())? != bytes {
        return Err(invalid(
            "Backup verification failed; no reviewed setting was changed.",
        ));
    }
    Ok(())
}
pub(crate) fn write_backup<T: Serialize>(path: &Path, value: &T) -> CoreResult<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| invalid("The recovery backup could not be encoded."))?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(invalid("The recovery backup is too large."));
    }
    write_new(path, &bytes)
}
pub(crate) fn read_backup<T: DeserializeOwned>(path: &Path) -> CoreResult<T> {
    serde_json::from_slice(&read_bounded(path, MAX_BACKUP_BYTES)?)
        .map_err(|_| invalid("The selected recovery backup is not valid."))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RegistryValue {
    pub key: String,
    pub name: String,
    pub kind: u32,
    pub data: Vec<u8>,
}
#[derive(Clone, Copy)]
pub(crate) enum RegistryArea {
    Run,
    AppPaths,
}
pub(crate) trait RegistryBackend {
    fn list(&mut self, area: RegistryArea) -> CoreResult<Vec<RegistryValue>>;
    fn read(&mut self, value: &RegistryValue) -> CoreResult<Option<RegistryValue>>;
    fn change(
        &mut self,
        expected: Option<&RegistryValue>,
        desired: Option<&RegistryValue>,
    ) -> CoreResult<()>;
}
pub(crate) fn validate_registry_value(value: &RegistryValue) -> CoreResult<()> {
    let app_leaf = value.key.strip_prefix(&format!("{APP_PATHS_KEY}\\"));
    let app = app_leaf.is_some_and(|leaf| {
        !leaf.is_empty()
            && leaf.len() <= 200
            && !leaf.contains(['\\', '/'])
            && leaf.to_ascii_lowercase().ends_with(".exe")
            && fs_safe::validate_portable_filename(leaf).is_ok()
            && value.name.is_empty()
    });
    if (value.key != RUN_KEY && !app)
        || value.name.len() > 16_383
        || value.name.contains('\0')
        || !matches!(value.kind, 1 | 2)
        || value.data.len() > 65_536
        || !value.data.len().is_multiple_of(2)
    {
        return Err(invalid(
            "Only reviewed current-user Run values or default App Paths executable mappings are supported.",
        ));
    }
    Ok(())
}
pub(crate) fn registry_text(value: &RegistryValue) -> CoreResult<String> {
    validate_registry_value(value)?;
    let mut words: Vec<_> = value
        .data
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    if words.last() == Some(&0) {
        words.pop();
    }
    if words.contains(&0) {
        return Err(invalid(
            "The registry text contains an embedded terminator.",
        ));
    }
    String::from_utf16(&words).map_err(|_| invalid("The registry text is not valid UTF-16."))
}

#[derive(Default)]
pub(crate) struct NativeRegistry;
#[cfg(not(windows))]
impl RegistryBackend for NativeRegistry {
    fn list(&mut self, _: RegistryArea) -> CoreResult<Vec<RegistryValue>> {
        Err(CoreError::Unsupported(
            crate::platform::Os::current().display_name(),
        ))
    }
    fn read(&mut self, _: &RegistryValue) -> CoreResult<Option<RegistryValue>> {
        Err(CoreError::Unsupported(
            crate::platform::Os::current().display_name(),
        ))
    }
    fn change(&mut self, _: Option<&RegistryValue>, _: Option<&RegistryValue>) -> CoreResult<()> {
        Err(CoreError::Unsupported(
            crate::platform::Os::current().display_name(),
        ))
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
        System::Registry::*,
    };
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn status(code: u32) -> CoreError {
        std::io::Error::from_raw_os_error(code as i32).into()
    }
    fn raw_value(key: HKEY, name: &str) -> CoreResult<Option<(u32, Vec<u8>)>> {
        let name = wide(name);
        let mut kind = 0;
        let mut size = 0;
        let result = unsafe {
            RegQueryValueExW(key, name.as_ptr(), null(), &mut kind, null_mut(), &mut size)
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if result != ERROR_SUCCESS {
            return Err(status(result));
        }
        if size > 65_536 {
            return Err(invalid("The registry value exceeds the review limit."));
        }
        let mut bytes = vec![0; size as usize];
        let result = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                null(),
                &mut kind,
                bytes.as_mut_ptr(),
                &mut size,
            )
        };
        if result != ERROR_SUCCESS {
            return Err(status(result));
        }
        bytes.truncate(size as usize);
        Ok(Some((kind, bytes)))
    }
    fn open(path: &str, writable: bool, create: bool) -> CoreResult<Key> {
        let components: Vec<_> = path.split('\\').collect();
        let mut parent = HKEY_CURRENT_USER;
        let mut owned = None;
        for (index, component) in components.iter().enumerate() {
            let last = index + 1 == components.len();
            let name = wide(component);
            let mut output = null_mut();
            let rights =
                KEY_READ | KEY_WOW64_64KEY | if last && writable { KEY_SET_VALUE } else { 0 };
            let mut result = unsafe {
                RegOpenKeyExW(
                    parent,
                    name.as_ptr(),
                    REG_OPTION_OPEN_LINK,
                    rights,
                    &mut output,
                )
            };
            if result == ERROR_FILE_NOT_FOUND && last && create {
                let mut disposition = 0;
                result = unsafe {
                    RegCreateKeyExW(
                        parent,
                        name.as_ptr(),
                        0,
                        null(),
                        REG_OPTION_NON_VOLATILE,
                        rights,
                        null(),
                        &mut output,
                        &mut disposition,
                    )
                };
            }
            if result != ERROR_SUCCESS {
                return Err(status(result));
            }
            let key = Key(output);
            if raw_value(key.0, "SymbolicLinkValue")?.is_some_and(|(kind, _)| kind == REG_LINK) {
                return Err(invalid("Registry symbolic-link keys are not supported."));
            }
            parent = key.0;
            owned = Some(key);
        }
        owned.ok_or_else(|| invalid("Invalid registry location."))
    }
    impl RegistryBackend for NativeRegistry {
        fn list(&mut self, area: RegistryArea) -> CoreResult<Vec<RegistryValue>> {
            let path = match area {
                RegistryArea::Run => RUN_KEY,
                RegistryArea::AppPaths => APP_PATHS_KEY,
            };
            let key = match open(path, false, false) {
                Ok(key) => key,
                Err(CoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Vec::new());
                }
                Err(e) => return Err(e),
            };
            let mut values = Vec::new();
            for index in 0..=1024 {
                let mut name = [0_u16; 16_384];
                let mut length = name.len() as u32;
                let result = match area {
                    RegistryArea::Run => unsafe {
                        RegEnumValueW(
                            key.0,
                            index,
                            name.as_mut_ptr(),
                            &mut length,
                            null(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                        )
                    },
                    RegistryArea::AppPaths => unsafe {
                        RegEnumKeyExW(
                            key.0,
                            index,
                            name.as_mut_ptr(),
                            &mut length,
                            null(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                        )
                    },
                };
                if result == ERROR_NO_MORE_ITEMS {
                    return Ok(values);
                }
                if result != ERROR_SUCCESS {
                    return Err(status(result));
                }
                if index == 1024 {
                    return Err(invalid(
                        "Startup/registry review exceeds 1,024 entries; no changes were made.",
                    ));
                }
                let name = String::from_utf16(&name[..length as usize])
                    .map_err(|_| invalid("Invalid registry entry name."))?;
                let (entry_key, entry_name) = match area {
                    RegistryArea::Run => (path.into(), name),
                    RegistryArea::AppPaths => (format!("{path}\\{name}"), String::new()),
                };
                let identity = RegistryValue {
                    key: entry_key,
                    name: entry_name,
                    kind: 1,
                    data: vec![],
                };
                if validate_registry_value(&identity).is_err() {
                    continue;
                }
                if let Some(value) = self.read(&identity)?
                    && matches!(value.kind, 1 | 2)
                {
                    values.push(value);
                }
            }
            Ok(values)
        }
        fn read(&mut self, value: &RegistryValue) -> CoreResult<Option<RegistryValue>> {
            validate_registry_value(value)?;
            let key = match open(&value.key, false, false) {
                Ok(key) => key,
                Err(CoreError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            Ok(
                raw_value(key.0, &value.name)?.map(|(kind, data)| RegistryValue {
                    key: value.key.clone(),
                    name: value.name.clone(),
                    kind,
                    data,
                }),
            )
        }
        fn change(
            &mut self,
            expected: Option<&RegistryValue>,
            desired: Option<&RegistryValue>,
        ) -> CoreResult<()> {
            let identity = expected
                .or(desired)
                .ok_or_else(|| invalid("No reviewed registry value was supplied."))?;
            validate_registry_value(identity)?;
            if let Some(value) = desired {
                validate_registry_value(value)?;
                if value.key != identity.key || value.name != identity.name {
                    return Err(invalid("Registry restoration identity changed."));
                }
            }
            if self.read(identity)?.as_ref() != expected {
                return Err(invalid(
                    "The registry entry changed; refresh and review again.",
                ));
            }
            let key = open(&identity.key, true, desired.is_some())?;
            let fresh = raw_value(key.0, &identity.name)?;
            if fresh
                .as_ref()
                .map(|(kind, bytes)| (*kind, bytes.as_slice()))
                != expected.map(|value| (value.kind, value.data.as_slice()))
            {
                return Err(invalid(
                    "The registry entry changed before the reviewed operation.",
                ));
            }
            let name = wide(&identity.name);
            let result = if let Some(value) = desired {
                unsafe {
                    RegSetValueExW(
                        key.0,
                        name.as_ptr(),
                        0,
                        value.kind,
                        value.data.as_ptr(),
                        value.data.len() as u32,
                    )
                }
            } else {
                unsafe { RegDeleteValueW(key.0, name.as_ptr()) }
            };
            if result != ERROR_SUCCESS {
                return Err(status(result));
            }
            let expected_after = desired.map(|value| (value.kind, value.data.clone()));
            if raw_value(key.0, &identity.name)? != expected_after {
                return Err(invalid(
                    "The registry readback did not match. Keep the recovery backup and inspect the entry.",
                ));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintenance_backups_are_exclusive_bounded_and_round_trip_exact_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("backup.json");
        write_backup(&path, &vec![0_u8, 1, 255]).unwrap();
        assert_eq!(read_backup::<Vec<u8>>(&path).unwrap(), vec![0, 1, 255]);
        assert!(write_backup(&path, &vec![2_u8]).is_err());
        assert_eq!(read_backup::<Vec<u8>>(&path).unwrap(), vec![0, 1, 255]);
    }

    #[test]
    fn maintenance_registry_scope_refuses_system_keys_and_unrelated_user_values() {
        assert!(
            validate_registry_value(&RegistryValue {
                key: RUN_KEY.into(),
                name: "Example".into(),
                kind: 1,
                data: vec![0, 0]
            })
            .is_ok()
        );
        assert!(
            validate_registry_value(&RegistryValue {
                key: format!("{APP_PATHS_KEY}\\example.exe"),
                name: String::new(),
                kind: 1,
                data: vec![0, 0]
            })
            .is_ok()
        );
        for key in [
            "HKEY_LOCAL_MACHINE\\Software\\Example",
            "Software\\Classes\\CLSID",
            "Software\\Other",
            "Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\x.exe\\nested",
        ] {
            assert!(
                validate_registry_value(&RegistryValue {
                    key: key.into(),
                    name: String::new(),
                    kind: 1,
                    data: vec![0, 0]
                })
                .is_err()
            );
        }
    }
}

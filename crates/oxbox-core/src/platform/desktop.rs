//! Native desktop adapters for the Phase 5 window, display and app tools.
use crate::tools::{
    eject_volumes::Volume,
    energy_saver::PowerProfile,
    resolution::{Display, DisplayMode},
    uninstall::InstalledApp,
    window_manager::Rect,
    window_top::WindowInfo,
};
use crate::{CoreError, CoreResult};

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        path::PathBuf,
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Storage::FileSystem::*,
        System::{IO::DeviceIoControl, Ioctl::*, Registry::*},
        UI::{Shell::*, WindowsAndMessaging::*},
    };
    use windows_sys::core::BOOL;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }
    fn text(value: &[u16]) -> String {
        String::from_utf16_lossy(
            &value[..value.iter().position(|c| *c == 0).unwrap_or(value.len())],
        )
    }
    fn os_error() -> CoreError {
        std::io::Error::last_os_error().into()
    }
    fn hwnd(id: u64) -> CoreResult<HWND> {
        let raw = usize::try_from(id)
            .map_err(|_| CoreError::InvalidInput("invalid window identifier".into()))?
            as HWND;
        if raw.is_null() || unsafe { IsWindow(raw) } == 0 {
            return Err(CoreError::InvalidInput(
                "the selected window is no longer available".into(),
            ));
        }
        Ok(raw)
    }

    pub fn window(id: u64) -> CoreResult<WindowInfo> {
        let raw = hwnd(id)?;
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(raw, &mut pid) };
        if pid == std::process::id() || pid == 0 {
            return Err(CoreError::InvalidInput(
                "Oxbox and system windows are excluded".into(),
            ));
        }
        let len = unsafe { GetWindowTextLengthW(raw) };
        if !(1..=16384).contains(&len) {
            return Err(CoreError::InvalidInput(
                "the selected window has no application title".into(),
            ));
        }
        let mut title = vec![0_u16; len as usize + 1];
        let count = unsafe { GetWindowTextW(raw, title.as_mut_ptr(), title.len() as i32) };
        if count <= 0 {
            return Err(os_error());
        }
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(raw, &mut rect) } == 0 {
            return Err(os_error());
        }
        let width = u32::try_from(i64::from(rect.right) - i64::from(rect.left))
            .map_err(|_| CoreError::InvalidInput("invalid window bounds".into()))?;
        let height = u32::try_from(i64::from(rect.bottom) - i64::from(rect.top))
            .map_err(|_| CoreError::InvalidInput("invalid window bounds".into()))?;
        Ok(WindowInfo {
            id,
            pid,
            title: String::from_utf16_lossy(&title[..count as usize]),
            bounds: Rect {
                x: rect.left,
                y: rect.top,
                width,
                height,
            },
            topmost: unsafe { GetWindowLongPtrW(raw, GWL_EXSTYLE) } & WS_EX_TOPMOST as isize != 0,
            minimized: unsafe { IsIconic(raw) } != 0,
        })
    }
    unsafe extern "system" fn collect(raw: HWND, data: LPARAM) -> BOOL {
        if unsafe { IsWindowVisible(raw) } != 0
            && unsafe { GetWindow(raw, GW_OWNER) }.is_null()
            && unsafe { GetWindowLongPtrW(raw, GWL_EXSTYLE) } & WS_EX_TOOLWINDOW as isize == 0
            && let Ok(info) = window(raw as usize as u64)
        {
            let result = unsafe { &mut *(data as *mut Vec<WindowInfo>) };
            result.push(info);
        }
        1
    }
    pub fn windows() -> CoreResult<Vec<WindowInfo>> {
        let mut result: Vec<WindowInfo> = Vec::new();
        if unsafe { EnumWindows(Some(collect), &mut result as *mut Vec<WindowInfo> as isize) } == 0
        {
            return Err(os_error());
        }
        result.sort_by_key(|a| a.title.to_lowercase());
        Ok(result)
    }
    pub fn set_topmost(id: u64, enabled: bool) -> CoreResult<()> {
        if unsafe {
            SetWindowPos(
                hwnd(id)?,
                if enabled {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(os_error());
        }
        Ok(())
    }
    pub fn work_area(id: u64) -> CoreResult<Rect> {
        let monitor = unsafe { MonitorFromWindow(hwnd(id)?, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return Err(os_error());
        }
        Ok(Rect {
            x: info.rcWork.left,
            y: info.rcWork.top,
            width: (info.rcWork.right - info.rcWork.left).max(0) as u32,
            height: (info.rcWork.bottom - info.rcWork.top).max(0) as u32,
        })
    }
    pub fn move_window(id: u64, bounds: Rect) -> CoreResult<()> {
        let raw = hwnd(id)?;
        if unsafe { IsIconic(raw) } != 0 || unsafe { IsZoomed(raw) } != 0 {
            unsafe { ShowWindow(raw, SW_RESTORE) };
        }
        if unsafe {
            SetWindowPos(
                raw,
                null_mut(),
                bounds.x,
                bounds.y,
                bounds.width as i32,
                bounds.height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(os_error());
        }
        Ok(())
    }
    pub fn minimize_window(id: u64) -> CoreResult<()> {
        if unsafe { ShowWindowAsync(hwnd(id)?, SW_MINIMIZE) } == 0 {
            return Err(os_error());
        }
        Ok(())
    }
    pub fn restore_window(id: u64) -> CoreResult<()> {
        if unsafe { ShowWindowAsync(hwnd(id)?, SW_RESTORE) } == 0 {
            return Err(os_error());
        }
        Ok(())
    }
    pub fn close_window(id: u64) -> CoreResult<()> {
        if unsafe { PostMessageW(hwnd(id)?, WM_CLOSE, 0, 0) } == 0 {
            return Err(os_error());
        }
        Ok(())
    }

    fn read_mode(device: &str, index: u32) -> Option<DEVMODEW> {
        let mut mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        (unsafe { EnumDisplaySettingsExW(wide(device).as_ptr(), index, &mut mode, 0) } != 0)
            .then_some(mode)
    }
    fn display_mode(device: &str, mode: DEVMODEW) -> DisplayMode {
        DisplayMode {
            device: device.into(),
            width: mode.dmPelsWidth,
            height: mode.dmPelsHeight,
            refresh: mode.dmDisplayFrequency,
            bits_per_pixel: mode.dmBitsPerPel,
        }
    }
    pub fn displays() -> CoreResult<Vec<Display>> {
        let mut result = Vec::new();
        for index in 0..128 {
            let mut device = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            if unsafe { EnumDisplayDevicesW(null(), index, &mut device, 0) } == 0 {
                break;
            }
            if device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0
                || device.StateFlags & DISPLAY_DEVICE_MIRRORING_DRIVER != 0
            {
                continue;
            }
            let id = text(&device.DeviceName);
            let Some(current) = read_mode(&id, ENUM_CURRENT_SETTINGS) else {
                continue;
            };
            let mut modes = std::collections::BTreeSet::new();
            for index in 0..10000 {
                let Some(mode) = read_mode(&id, index) else {
                    break;
                };
                let mode = display_mode(&id, mode);
                if crate::tools::resolution::validate_mode(&mode).is_ok() {
                    modes.insert(mode);
                }
            }
            let current = display_mode(&id, current);
            modes.insert(current.clone());
            result.push(Display {
                device: id,
                name: text(&device.DeviceString),
                current,
                modes: modes.into_iter().collect(),
            });
        }
        Ok(result)
    }
    pub fn set_resolution(request: &DisplayMode) -> CoreResult<()> {
        let mut mode = read_mode(&request.device, ENUM_CURRENT_SETTINGS)
            .ok_or_else(|| CoreError::InvalidInput("the display is no longer connected".into()))?;
        mode.dmPelsWidth = request.width;
        mode.dmPelsHeight = request.height;
        mode.dmBitsPerPel = request.bits_per_pixel;
        mode.dmDisplayFrequency = request.refresh;
        mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY;
        let device = wide(&request.device);
        let test = unsafe {
            ChangeDisplaySettingsExW(device.as_ptr(), &mode, null_mut(), CDS_TEST, null())
        };
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(CoreError::InvalidInput(format!(
                "the display driver refused this mode ({test})"
            )));
        }
        let result =
            unsafe { ChangeDisplaySettingsExW(device.as_ptr(), &mode, null_mut(), 0, null()) };
        if result != DISP_CHANGE_SUCCESSFUL {
            return Err(CoreError::InvalidInput(format!(
                "display mode change failed ({result})"
            )));
        }
        Ok(())
    }

    pub fn volumes() -> CoreResult<Vec<Volume>> {
        let mask = unsafe { GetLogicalDrives() };
        if mask == 0 {
            return Err(os_error());
        }
        let mut result = Vec::new();
        for index in 0..26 {
            if mask & (1 << index) == 0 {
                continue;
            }
            let root = format!("{}:\\", (b'A' + index as u8) as char);
            let kind = unsafe { GetDriveTypeW(wide(&root).as_ptr()) };
            if kind != 2 && kind != 5 {
                continue;
            }
            let mut label = [0_u16; 261];
            unsafe {
                GetVolumeInformationW(
                    wide(&root).as_ptr(),
                    label.as_mut_ptr(),
                    label.len() as u32,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    0,
                )
            };
            result.push(Volume {
                root,
                label: text(&label),
            });
        }
        Ok(result)
    }
    struct VolumeHandle(HANDLE);
    impl Drop for VolumeHandle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }
    pub fn eject(root: &str) -> CoreResult<()> {
        let letter = crate::tools::eject_volumes::validate_drive(
            root,
            &std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()),
        )?;
        let handle = unsafe {
            CreateFileW(
                wide(&format!("\\\\.\\{letter}:")).as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(os_error());
        }
        let handle = VolumeHandle(handle);
        let control = |code| {
            let mut returned = 0;
            (unsafe {
                DeviceIoControl(
                    handle.0,
                    code,
                    null(),
                    0,
                    null_mut(),
                    0,
                    &mut returned,
                    null_mut(),
                )
            }) != 0
        };
        if !control(FSCTL_LOCK_VOLUME) {
            return Err(CoreError::InvalidInput(
                "the volume is busy or access was denied; close open files and try again".into(),
            ));
        }
        if !control(FSCTL_DISMOUNT_VOLUME) || !control(IOCTL_STORAGE_EJECT_MEDIA) {
            let error = os_error();
            control(FSCTL_UNLOCK_VOLUME);
            return Err(error);
        }
        Ok(())
    }

    fn powercfg(args: &[&str]) -> CoreResult<String> {
        let executable =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32/powercfg.exe");
        let output = std::process::Command::new(executable).args(args).output()?;
        if !output.status.success() {
            return Err(CoreError::InvalidInput(
                "Windows refused the power-profile request".into(),
            ));
        }
        if output.stdout.iter().take(32).any(|b| *b == 0) {
            let utf16: Vec<u16> = output
                .stdout
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&utf16))
        } else {
            Ok(String::from_utf8_lossy(&output.stdout).into())
        }
    }
    pub fn power_profiles() -> CoreResult<Vec<PowerProfile>> {
        Ok(crate::tools::energy_saver::parse_profiles(&powercfg(&[
            "/list",
        ])?))
    }
    pub fn set_power_profile(id: &str) -> CoreResult<()> {
        crate::tools::energy_saver::validate_profile(id)?;
        powercfg(&["/setactive", id])?;
        Ok(())
    }
    pub fn hidden_files(show: Option<bool>) -> CoreResult<bool> {
        let mut state = SHELLSTATEA::default();
        unsafe { SHGetSetSettings(&mut state, SSF_SHOWALLOBJECTS, 0) };
        if let Some(show) = show {
            state._bitfield1 =
                crate::tools::hidden_files::update_shell_flags(state._bitfield1, show);
            unsafe {
                SHGetSetSettings(&mut state, SSF_SHOWALLOBJECTS, 1);
                SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_IDLIST, null(), null());
            }
        }
        let mut current = SHELLSTATEA::default();
        unsafe { SHGetSetSettings(&mut current, SSF_SHOWALLOBJECTS, 0) };
        Ok(current._bitfield1 & 1 != 0)
    }

    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe { RegCloseKey(self.0) };
        }
    }
    fn open_key(root: HKEY, name: &str, view: u32) -> Option<Key> {
        let mut key = null_mut();
        (unsafe { RegOpenKeyExW(root, wide(name).as_ptr(), 0, KEY_READ | view, &mut key) }
            == ERROR_SUCCESS)
            .then_some(Key(key))
    }
    fn reg_value(key: HKEY, name: &str) -> Option<(u32, Vec<u8>)> {
        let mut kind = 0;
        let mut size = 0;
        let name = wide(name);
        if unsafe { RegQueryValueExW(key, name.as_ptr(), null(), &mut kind, null_mut(), &mut size) }
            != ERROR_SUCCESS
            || size > 131072
        {
            return None;
        }
        let mut bytes = vec![0_u8; size as usize];
        if unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                null(),
                &mut kind,
                bytes.as_mut_ptr(),
                &mut size,
            )
        } != ERROR_SUCCESS
        {
            return None;
        }
        bytes.truncate(size as usize);
        Some((kind, bytes))
    }
    fn reg_string(key: HKEY, name: &str) -> String {
        let Some((kind, bytes)) = reg_value(key, name) else {
            return String::new();
        };
        if kind != REG_SZ && kind != REG_EXPAND_SZ {
            return String::new();
        }
        let utf16: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        let value = text(&utf16);
        if kind == REG_EXPAND_SZ {
            expand_environment(&value)
        } else {
            value
        }
    }
    fn reg_dword(key: HKEY, name: &str) -> u32 {
        reg_value(key, name)
            .filter(|(kind, b)| *kind == REG_DWORD && b.len() == 4)
            .map_or(0, |(_, b)| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn expand_environment(value: &str) -> String {
        let mut result = value.to_owned();
        let mut expansions = 0;
        for (key, value) in std::env::vars() {
            let needle = format!("%{key}%");
            while let Some(index) = result
                .to_ascii_lowercase()
                .find(&needle.to_ascii_lowercase())
            {
                expansions += 1;
                if expansions > 128 {
                    return String::new();
                }
                result.replace_range(index..index + needle.len(), &value);
                if result.len() > 32768 {
                    return String::new();
                }
            }
        }
        result
    }
    pub fn installed_apps() -> CoreResult<Vec<InstalledApp>> {
        let mut result = Vec::new();
        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
        for (root, label, view) in [
            (HKEY_CURRENT_USER, "user64", KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, "machine64", KEY_WOW64_64KEY),
            (HKEY_CURRENT_USER, "user32", KEY_WOW64_32KEY),
            (HKEY_LOCAL_MACHINE, "machine32", KEY_WOW64_32KEY),
        ] {
            let Some(key) = open_key(root, path, view) else {
                continue;
            };
            for index in 0..20000 {
                let mut name = [0_u16; 256];
                let mut length = name.len() as u32;
                let status = unsafe {
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
                };
                if status == ERROR_NO_MORE_ITEMS {
                    break;
                }
                if status != ERROR_SUCCESS {
                    continue;
                }
                let name = String::from_utf16_lossy(&name[..length as usize]);
                let Some(app) = open_key(key.0, &name, view) else {
                    continue;
                };
                if reg_dword(app.0, "SystemComponent") != 0 || reg_dword(app.0, "NoRemove") != 0 {
                    continue;
                }
                let display_name = reg_string(app.0, "DisplayName");
                let command = reg_string(app.0, "UninstallString");
                if display_name.trim().is_empty() || command.trim().is_empty() {
                    continue;
                }
                result.push(InstalledApp {
                    id: format!("{label}:{name}"),
                    name: display_name,
                    publisher: reg_string(app.0, "Publisher"),
                    version: reg_string(app.0, "DisplayVersion"),
                    command,
                    install_location: reg_string(app.0, "InstallLocation"),
                });
            }
        }
        result.sort_by_key(|a| a.name.to_lowercase());
        Ok(result)
    }
    pub fn run_uninstaller(command: &str) -> CoreResult<u32> {
        let mut args = crate::tools::uninstall::parse_command_line(command)?;
        let executable = args.remove(0);
        let program = if executable.eq_ignore_ascii_case("msiexec.exe")
            || executable.eq_ignore_ascii_case("msiexec")
        {
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
                .join("System32/msiexec.exe")
        } else {
            PathBuf::from(executable)
        };
        if !program.is_absolute()
            || !program.is_file()
            || !program
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        {
            return Err(CoreError::InvalidInput(
                "this app's uninstaller is not an absolute executable; use Windows Installed apps"
                    .into(),
            ));
        }
        let leaf = program
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if [
            "cmd.exe",
            "powershell.exe",
            "pwsh.exe",
            "wscript.exe",
            "cscript.exe",
            "mshta.exe",
        ]
        .contains(&leaf.as_str())
        {
            return Err(CoreError::InvalidInput(
                "script-based uninstallers must be started from Windows Installed apps".into(),
            ));
        }
        let child = std::process::Command::new(program).args(args).spawn()?;
        Ok(child.id())
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
macro_rules! unavailable {($($name:ident($($arg:ident:$ty:ty),*)->$ret:ty;)*)=>{$(pub fn $name($($arg:$ty),*)->CoreResult<$ret>{$(let _=$arg;)*Err(CoreError::Unsupported(crate::platform::Os::current().display_name()))})*};}
#[cfg(not(windows))]
unavailable! {windows()->Vec<WindowInfo>;window(id:u64)->WindowInfo;set_topmost(id:u64,enabled:bool)->();work_area(id:u64)->Rect;move_window(id:u64,bounds:Rect)->();minimize_window(id:u64)->();restore_window(id:u64)->();close_window(id:u64)->();displays()->Vec<Display>;set_resolution(mode:&DisplayMode)->();volumes()->Vec<Volume>;eject(root:&str)->();power_profiles()->Vec<PowerProfile>;set_power_profile(id:&str)->();installed_apps()->Vec<InstalledApp>;run_uninstaller(command:&str)->u32;}
#[cfg(not(windows))]
pub fn hidden_files(show: Option<bool>) -> CoreResult<bool> {
    #[cfg(target_os = "macos")]
    {
        if let Some(show) = show {
            crate::platform::run_command(&crate::platform::CommandSpec {
                program: "defaults",
                args: vec![
                    "write",
                    "com.apple.finder",
                    "AppleShowAllFiles",
                    "-bool",
                    if show { "true" } else { "false" },
                ],
            })?;
            crate::platform::run_command(&crate::platform::CommandSpec {
                program: "killall",
                args: vec!["Finder"],
            })?;
            return Ok(show);
        }
        let output = std::process::Command::new("defaults")
            .args(["read", "com.apple.finder", "AppleShowAllFiles"])
            .output()?;
        Ok(output.status.success()
            && ["1", "true", "yes"].contains(
                &String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_ascii_lowercase()
                    .as_str(),
            ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = show;
        Err(CoreError::Unsupported(
            crate::platform::Os::current().display_name(),
        ))
    }
}

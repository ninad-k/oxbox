//! Small audited Win32 effects. The scheduler and command plans stay in tool modules.
use crate::tools::sleep_timer::PowerAction;
use crate::{CoreError, CoreResult};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    UI::WindowsAndMessaging::*,
};

pub fn lock_session() -> CoreResult<()> {
    // SAFETY: LockWorkStation has no pointer arguments and applies to this session.
    if unsafe { windows_sys::Win32::System::Shutdown::LockWorkStation() } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

pub fn execute_power(action: PowerAction) -> CoreResult<()> {
    if action == PowerAction::Sleep {
        // SAFETY: no pointers, no forced sleep, normal wake events remain enabled.
        if !unsafe { windows_sys::Win32::System::Power::SetSuspendState(false, false, false) } {
            return Err(std::io::Error::last_os_error().into());
        }
        return Ok(());
    }
    crate::platform::run_command(&crate::tools::sleep_timer::command(
        crate::platform::Os::Windows,
        action,
    )?)
}

pub struct DesktopRestore {
    identity: crate::tools::hide_desktop::DesktopWindowIdentity,
    visible: bool,
}

unsafe fn list_view(parent: HWND) -> HWND {
    let def_class: Vec<u16> = "SHELLDLL_DefView\0".encode_utf16().collect();
    let list_class: Vec<u16> = "SysListView32\0".encode_utf16().collect();
    // SAFETY: UTF-16 strings are NUL terminated and pointers live for each call.
    let def = unsafe {
        FindWindowExW(
            parent,
            std::ptr::null_mut(),
            def_class.as_ptr(),
            std::ptr::null(),
        )
    };
    if def.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        FindWindowExW(
            def,
            std::ptr::null_mut(),
            list_class.as_ptr(),
            std::ptr::null(),
        )
    }
}

unsafe extern "system" fn find_desktop(hwnd: HWND, data: LPARAM) -> i32 {
    // SAFETY: data points to the local HWND for the duration of synchronous EnumWindows.
    let result = unsafe { list_view(hwnd) };
    if result.is_null() {
        return 1;
    }
    unsafe {
        *(data as *mut HWND) = result;
    }
    0
}

fn desktop_handle() -> HWND {
    let mut handle: HWND = std::ptr::null_mut();
    // SAFETY: callback is synchronous and the pointer remains valid until it returns.
    unsafe {
        EnumWindows(Some(find_desktop), &mut handle as *mut HWND as LPARAM);
    }
    handle
}

fn desktop_identity(
    handle: HWND,
    desktop: HWND,
) -> Option<crate::tools::hide_desktop::DesktopWindowIdentity> {
    let mut process = 0;
    // SAFETY: process points to live local storage. Invalid handles return zero.
    let thread = unsafe { GetWindowThreadProcessId(handle, &mut process) };
    if thread == 0 || process == 0 {
        return None;
    }
    let mut class_name = [0_u16; 128];
    // SAFETY: the buffer is writable and its length is passed to Win32.
    let length = unsafe { GetClassNameW(handle, class_name.as_mut_ptr(), class_name.len() as i32) };
    if length <= 0 {
        return None;
    }
    Some(crate::tools::hide_desktop::DesktopWindowIdentity {
        handle: handle as isize,
        process,
        thread,
        class_name: String::from_utf16_lossy(&class_name[..length as usize]),
        is_desktop: !desktop.is_null() && desktop == handle,
    })
}

pub fn hide_desktop() -> CoreResult<DesktopRestore> {
    let handle = desktop_handle();
    if handle.is_null() {
        return Err(CoreError::InvalidInput(
            "Windows Explorer's desktop icon view was not found.".into(),
        ));
    }
    let identity = desktop_identity(handle, handle)
        .filter(|identity| identity.class_name == "SysListView32")
        .ok_or_else(|| {
            CoreError::InvalidInput("The desktop icon view changed; try Hide Desktop again.".into())
        })?;
    let visible = unsafe { IsWindowVisible(handle) } != 0;
    unsafe {
        ShowWindow(handle, SW_HIDE);
    }
    Ok(DesktopRestore { identity, visible })
}

pub fn restore_desktop(state: &DesktopRestore) -> CoreResult<()> {
    let handle = state.identity.handle as HWND;
    let current = desktop_identity(handle, desktop_handle());
    crate::tools::hide_desktop::restore_owned_window(&state.identity, current.as_ref(), || {
        // SAFETY: identity, PID/thread, class and current desktop membership were
        // checked immediately above. Win32 offers no atomic identity+ShowWindow
        // operation, so an external destroy/reuse race remains a platform limit.
        unsafe {
            ShowWindow(handle, if state.visible { SW_SHOWNA } else { SW_HIDE });
        }
        Ok(())
    })
}

/// Best effort primary-monitor fullscreen and idle detection; no keyboard content is inspected.
pub fn activity_paused(pause_fullscreen: bool) -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut input = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut input) } != 0 {
        let ticks =
            unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() } as u32;
        if ticks.wrapping_sub(input.dwTime) >= 60_000 {
            return true;
        }
    }
    if !pause_fullscreen {
        return false;
    }
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return false;
    }
    let mut rect = RECT::default();
    unsafe {
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }
        // Ignore Explorer's own desktop, which occupies the full screen by design.
        if hwnd == GetDesktopWindow() || !list_view(hwnd).is_null() {
            return false;
        }
        rect.left <= 0
            && rect.top <= 0
            && rect.right >= GetSystemMetrics(SM_CXSCREEN)
            && rect.bottom >= GetSystemMetrics(SM_CYSCREEN)
    }
}

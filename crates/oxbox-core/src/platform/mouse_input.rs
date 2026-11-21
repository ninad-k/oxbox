//! Windows adapter for the explicitly started Mouse Activity session.
//! Hook callbacks retain timing only and always continue the hook chain.
use super::{Backend, Bounds, InputState, Point, step};
use crate::{CoreError, CoreResult};
use std::{
    cell::Cell,
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, Ordering},
};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, POINT, WPARAM},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromPoint},
    System::{
        LibraryLoader::GetModuleHandleW,
        StationsAndDesktops::{
            CloseDesktop, DESKTOP_READOBJECTS, GetUserObjectInformationW, OpenInputDesktop,
            UOI_NAME,
        },
        SystemInformation::GetTickCount64,
    },
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetLastInputInfo, INPUT, INPUT_0, INPUT_MOUSE, LASTINPUTINFO,
            MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
        },
        WindowsAndMessaging::*,
    },
};

const TAG: usize = 0x4f58424d;
thread_local! {static LAST_REAL:Cell<u64>=const{Cell::new(0)};}
fn activity() {
    LAST_REAL.with(|last| last.set(unsafe { GetTickCount64() }));
}
unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 && lparam != 0 {
        let event = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        if event.flags & LLMHF_INJECTED == 0 {
            activity();
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}
unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 && lparam != 0 {
        let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        if event.flags & LLKHF_INJECTED == 0 {
            activity();
        }
    }
    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}
fn failure(message: &str) -> CoreError {
    CoreError::InvalidInput(format!("Mouse Activity: {message}"))
}
fn input_tick() -> CoreResult<u32> {
    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return Err(failure(
            "input timing is unavailable; the session was stopped.",
        ));
    }
    Ok(info.dwTime)
}
fn point() -> CoreResult<Point> {
    let mut point = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return Err(failure(
            "cursor access is unavailable on this desktop; the session was stopped.",
        ));
    }
    Ok(Point {
        x: point.x,
        y: point.y,
    })
}
fn movement(initial: Point, bounds: Bounds, right: bool, tick: u32) -> CoreResult<INPUT> {
    let target = step(initial, bounds, right)?;
    // Pixel centers avoid acceleration and preserve negative-monitor coordinates.
    let normalize = |coordinate: i32, origin: i32, extent: i32| -> i32 {
        (((i64::from(coordinate) - i64::from(origin)) * 2 + 1) * 65536 / (2 * i64::from(extent)))
            .clamp(0, 65535) as i32
    };
    Ok(INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: normalize(target.x, bounds.left, bounds.width),
                dy: normalize(target.y, bounds.top, bounds.height),
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: tick,
                dwExtraInfo: TAG,
            },
        },
    })
}
fn movement_on_monitor(
    initial: Point,
    desktop: Bounds,
    monitor: Bounds,
    right: bool,
    tick: u32,
) -> CoreResult<INPUT> {
    let target = step(initial, monitor, right)?;
    movement(initial, desktop, target.x > initial.x, tick)
}
fn current_monitor(point: Point) -> CoreResult<Bounds> {
    let monitor = unsafe {
        MonitorFromPoint(
            POINT {
                x: point.x,
                y: point.y,
            },
            MONITOR_DEFAULTTONULL,
        )
    };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if monitor.is_null() || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return Err(failure(
            "the pointer is not on an available monitor; the session was stopped.",
        ));
    }
    Ok(Bounds {
        left: info.rcMonitor.left,
        top: info.rcMonitor.top,
        width: info.rcMonitor.right.saturating_sub(info.rcMonitor.left),
        height: info.rcMonitor.bottom.saturating_sub(info.rcMonitor.top),
    })
}
fn default_desktop() -> CoreResult<()> {
    let desktop = unsafe { OpenInputDesktop(0, 0, DESKTOP_READOBJECTS) };
    if desktop.is_null() {
        return Err(failure(
            "the input desktop is protected or unavailable; the session was stopped.",
        ));
    }
    let mut name = [0_u16; 64];
    let mut needed = 0;
    let okay = unsafe {
        GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            name.as_mut_ptr().cast(),
            std::mem::size_of_val(&name) as u32,
            &mut needed,
        )
    };
    unsafe { CloseDesktop(desktop) };
    if okay == 0
        || name.split(|value| *value == 0).next() != Some(&[68, 101, 102, 97, 117, 108, 116][..])
    {
        return Err(failure(
            "movement is allowed only on the normal input desktop; the session was stopped.",
        ));
    }
    Ok(())
}

pub(super) struct NativeInput {
    mouse: HHOOK,
    keyboard: HHOOK,
    system_tick: u32,
    own_tick: Option<u32>,
}
impl NativeInput {
    pub(super) fn new() -> CoreResult<Self> {
        default_desktop()?;
        let system_tick = input_tick()?;
        // Both hooks and their message queue belong to this one dedicated worker.
        let mut message = MSG::default();
        unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_NOREMOVE) };
        let module = unsafe { GetModuleHandleW(null()) };
        if module.is_null() {
            return Err(failure("input observation could not be initialized."));
        }
        let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0) };
        if mouse.is_null() {
            return Err(failure(
                "mouse observation was denied; no movement was started.",
            ));
        }
        let keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0) };
        if keyboard.is_null() {
            unsafe { UnhookWindowsHookEx(mouse) };
            return Err(failure(
                "keyboard observation was denied; no movement was started.",
            ));
        }
        activity();
        Ok(Self {
            mouse,
            keyboard,
            system_tick,
            own_tick: None,
        })
    }
}
impl Backend for NativeInput {
    fn poll(&mut self) -> CoreResult<InputState> {
        let mut message = MSG::default();
        let mut count = 0;
        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if message.message == WM_QUIT {
                return Err(CoreError::Cancelled);
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            count += 1;
            if count >= 1024 {
                activity();
                break;
            }
        }
        // Windows may silently remove a timed-out low-level hook. Independently
        // observe session input timing, accepting only our known timestamp as
        // synthetic. Uncertain/external input conservatively restarts waiting.
        let tick = input_tick()?;
        if tick != self.system_tick && self.own_tick != Some(tick) {
            activity();
        }
        self.system_tick = tick;
        Ok(InputState {
            now: unsafe { GetTickCount64() },
            last_genuine: LAST_REAL.with(Cell::get),
        })
    }
    fn move_one(
        &mut self,
        last_genuine: u64,
        right: bool,
        stop: &AtomicBool,
        deadline: Option<u64>,
    ) -> CoreResult<bool> {
        let initial = point()?;
        let input = self.poll()?;
        if input.last_genuine != last_genuine || stop.load(Ordering::Acquire) {
            return Ok(false);
        }
        // An idle held button/key must never turn a simulated move into a drag
        // or interfere with a shortcut. No virtual-key values are retained.
        if (1..=254).any(|key| unsafe { GetAsyncKeyState(key) } < 0) {
            activity();
            return Ok(false);
        }
        default_desktop()?;
        let bounds = Bounds {
            left: unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
            top: unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
            width: unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
            height: unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
        };
        if point()? != initial {
            activity();
            return Ok(false);
        }
        let tick = unsafe { GetTickCount64() } as u32;
        let event = movement_on_monitor(initial, bounds, current_monitor(initial)?, right, tick)?;
        if stop.load(Ordering::Acquire)
            || LAST_REAL.with(Cell::get) != last_genuine
            || deadline.is_some_and(|deadline| unsafe { GetTickCount64() } >= deadline)
        {
            return Ok(false);
        }
        self.own_tick = Some(tick);
        if unsafe { SendInput(1, &event, std::mem::size_of::<INPUT>() as i32) } != 1 {
            return Err(failure(
                "Windows refused the pointer movement; the session was stopped. Protected or elevated apps may block input.",
            ));
        }
        Ok(true)
    }
}
impl Drop for NativeInput {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.keyboard);
            UnhookWindowsHookEx(self.mouse);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_contains_only_bounded_absolute_motion_and_own_marker() {
        let bounds = Bounds {
            left: -1920,
            top: -1080,
            width: 3840,
            height: 2160,
        };
        for initial in [Point { x: -1920, y: -1080 }, Point { x: 1919, y: 1079 }] {
            let event = movement(initial, bounds, true, 123).unwrap();
            assert_eq!(event.r#type, INPUT_MOUSE);
            let mouse = unsafe { event.Anonymous.mi };
            assert_eq!(
                mouse.dwFlags,
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK
            );
            assert!((0..=65535).contains(&mouse.dx));
            assert!((0..=65535).contains(&mouse.dy));
            assert_eq!(mouse.mouseData, 0);
            assert_eq!(mouse.dwExtraInfo, TAG);
            assert_eq!(mouse.time, 123);
        }
        assert!(movement(Point { x: 0, y: 0 }, Bounds { width: 0, ..bounds }, true, 0).is_err());
    }
    #[test]
    fn monitor_edge_bounces_inside_monitor_instead_of_entering_desktop_gap() {
        let desktop = Bounds {
            left: 0,
            top: 0,
            width: 3920,
            height: 1080,
        };
        let monitor = Bounds {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        let event =
            movement_on_monitor(Point { x: 1919, y: 10 }, desktop, monitor, true, 10).unwrap();
        let actual = unsafe { event.Anonymous.mi };
        let expected = movement(Point { x: 1919, y: 10 }, desktop, false, 10).unwrap();
        assert_eq!(actual.dx, unsafe { expected.Anonymous.mi }.dx);
    }
}

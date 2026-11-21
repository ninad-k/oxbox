//! Windows shade rendering. Window handles never leave the creating worker.
use oxbox_core::{CoreError, CoreResult};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
#[derive(Clone, Default)]
pub struct Status {
    pub active: bool,
    pub error: Option<String>,
}
pub struct Focus {
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<Status>>,
    worker: Option<JoinHandle<()>>,
}
impl Focus {
    pub fn start(id: u64, pid: u32, opacity: f64) -> CoreResult<Self> {
        if !opacity.is_finite() || !(0.1..=0.85).contains(&opacity) {
            return Err(super::runtime::error("Choose dimming from 10% to 85%."));
        }
        #[cfg(not(windows))]
        {
            let _ = (id, pid);
            Err(CoreError::Unsupported(
                oxbox_core::platform::Os::current().display_name(),
            ))
        }
        #[cfg(windows)]
        {
            oxbox_core::tools::window_top::window(id, pid)?;
            let stop = Arc::new(AtomicBool::new(false));
            let status = Arc::new(Mutex::new(Status::default()));
            let stopping = stop.clone();
            let output = status.clone();
            let (ready, reply) = std::sync::mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("oxbox-native-focus".into())
                .spawn(move || {
                    let result = native::run(id, pid, opacity, &stopping, &output, &ready);
                    if let Err(error) = result {
                        let _ = ready.send(Err(error.to_string()));
                        if let Ok(mut s) = output.lock() {
                            s.error = Some(error.to_string());
                            s.active = false;
                        }
                    }
                })?;
            let mut focus = Self {
                stop,
                status,
                worker: Some(worker),
            };
            match reply
                .recv()
                .map_err(|_| super::runtime::error("Focus renderer did not initialize."))?
            {
                Ok(()) => Ok(focus),
                Err(e) => {
                    focus.stop();
                    Err(super::runtime::error(e))
                }
            }
        }
    }
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| Status {
                active: false,
                error: Some("Focus status is unavailable.".into()),
            })
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
        if let Ok(mut s) = self.status.lock() {
            s.active = false;
        }
    }
}
impl Drop for Focus {
    fn drop(&mut self) {
        self.stop();
    }
}
#[cfg(windows)]
mod native {
    use super::*;
    use oxbox_core::tools::{
        focus_window,
        screenshot::{self, CaptureKind},
        window_manager::Rect,
        window_top,
    };
    use std::{
        ptr::{null, null_mut},
        time::Duration,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    struct Shade {
        handle: windows_sys::Win32::Foundation::HWND,
        thread: u32,
        process: u32,
    }
    impl Shade {
        fn owned(&self) -> bool {
            let mut process = 0;
            let thread = unsafe { GetWindowThreadProcessId(self.handle, &mut process) };
            thread != 0 && thread == self.thread && process == self.process
        }
    }
    struct Shades(Vec<Shade>);
    impl Drop for Shades {
        fn drop(&mut self) {
            for shade in self.0.drain(..) {
                if shade.owned() {
                    unsafe {
                        DestroyWindow(shade.handle);
                    }
                }
            }
        }
    }
    pub fn run(
        id: u64,
        pid: u32,
        opacity: f64,
        stop: &AtomicBool,
        status: &Mutex<Status>,
        ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
    ) -> CoreResult<()> {
        let monitors: Vec<_> = screenshot::targets()?
            .into_iter()
            .filter(|t| t.kind == CaptureKind::Monitor)
            .collect();
        if monitors.is_empty() {
            return Err(super::super::runtime::error("No monitors are available."));
        }
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let title: Vec<u16> = "Oxbox Focus Shade\0".encode_utf16().collect();
        let mut shades = Shades(vec![]);
        for _ in 0..monitors.len() * 4 {
            if stop.load(Ordering::Acquire) {
                return Err(CoreError::Cancelled);
            }
            // Predefined STATIC/SS_BLACKRECT paints black without a custom callback.
            // Layered, transparent and no-activate styles never intercept pointer/focus.
            let handle = unsafe {
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                    class.as_ptr(),
                    title.as_ptr(),
                    WS_POPUP | 4,
                    0,
                    0,
                    1,
                    1,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                )
            };
            if handle.is_null() {
                return Err(super::super::runtime::error(
                    "Windows refused a focus shade window.",
                ));
            }
            let mut process = 0;
            let thread = unsafe { GetWindowThreadProcessId(handle, &mut process) };
            if thread == 0 || process != std::process::id() {
                return Err(super::super::runtime::error(
                    "The focus shade window is no longer owned by Oxbox.",
                ));
            }
            shades.0.push(Shade {
                handle,
                thread,
                process,
            });
            if unsafe {
                SetLayeredWindowAttributes(handle, 0, (opacity * 255.0).round() as u8, LWA_ALPHA)
            } == 0
            {
                return Err(super::super::runtime::error(
                    "Windows refused focus shade transparency.",
                ));
            }
        }
        if let Ok(mut s) = status.lock() {
            s.active = true;
        }
        let _ = ready.send(Ok(()));
        while !stop.load(Ordering::Acquire) {
            let target = window_top::window(id, pid)?;
            if target.minimized {
                break;
            }
            for (index, monitor) in monitors.iter().enumerate() {
                let bounds = Rect {
                    x: monitor.x,
                    y: monitor.y,
                    width: monitor.width,
                    height: monitor.height,
                };
                let area = focus_window::cutout(bounds, target.bounds, 1.0)?;
                let width = f64::from(bounds.width);
                let height = f64::from(bounds.height);
                let rects = if let Some(cut) = area {
                    [
                        (0.0, 0.0, width, cut.y),
                        (0.0, cut.y + cut.height, width, height - cut.y - cut.height),
                        (0.0, cut.y, cut.x, cut.height),
                        (
                            cut.x + cut.width,
                            cut.y,
                            width - cut.x - cut.width,
                            cut.height,
                        ),
                    ]
                } else {
                    [
                        (0.0, 0.0, width, height),
                        (0.0, 0.0, 0.0, 0.0),
                        (0.0, 0.0, 0.0, 0.0),
                        (0.0, 0.0, 0.0, 0.0),
                    ]
                };
                for (slot, (x, y, w, h)) in rects.into_iter().enumerate() {
                    let shade = &shades.0[index * 4 + slot];
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    if !shade.owned() {
                        return Err(super::super::runtime::error(
                            "A focus shade window was closed externally. Start Focus Window again.",
                        ));
                    }
                    let handle = shade.handle;
                    if w <= 0.0 || h <= 0.0 {
                        unsafe {
                            ShowWindow(handle, SW_HIDE);
                        }
                    } else if unsafe {
                        SetWindowPos(
                            handle,
                            HWND_TOPMOST,
                            bounds.x.saturating_add(x.round() as i32),
                            bounds.y.saturating_add(y.round() as i32),
                            w.round() as i32,
                            h.round() as i32,
                            SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        )
                    } == 0
                    {
                        return Err(super::super::runtime::error(
                            "Windows refused to position a focus shade.",
                        ));
                    }
                }
            }
            let mut message = MSG::default();
            while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            std::thread::park_timeout(Duration::from_millis(150));
        }
        if let Ok(mut s) = status.lock() {
            s.active = false;
        }
        Ok(())
    }
}

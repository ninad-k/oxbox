//! App-scoped runtime controls. These do not claim to block browser-owned services.

mod policy;

use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use tauri::{AppHandle, WebviewWindow, WebviewWindowBuilder, Wry};

static CONFIGURED: AtomicBool = AtomicBool::new(false);
static BLOCKED: AtomicU64 = AtomicU64::new(0);
static LOCAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePrivacyStatus {
    supported: bool,
    configured: bool,
    blocked_web_resources: u64,
    local_web_resources: u64,
}

/// Counters only: never records URIs, headers, user content or identifiers.
#[tauri::command]
pub fn runtime_privacy_status() -> RuntimePrivacyStatus {
    RuntimePrivacyStatus {
        supported: cfg!(windows),
        configured: CONFIGURED.load(Ordering::Acquire),
        blocked_web_resources: BLOCKED.load(Ordering::Relaxed),
        local_web_resources: LOCAL_REQUESTS.load(Ordering::Relaxed),
    }
}

/// Called first from setup; the configured main window must have `create: false`.
pub fn initialize(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == crate::tray::MAIN_WINDOW)
        .ok_or_else(|| std::io::Error::other("main window configuration is missing"))?
        .clone();
    #[cfg(windows)]
    windows_runtime::initialize(app, config).map_err(std::io::Error::other)?;
    #[cfg(not(windows))]
    WebviewWindowBuilder::from_config(app, &config)?.build()?;
    Ok(())
}

type WindowBuilder<'a> = WebviewWindowBuilder<'a, Wry, AppHandle>;

/// Build on the UI thread so every Windows view shares the configured environment.
/// Windows starts blank, installs the filter, then navigates to the local route.
pub fn create_window<F>(
    app: &AppHandle,
    label: String,
    path: PathBuf,
    configure: F,
) -> Result<WebviewWindow, crate::error::ToolError>
where
    F: for<'a> FnOnce(WindowBuilder<'a>) -> WindowBuilder<'a> + Send + 'static,
{
    let handle = app.clone();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        #[cfg(windows)]
        let result = windows_runtime::create_window(&handle, label, path, configure);
        #[cfg(not(windows))]
        let result = configure(WebviewWindowBuilder::new(
            &handle,
            &label,
            tauri::WebviewUrl::App(path),
        ))
        .build()
        .map_err(|error| error.to_string());
        policy::deliver_or_discard(send, result, |window| {
            let _ = window.destroy();
        });
    })
    .map_err(|error| crate::error::ToolError::internal(error.to_string()))?;
    receive
        .recv_timeout(std::time::Duration::from_secs(30))
        .map_err(|_| {
            crate::error::ToolError::internal("webview initialization did not complete".into())
        })?
        .map_err(crate::error::ToolError::internal)
}

#[cfg(windows)]
mod windows_runtime {
    use super::*;
    use policy::{
        EnvironmentBackend, StartupBackend, activate, allowed_request, prepare_environment,
    };
    use std::{cell::RefCell, sync::mpsc, time::Duration};
    use tauri::Manager;
    use webview2_com::{
        CoreWebView2EnvironmentOptions, CreateCoreWebView2EnvironmentCompletedHandler,
        Microsoft::Web::WebView2::Win32::{
            COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
            COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2_22, ICoreWebView2Environment,
            ICoreWebView2Environment7, ICoreWebView2EnvironmentOptions,
        },
        WebResourceRequestedEventHandler, take_pwstr,
    };
    use windows::{
        Win32::{
            Foundation::E_POINTER,
            System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
        },
        core::{HSTRING, Interface, PCWSTR, PWSTR},
    };

    // COM interfaces stay on the UI thread, never in Send/Sync managed state.
    thread_local! {
        // Accessed before ENVIRONMENT so its destructor runs after that COM
        // interface is released. A config-created window no longer initializes
        // the apartment for us before this first SDK call.
        static APARTMENT: RefCell<Option<ApartmentGuard>> = const { RefCell::new(None) };
        static ENVIRONMENT: RefCell<Option<ICoreWebView2Environment>> = const { RefCell::new(None) };
    }

    struct ApartmentGuard {
        _thread: std::marker::PhantomData<std::rc::Rc<()>>,
    }
    impl Drop for ApartmentGuard {
        fn drop(&mut self) {
            // Exactly balances our successful S_OK or S_FALSE initialization,
            // on the same UI thread after the owned environment is released.
            unsafe {
                CoUninitialize();
            }
        }
    }
    struct EnvironmentStartup<'a> {
        profile: &'a std::path::Path,
    }
    impl EnvironmentBackend for EnvironmentStartup<'_> {
        type Environment = ICoreWebView2Environment;
        fn initialize_apartment(&mut self) -> Result<(), String> {
            APARTMENT.with(|slot| {
                if slot.borrow().is_some() {
                    return Ok(());
                }
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                    .ok()
                    .map_err(|error| error.to_string())?;
                slot.replace(Some(ApartmentGuard {
                    _thread: std::marker::PhantomData,
                }));
                Ok(())
            })
        }
        fn create_environment(&mut self) -> Result<Self::Environment, String> {
            create_environment(self.profile)
        }
    }

    pub(super) fn initialize(
        app: &mut tauri::App,
        mut config: tauri::utils::config::WindowConfig,
    ) -> Result<(), String> {
        let profile = app
            .path()
            .app_local_data_dir()
            .map_err(|error| error.to_string())?
            .join("webview-v1");
        std::fs::create_dir_all(&profile).map_err(|error| error.to_string())?;
        let profile = std::fs::canonicalize(profile).map_err(|error| error.to_string())?;
        let environment = prepare_environment(&mut EnvironmentStartup { profile: &profile })?;
        ENVIRONMENT.with(|value| value.replace(Some(environment.clone())));
        let target = local_url(
            app.handle(),
            match &config.url {
                tauri::WebviewUrl::App(path) => path.clone(),
                _ => return Err("the launcher must use an embedded app route".into()),
            },
        )?;
        let visible = config.visible && !std::env::args().any(|arg| arg == "--autostart");
        config.visible = false;
        config.url = tauri::WebviewUrl::External(blank()?);
        let window = WebviewWindowBuilder::from_config(app, &config)
            .map_err(|error| error.to_string())?
            .with_environment(environment)
            .build()
            .map_err(|error| error.to_string())?;
        start(&window, target)?;
        CONFIGURED.store(true, Ordering::Release);
        if visible {
            window.show().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn create_environment(profile: &std::path::Path) -> Result<ICoreWebView2Environment, String> {
        let options = CoreWebView2EnvironmentOptions::default();
        // Supported SDK options only. Keep tracking prevention and reputation
        // checking enabled; no browser flags or global settings are changed.
        unsafe {
            options.set_allow_single_sign_on_using_os_primary_account(false);
            options.set_exclusive_user_data_folder_access(true);
            options.set_is_custom_crash_reporting_enabled(true);
            options.set_are_browser_extensions_enabled(false);
            options.set_enable_tracking_prevention(true);
        }
        let (send, receive) = mpsc::channel();
        unsafe {
            CreateCoreWebView2EnvironmentWithOptions(
                PCWSTR::null(),
                &HSTRING::from(profile.as_os_str()),
                &ICoreWebView2EnvironmentOptions::from(options),
                &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
                    move |code, environment| {
                        let result = code
                            .and_then(|()| {
                                environment.ok_or_else(|| windows::core::Error::from(E_POINTER))
                            })
                            .map_err(|error| error.to_string());
                        let _ = send.send(result);
                        Ok(())
                    },
                )),
            )
            .map_err(|error| error.to_string())?;
        }
        let environment =
            webview2_com::wait_with_pump(receive).map_err(|error| error.to_string())??;
        // SDK environment/registry overrides must not silently defeat isolation.
        let actual = unsafe {
            let mut actual = PWSTR::null();
            environment
                .cast::<ICoreWebView2Environment7>()
                .map_err(|error| error.to_string())?
                .UserDataFolder(&mut actual)
                .map_err(|error| error.to_string())?;
            PathBuf::from(take_pwstr(actual))
        };
        let actual = std::fs::canonicalize(actual).map_err(|error| error.to_string())?;
        if actual != profile {
            return Err(
                "a WebView2 override replaced the isolated app profile; initialization stopped"
                    .into(),
            );
        }
        Ok(environment)
    }

    fn blank() -> Result<tauri::Url, String> {
        tauri::Url::parse("about:blank").map_err(|error| error.to_string())
    }

    fn local_url(app: &AppHandle, path: PathBuf) -> Result<tauri::Url, String> {
        let base = if tauri::is_dev() {
            app.config()
                .build
                .dev_url
                .clone()
                .ok_or("development URL is missing")?
        } else {
            tauri::Url::parse("http://tauri.localhost/").map_err(|error| error.to_string())?
        };
        let path = path
            .to_str()
            .ok_or("app route is not valid Unicode")?
            .replace('\\', "/");
        let target = base
            .join(policy::navigation_path(&path))
            .map_err(|error| error.to_string())?;
        if !allowed_request(target.as_str(), tauri::is_dev()) {
            return Err("app navigation is outside the local runtime policy".into());
        }
        Ok(target)
    }

    pub(super) fn create_window<F>(
        app: &AppHandle,
        label: String,
        path: PathBuf,
        configure: F,
    ) -> Result<WebviewWindow, String>
    where
        F: for<'a> FnOnce(WindowBuilder<'a>) -> WindowBuilder<'a>,
    {
        let environment = ENVIRONMENT
            .with(|value| value.borrow().clone())
            .ok_or("the isolated webview environment is not initialized on the UI thread")?;
        let target = local_url(app, path)?;
        let window = configure(WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::External(blank()?),
        ))
        .with_environment(environment)
        .build()
        .map_err(|error| error.to_string())?;
        start(&window, target)?;
        Ok(window)
    }

    fn start(window: &WebviewWindow, target: tauri::Url) -> Result<(), String> {
        let result = activate(&mut WindowStartup { window, target });
        if result.is_err() {
            let _ = window.destroy();
        }
        result
    }

    struct WindowStartup<'a> {
        window: &'a WebviewWindow,
        target: tauri::Url,
    }
    impl StartupBackend for WindowStartup<'_> {
        fn install_filter(&mut self) -> Result<(), String> {
            let (send, receive) = mpsc::sync_channel(1);
            self.window.with_webview(move |native| {
                // This callback executes on the STA/UI thread. Its result is
                // returned before initial app navigation can proceed.
                let result = (|| unsafe {
                    let webview = native.controller().CoreWebView2()?;
                    let extended = webview.cast::<ICoreWebView2_22>()?;
                    let environment = native.environment();
                    let handler = WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let mut uri = PWSTR::null();
                        args.Request()?.Uri(&mut uri)?;
                        let uri = take_pwstr(uri);
                        if allowed_request(&uri, tauri::is_dev()) {
                            LOCAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
                        } else {
                            BLOCKED.fetch_add(1, Ordering::Relaxed);
                            let response = environment.CreateWebResourceResponse(
                                None, 403, &HSTRING::from("Blocked by Oxbox"),
                                &HSTRING::from("Content-Type: text/plain\r\nCache-Control: no-store\r\n"),
                            )?;
                            args.SetResponse(&response)?;
                        }
                        Ok(())
                    }));
                    let mut token = 0;
                    webview.add_WebResourceRequested(&handler, &mut token)?;
                    extended.AddWebResourceRequestedFilterWithRequestSourceKinds(
                        &HSTRING::from("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                        COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                    )?;
                    Ok::<(), windows::core::Error>(())
                })().map_err(|error| format!("WebView2 privacy filter could not initialize: {error}"));
                let _ = send.send(result);
            }).map_err(|error| error.to_string())?;
            receive
                .recv_timeout(Duration::from_secs(5))
                .map_err(|_| "privacy filter initialization did not complete".to_string())?
        }
        fn navigate(&mut self) -> Result<(), String> {
            self.window
                .navigate(self.target.clone())
                .map_err(|error| error.to_string())
        }
    }
}

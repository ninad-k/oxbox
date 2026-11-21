//! App-scoped embedded-webview privacy. Browser-owned traffic is measured separately.

#[cfg(any(windows, test))]
pub(crate) fn allowed_request(uri: &str, development: bool) -> bool {
    if uri == "about:blank" || uri.starts_with("data:") {
        return true;
    }
    // A blob's origin may be local, but nested or opaque blob URLs are not
    // needed. Avoid recursive processing of attacker-controlled URI strings.
    let uri = uri.strip_prefix("blob:").unwrap_or(uri);
    let origins = [
        "http://tauri.localhost",
        "http://ipc.localhost",
        "http://asset.localhost",
    ];
    origins
        .into_iter()
        .chain(development.then_some("http://localhost:1420"))
        .chain(development.then_some("ws://localhost:1420"))
        .any(|origin| {
            uri.strip_prefix(origin)
                .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with(['/', '?', '#']))
        })
}

#[cfg(any(windows, test))]
pub(crate) trait StartupBackend {
    fn install_filter(&mut self) -> Result<(), String>;
    fn navigate(&mut self) -> Result<(), String>;
}

#[cfg(any(windows, test))]
pub(crate) fn activate(backend: &mut impl StartupBackend) -> Result<(), String> {
    backend.install_filter()?;
    backend.navigate()
}

#[cfg(any(windows, test))]
pub(crate) trait EnvironmentBackend {
    type Environment;
    fn initialize_apartment(&mut self) -> Result<(), String>;
    fn create_environment(&mut self) -> Result<Self::Environment, String>;
}

#[cfg(any(windows, test))]
pub(crate) fn prepare_environment<B: EnvironmentBackend>(
    backend: &mut B,
) -> Result<B::Environment, String> {
    backend.initialize_apartment()?;
    backend.create_environment()
}

#[cfg(any(windows, test))]
pub(crate) fn navigation_path(path: &str) -> &str {
    // Match Tauri's default-document resolution for SPA root navigation.
    if path == "index.html" { "" } else { path }
}

pub(crate) fn deliver_or_discard<T, E>(
    send: std::sync::mpsc::SyncSender<Result<T, E>>,
    result: Result<T, E>,
    discard: impl FnOnce(T),
) {
    if let Err(std::sync::mpsc::SendError(Ok(value))) = send.send(result) {
        discard(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_requests_are_limited_to_exact_local_app_origins() {
        for uri in [
            "http://tauri.localhost/",
            "http://ipc.localhost/plugin:test",
            "http://asset.localhost/a.png",
            "about:blank",
            "data:image/png;base64,AA==",
            "blob:http://tauri.localhost/00000000-0000-0000-0000-000000000000",
        ] {
            assert!(allowed_request(uri, false), "{uri}");
        }
        for uri in [
            "https://config.edge.skype.com/config/v1/Edge/157",
            "https://edge.microsoft.com/componentupdater/",
            "http://tauri.localhost.evil.invalid/",
            "http://tauri.localhost@evil.invalid/",
            "http://evil.invalid@tauri.localhost/",
            "http://tauri.localhost:8123/",
            "http://127.0.0.1/",
            "http://localhost:1420/",
            "file:///C:/private.txt",
            "blob:https://evil.invalid/id",
            "blob:blob:http://tauri.localhost/id",
            "javascript:fetch('https://evil.invalid')",
            "not a URI",
        ] {
            assert!(!allowed_request(uri, false), "{uri}");
        }
    }

    #[test]
    fn development_exception_is_only_the_configured_loopback_dev_port() {
        assert!(allowed_request("http://localhost:1420/@vite/client", true));
        assert!(allowed_request("ws://localhost:1420/", true));
        for uri in [
            "http://localhost:1421/",
            "http://localhost.evil.invalid:1420/",
            "http://127.0.0.1:1420/",
        ] {
            assert!(!allowed_request(uri, true), "{uri}");
        }
    }

    #[derive(Default)]
    struct Fake {
        calls: Vec<&'static str>,
        fail_filter: bool,
    }
    impl StartupBackend for Fake {
        fn install_filter(&mut self) -> Result<(), String> {
            self.calls.push("filter");
            if self.fail_filter {
                Err("SDK filter failed".into())
            } else {
                Ok(())
            }
        }
        fn navigate(&mut self) -> Result<(), String> {
            self.calls.push("navigate");
            Ok(())
        }
    }
    #[test]
    fn navigation_happens_only_after_the_sdk_filter_is_ready() {
        let mut backend = Fake::default();
        activate(&mut backend).unwrap();
        assert_eq!(backend.calls, ["filter", "navigate"]);
    }
    #[test]
    fn failed_filter_never_navigates_to_the_application() {
        let mut backend = Fake {
            fail_filter: true,
            ..Fake::default()
        };
        assert!(activate(&mut backend).is_err());
        assert_eq!(backend.calls, ["filter"]);
    }

    #[test]
    fn abandoned_construction_discards_the_late_window() {
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        drop(receive);
        let discarded = std::cell::Cell::new(false);
        deliver_or_discard(send, Ok::<_, ()>(42), |value| discarded.set(value == 42));
        assert!(discarded.get());
    }

    #[test]
    fn delivered_window_stays_owned_by_the_waiting_caller() {
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        deliver_or_discard(send, Ok::<_, ()>(42), |_| {
            panic!("live receiver lost its window")
        });
        assert_eq!(receive.recv().unwrap(), Ok(42));
    }

    #[derive(Default)]
    struct FakeEnvironment {
        calls: Vec<&'static str>,
        fail_apartment: bool,
    }
    impl EnvironmentBackend for FakeEnvironment {
        type Environment = ();
        fn initialize_apartment(&mut self) -> Result<(), String> {
            self.calls.push("apartment");
            if self.fail_apartment {
                Err("STA unavailable".into())
            } else {
                Ok(())
            }
        }
        fn create_environment(&mut self) -> Result<(), String> {
            let initialized = self.calls == ["apartment"] && !self.fail_apartment;
            self.calls.push("environment");
            if initialized {
                Ok(())
            } else {
                Err("CoInitialize has not been called".into())
            }
        }
    }
    #[test]
    fn environment_is_created_only_after_sta_initialization() {
        let mut backend = FakeEnvironment::default();
        prepare_environment(&mut backend).unwrap();
        assert_eq!(backend.calls, ["apartment", "environment"]);
    }
    #[test]
    fn failed_sta_initialization_never_starts_the_runtime() {
        let mut backend = FakeEnvironment {
            fail_apartment: true,
            ..FakeEnvironment::default()
        };
        assert!(prepare_environment(&mut backend).is_err());
        assert_eq!(backend.calls, ["apartment"]);
    }

    #[test]
    fn default_document_navigates_to_the_spa_base_like_tauri() {
        assert_eq!(navigation_path("index.html"), "");
        assert_eq!(navigation_path("overlay"), "overlay");
        assert_eq!(navigation_path("focus-overlay"), "focus-overlay");
    }
}

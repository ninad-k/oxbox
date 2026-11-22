//! Default microphone mute status and explicit changes.
use crate::{CoreError, CoreResult, jobs::CancelToken};
/// Compatibility wrapper for callers without a managed task.
pub fn status() -> CoreResult<bool> {
    status_with_cancel(&CancelToken::default())
}
/// Compatibility wrapper for callers without a managed task.
pub fn set_muted(muted: bool) -> CoreResult<bool> {
    set_muted_with_cancel(muted, &CancelToken::default())
}
pub fn parse_pulse_mute(output: &str) -> CoreResult<bool> {
    match output.trim() {
        "Mute: yes" => Ok(true),
        "Mute: no" => Ok(false),
        _ => Err(CoreError::InvalidInput(
            "the audio provider returned an unknown mute state".into(),
        )),
    }
}
#[cfg(target_os = "windows")]
pub fn status_with_cancel(cancel: &CancelToken) -> CoreResult<bool> {
    windows_mute(None, cancel)
}
#[cfg(target_os = "windows")]
pub fn set_muted_with_cancel(muted: bool, cancel: &CancelToken) -> CoreResult<bool> {
    windows_mute(Some(muted), cancel)
}
#[cfg(target_os = "windows")]
fn windows_mute(muted: Option<bool>, cancel: &CancelToken) -> CoreResult<bool> {
    cancel.check()?;
    use windows::Win32::{
        Media::Audio::{
            Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
            eCommunications,
        },
        System::Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        },
    };
    let error = |error: windows::core::Error| {
        CoreError::InvalidInput(format!("microphone control failed: {error}"))
    };
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }
    // SAFETY: balance successful COM initialization; accept an already initialized apartment.
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let _apartment = if initialized.is_ok() {
        Apartment(true)
    } else if initialized.0 == 0x80010106_u32 as i32 {
        Apartment(false)
    } else {
        return Err(error(initialized.into()));
    };
    // SAFETY: COM interfaces own reference counts and all calls remain on this initialized thread.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(error)?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eCapture, eCommunications)
            .map_err(error)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).map_err(error)?;
        cancel.check()?;
        if let Some(muted) = muted {
            volume.SetMute(muted, std::ptr::null()).map_err(error)?;
        }
        cancel.check()?;
        Ok(volume.GetMute().map_err(error)?.as_bool())
    }
}
#[cfg(target_os = "linux")]
pub fn status_with_cancel(cancel: &CancelToken) -> CoreResult<bool> {
    pulse_status_using(cancel, &mut pulse_run)
}
#[cfg(target_os = "linux")]
pub fn set_muted_with_cancel(muted: bool, cancel: &CancelToken) -> CoreResult<bool> {
    pulse_set_using(muted, cancel, &mut pulse_run)
}
#[cfg(target_os = "linux")]
fn pulse_run(arguments: &[&str], cancel: &CancelToken) -> CoreResult<String> {
    cancel.check()?;
    let executable = super::media_provider::find_executable("pactl")?;
    super::media_provider::run_text_c_locale(
        &executable,
        &arguments
            .iter()
            .map(|value| (*value).into())
            .collect::<Vec<_>>(),
        cancel,
        std::time::Duration::from_secs(5),
        4096,
    )
}
#[cfg(any(target_os = "linux", test))]
fn pulse_status_using(
    cancel: &CancelToken,
    runner: &mut impl FnMut(&[&str], &CancelToken) -> CoreResult<String>,
) -> CoreResult<bool> {
    cancel.check()?;
    let output = runner(&["get-source-mute", "@DEFAULT_SOURCE@"], cancel)?;
    cancel.check()?;
    parse_pulse_mute(&output)
}
#[cfg(any(target_os = "linux", test))]
fn pulse_set_using(
    muted: bool,
    cancel: &CancelToken,
    runner: &mut impl FnMut(&[&str], &CancelToken) -> CoreResult<String>,
) -> CoreResult<bool> {
    cancel.check()?;
    runner(
        &[
            "set-source-mute",
            "@DEFAULT_SOURCE@",
            if muted { "1" } else { "0" },
        ],
        cancel,
    )?;
    cancel.check()?;
    pulse_status_using(cancel, runner)
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn status_with_cancel(cancel: &CancelToken) -> CoreResult<bool> {
    cancel.check()?;
    Err(CoreError::Unsupported(
        "this OS microphone endpoint control",
    ))
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn set_muted_with_cancel(_muted: bool, cancel: &CancelToken) -> CoreResult<bool> {
    status_with_cancel(cancel)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linux_mute_parse_refuses_unknown_output() {
        assert!(parse_pulse_mute("Mute: yes\n").unwrap());
        assert!(!parse_pulse_mute("Mute: no\n").unwrap());
        assert!(parse_pulse_mute("unexpected").is_err());
    }

    #[test]
    fn verification56_pulse_status_rejects_cancelled_and_late_results_without_audio_access() {
        let cancel = crate::jobs::CancelToken::default();
        cancel.cancel();
        let mut calls = 0;
        let result = pulse_status_using(&cancel, &mut |_, _| {
            calls += 1;
            Ok("Mute: yes".into())
        });
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert_eq!(calls, 0);

        let cancel = crate::jobs::CancelToken::default();
        let result = pulse_status_using(&cancel, &mut |arguments, received| {
            assert_eq!(arguments, ["get-source-mute", "@DEFAULT_SOURCE@"]);
            received.cancel();
            Ok("Mute: yes".into())
        });
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert!(
            cancel.check().is_err(),
            "the caller's token must reach the runner"
        );
    }

    #[test]
    fn verification56_pulse_change_cancel_prevents_follow_up_inspection_without_audio_access() {
        let cancel = crate::jobs::CancelToken::default();
        let mut calls = Vec::new();
        let result = pulse_set_using(true, &cancel, &mut |arguments, received| {
            calls.push(
                arguments
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect::<Vec<_>>(),
            );
            received.cancel();
            Ok(String::new())
        });
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert_eq!(
            calls,
            vec![vec!["set-source-mute", "@DEFAULT_SOURCE@", "1"]]
        );
    }
}

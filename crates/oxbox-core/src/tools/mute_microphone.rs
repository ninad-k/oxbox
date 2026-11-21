//! Default microphone mute status and explicit changes.
use crate::{CoreError, CoreResult};
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
pub fn status() -> CoreResult<bool> {
    windows_mute(None)
}
#[cfg(target_os = "windows")]
pub fn set_muted(muted: bool) -> CoreResult<bool> {
    windows_mute(Some(muted))
}
#[cfg(target_os = "windows")]
fn windows_mute(muted: Option<bool>) -> CoreResult<bool> {
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
        if let Some(muted) = muted {
            volume.SetMute(muted, std::ptr::null()).map_err(error)?;
        }
        Ok(volume.GetMute().map_err(error)?.as_bool())
    }
}
#[cfg(target_os = "linux")]
pub fn status() -> CoreResult<bool> {
    let output = std::process::Command::new("pactl")
        .args(["get-source-mute", "@DEFAULT_SOURCE@"])
        .env("LC_ALL", "C")
        .output()?;
    if !output.status.success() {
        return Err(CoreError::InvalidInput(
            "PulseAudio/PipeWire microphone control is unavailable".into(),
        ));
    }
    parse_pulse_mute(&String::from_utf8_lossy(&output.stdout))
}
#[cfg(target_os = "linux")]
pub fn set_muted(muted: bool) -> CoreResult<bool> {
    let result = std::process::Command::new("pactl")
        .args([
            "set-source-mute",
            "@DEFAULT_SOURCE@",
            if muted { "1" } else { "0" },
        ])
        .status()?;
    if !result.success() {
        return Err(CoreError::InvalidInput(
            "PulseAudio/PipeWire refused the microphone change".into(),
        ));
    }
    status()
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn status() -> CoreResult<bool> {
    Err(CoreError::Unsupported(
        "this OS microphone endpoint control",
    ))
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn set_muted(_muted: bool) -> CoreResult<bool> {
    status()
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
}

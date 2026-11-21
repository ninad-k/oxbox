//! Explicit microphone recording to WAV through a local FFmpeg provider.
use crate::{CoreError, CoreResult, platform::Os};
pub fn validate_duration(seconds: u64) -> CoreResult<()> {
    if !(1..=86_400).contains(&seconds) {
        return Err(CoreError::InvalidInput(
            "recording duration must be 1 second to 24 hours".into(),
        ));
    }
    Ok(())
}
pub fn validate_device(device: &str) -> CoreResult<()> {
    let normalized = device.to_lowercase();
    if device.trim().is_empty()
        || device.len() > 512
        || device.contains(['\0', '\n', '\r'])
        || normalized.contains(":audio=")
        || normalized.contains(":video=")
    {
        return Err(CoreError::InvalidInput(
            "enter a valid single local device name or index".into(),
        ));
    }
    Ok(())
}
pub fn plan(os: Os, device: &str, seconds: u64) -> CoreResult<Vec<String>> {
    validate_duration(seconds)?;
    validate_device(device)?;
    let mut args = match os {
        Os::Windows => vec![
            "-f".into(),
            "dshow".into(),
            "-i".into(),
            format!("audio={device}"),
        ],
        Os::Macos => {
            if !device.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(CoreError::InvalidInput(
                    "macOS microphone input must be a numeric AVFoundation index".into(),
                ));
            }
            vec![
                "-f".into(),
                "avfoundation".into(),
                "-i".into(),
                format!(":{device}"),
            ]
        }
        Os::Linux => vec!["-f".into(), "pulse".into(), "-i".into(), device.into()],
        Os::Other => return Err(CoreError::Unsupported(os.display_name())),
    };
    args.extend([
        "-t".into(),
        seconds.to_string(),
        "-vn".into(),
        "-c:a".into(),
        "pcm_s16le".into(),
        "-ar".into(),
        "48000".into(),
        "-ac".into(),
        "2".into(),
    ]);
    Ok(args)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_plans_are_explicit_and_bounded() {
        let args = plan(crate::platform::Os::Windows, "Mic;$(touch nope)", 60).unwrap();
        assert!(args.contains(&"audio=Mic;$(touch nope)".to_string()));
        assert!(args.contains(&"pcm_s16le".to_string()));
        assert!(plan(crate::platform::Os::Windows, "", 60).is_err());
        assert!(plan(crate::platform::Os::Linux, "default", 0).is_err());
    }
}

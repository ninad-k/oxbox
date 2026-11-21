//! Explicit camera video recording, with no automatic microphone input.
use crate::{CoreResult, platform::Os};
pub fn plan(os: Os, device: &str, seconds: u64) -> CoreResult<Vec<String>> {
    super::record_audio::validate_duration(seconds)?;
    let mut args = super::take_photo::camera_input(os, device)?;
    args.extend([
        "-t".into(),
        seconds.to_string(),
        "-an".into(),
        "-vf".into(),
        "pad=ceil(iw/2)*2:ceil(ih/2)*2".into(),
        "-c:v".into(),
        "mpeg4".into(),
        "-q:v".into(),
        "5".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
    ]);
    Ok(args)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn video_does_not_enable_audio_implicitly() {
        let args = plan(crate::platform::Os::Windows, "Camera", 60).unwrap();
        assert!(args.contains(&"-an".to_string()));
        assert!(plan(crate::platform::Os::Windows, "Camera", u64::MAX).is_err());
    }
}

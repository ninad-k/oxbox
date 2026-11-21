//! Explicit camera still capture through a local provider.
use crate::{CoreError, CoreResult, platform::Os};
pub fn camera_input(os: Os, device: &str) -> CoreResult<Vec<String>> {
    super::record_audio::validate_device(device)?;
    Ok(match os {
        Os::Windows => vec![
            "-f".into(),
            "dshow".into(),
            "-i".into(),
            format!("video={device}"),
        ],
        Os::Macos => {
            if !device.bytes().all(|value| value.is_ascii_digit()) {
                return Err(CoreError::InvalidInput(
                    "macOS camera must be a numeric AVFoundation device index".into(),
                ));
            }
            vec![
                "-f".into(),
                "avfoundation".into(),
                "-framerate".into(),
                "30".into(),
                "-i".into(),
                format!("{device}:none"),
            ]
        }
        Os::Linux => {
            if !device.starts_with("/dev/video")
                || !device[10..].bytes().all(|value| value.is_ascii_digit())
                || device.len() == 10
            {
                return Err(CoreError::InvalidInput(
                    "choose a local /dev/videoN camera device".into(),
                ));
            }
            vec!["-f".into(), "v4l2".into(), "-i".into(), device.into()]
        }
        Os::Other => return Err(CoreError::Unsupported(os.display_name())),
    })
}
pub fn plan(os: Os, device: &str) -> CoreResult<Vec<String>> {
    let mut args = camera_input(os, device)?;
    args.extend([
        "-an".into(),
        "-frames:v".into(),
        "1".into(),
        "-update".into(),
        "1".into(),
    ]);
    Ok(args)
}
pub fn capture(
    device: &str,
    output: &std::path::Path,
    cancel: &crate::jobs::CancelToken,
    progress: &mut dyn FnMut(crate::jobs::Progress),
) -> CoreResult<std::path::PathBuf> {
    let args = plan(Os::current(), device)?;
    super::media_provider::ffmpeg_job(&args, output, "png", cancel, progress)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn still_plan_uses_one_frame_only() {
        let args = plan(crate::platform::Os::Windows, "Camera").unwrap();
        assert!(args.windows(2).any(|pair| pair == ["-frames:v", "1"]));
        assert!(plan(crate::platform::Os::Windows, "").is_err());
    }
}

//! Explicit screen, area and window recording.
use super::record_audio::validate_duration;
use crate::{CoreError, CoreResult, platform::Os};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScreenSource {
    Desktop,
    Area {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    Window {
        title: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRequest {
    pub source: ScreenSource,
    pub fps: u32,
    pub seconds: u64,
}
pub fn plan(os: Os, request: &ScreenRequest) -> CoreResult<Vec<String>> {
    validate_duration(request.seconds)?;
    if !(1..=60).contains(&request.fps) {
        return Err(CoreError::InvalidInput("frame rate must be 1–60".into()));
    }
    let mut args = vec!["-f".into()];
    match os {
        Os::Windows => {
            args.extend([
                "gdigrab".into(),
                "-framerate".into(),
                request.fps.to_string(),
            ]);
            match &request.source {
                ScreenSource::Desktop => args.extend(["-i".into(), "desktop".into()]),
                ScreenSource::Window { title } => {
                    super::record_audio::validate_device(title)?;
                    args.extend(["-i".into(), format!("title={title}")]);
                }
                ScreenSource::Area {
                    x,
                    y,
                    width,
                    height,
                } => {
                    if *width == 0 || *height == 0 || *width > 16_384 || *height > 16_384 {
                        return Err(CoreError::InvalidInput(
                            "area dimensions must be 1–16384 pixels".into(),
                        ));
                    }
                    args.extend([
                        "-offset_x".into(),
                        x.to_string(),
                        "-offset_y".into(),
                        y.to_string(),
                        "-video_size".into(),
                        format!("{width}x{height}"),
                        "-i".into(),
                        "desktop".into(),
                    ]);
                }
            }
        }
        Os::Linux => {
            let display=std::env::var("DISPLAY").map_err(|_| CoreError::Unsupported("this Linux session (FFmpeg X11 recording requires DISPLAY; Wayland capture needs a portal provider)"))?;
            if display.len() > 100
                || display.contains(['\0', '\n'])
                || !(display.starts_with(':') || display.starts_with("unix:"))
            {
                return Err(CoreError::InvalidInput("invalid X11 display".into()));
            }
            args.extend([
                "x11grab".into(),
                "-framerate".into(),
                request.fps.to_string(),
            ]);
            match request.source {
                ScreenSource::Desktop => args.extend(["-i".into(), display]),
                ScreenSource::Area {
                    x,
                    y,
                    width,
                    height,
                } => {
                    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
                        return Err(CoreError::InvalidInput("invalid recording area".into()));
                    }
                    args.extend([
                        "-video_size".into(),
                        format!("{width}x{height}"),
                        "-i".into(),
                        format!("{display}+{x},{y}"),
                    ]);
                }
                ScreenSource::Window { .. } => {
                    return Err(CoreError::Unsupported(
                        "Linux window recording (select its area instead)",
                    ));
                }
            }
        }
        Os::Macos => {
            return Err(CoreError::Unsupported(
                "macOS screen recording through this provider (use the OS recording controls)",
            ));
        }
        Os::Other => return Err(CoreError::Unsupported(os.display_name())),
    }
    args.extend([
        "-t".into(),
        request.seconds.to_string(),
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
    fn capture_plan_keeps_window_titles_out_of_shells() {
        let request = ScreenRequest {
            source: ScreenSource::Window {
                title: "A; B".into(),
            },
            fps: 15,
            seconds: 60,
        };
        let args = plan(crate::platform::Os::Windows, &request).unwrap();
        assert!(args.contains(&"title=A; B".to_string()));
        assert!(
            plan(
                crate::platform::Os::Linux,
                &ScreenRequest { fps: 0, ..request }
            )
            .is_err()
        );
    }
}

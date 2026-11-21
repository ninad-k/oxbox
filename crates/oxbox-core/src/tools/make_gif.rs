//! Bounded local video to GIF conversion.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GifOptions {
    pub start_seconds: f64,
    pub seconds: f64,
    pub fps: u32,
    pub width: u32,
}
pub fn plan(options: &GifOptions) -> CoreResult<Vec<String>> {
    if !options.start_seconds.is_finite()
        || options.start_seconds < 0.0
        || options.start_seconds > 7.0 * 24.0 * 3600.0
        || !options.seconds.is_finite()
        || !(0.1..=60.0).contains(&options.seconds)
        || !(1..=30).contains(&options.fps)
        || !(16..=1280).contains(&options.width)
    {
        return Err(CoreError::InvalidInput(
            "GIF: choose a finite start, duration 0.1–60s, 1–30 FPS and width 16–1280".into(),
        ));
    }
    Ok(vec![
        "-ss".into(),
        options.start_seconds.to_string(),
        "-t".into(),
        options.seconds.to_string(),
        "-filter_complex".into(),
        format!(
            "fps={},scale={}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            options.fps, options.width
        ),
    ])
}
pub fn make(
    input: &std::path::Path,
    output: &std::path::Path,
    options: &GifOptions,
    cancel: &crate::jobs::CancelToken,
    progress: &mut dyn FnMut(crate::jobs::Progress),
) -> CoreResult<std::path::PathBuf> {
    let mut args = super::media_provider::file_input(input)?;
    args.extend(plan(options)?);
    super::media_provider::ffmpeg_job(&args, output, "gif", cancel, progress)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gif_filter_comes_only_from_validated_numbers() {
        let args = plan(&GifOptions {
            start_seconds: 0.0,
            seconds: 5.0,
            fps: 10,
            width: 640,
        })
        .unwrap();
        assert!(args.iter().any(|arg| arg.contains("palettegen")));
        assert!(
            plan(&GifOptions {
                start_seconds: f64::NAN,
                seconds: 5.0,
                fps: 10,
                width: 640
            })
            .is_err()
        );
        assert!(
            plan(&GifOptions {
                start_seconds: 0.0,
                seconds: 3600.0,
                fps: 10,
                width: 640
            })
            .is_err()
        );
    }
}

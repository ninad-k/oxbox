//! Local audio/video format conversion with fixed codec presets.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaFormat {
    Mp4,
    Webm,
    Mp3,
    Wav,
    Flac,
}
impl MediaFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}
pub fn plan(format: MediaFormat) -> Vec<String> {
    let args: &[&str] = match format {
        MediaFormat::Mp4 => &[
            "-c:v",
            "mpeg4",
            "-q:v",
            "5",
            "-vf",
            "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ],
        MediaFormat::Webm => &[
            "-c:v",
            "libvpx-vp9",
            "-crf",
            "32",
            "-b:v",
            "0",
            "-c:a",
            "libopus",
        ],
        MediaFormat::Mp3 => &["-vn", "-c:a", "libmp3lame", "-q:a", "2"],
        MediaFormat::Wav => &["-vn", "-c:a", "pcm_s16le", "-ar", "48000"],
        MediaFormat::Flac => &["-vn", "-c:a", "flac"],
    };
    args.iter().map(|value| (*value).to_string()).collect()
}
pub fn convert(
    input: &std::path::Path,
    output: &std::path::Path,
    format: MediaFormat,
    cancel: &crate::jobs::CancelToken,
    progress: &mut dyn FnMut(crate::jobs::Progress),
) -> crate::CoreResult<std::path::PathBuf> {
    let mut args = super::media_provider::file_input(input)?;
    args.extend(plan(format));
    super::media_provider::ffmpeg_job(&args, output, format.extension(), cancel, progress)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_do_not_accept_arbitrary_arguments() {
        assert!(plan(MediaFormat::Wav).contains(&"pcm_s16le".to_string()));
        assert!(plan(MediaFormat::Mp4).contains(&"mpeg4".to_string()));
        assert_eq!(MediaFormat::Webm.extension(), "webm");
    }

    #[test]
    fn referencing_manifest_inputs_do_not_enable_manifest_demuxers_or_external_mov_tracks() {
        let directory = tempfile::tempdir().unwrap();
        for (name, contents) in [
            (
                "fixture.m3u8",
                "#EXTM3U\n#EXTINF:1,\nfile://oxbox-fixture.invalid/share/segment.ts\n",
            ),
            (
                "fixture.ffconcat",
                "ffconcat version 1.0\nfile '\\\\oxbox-fixture.invalid\\share\\segment.wav'\n",
            ),
            (
                "fixture.mpd",
                "<MPD><BaseURL>https://oxbox-fixture.invalid/</BaseURL></MPD>",
            ),
        ] {
            let input = directory.path().join(name);
            std::fs::write(&input, contents).unwrap();
            // Only build the literal input plan. Never invoke a provider or open
            // any reference from these generated manifest contents.
            let args = super::super::media_provider::file_input(&input).unwrap();
            let formats = args
                .windows(2)
                .find(|pair| pair[0] == "-format_whitelist")
                .unwrap()[1]
                .split(',')
                .collect::<Vec<_>>();
            for excluded in [
                "hls",
                "concat",
                "dash",
                "imf",
                "image2",
                "sdp",
                "avisynth",
                "vapoursynth",
            ] {
                assert!(!formats.contains(&excluded), "{name}: {excluded}");
            }
            assert!(
                args.windows(2)
                    .any(|pair| pair == ["-protocol_whitelist", "file,pipe"])
            );
            assert!(
                !args
                    .iter()
                    .any(|arg| ["-enable_drefs", "-use_absolute_path"].contains(&arg.as_str()))
            );
        }
    }
}

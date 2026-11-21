//! Explicit, non-device provider checks. Never runs automatically in the unit suite.
use oxbox_core::{
    jobs::CancelToken,
    tools::{
        convert_media::{self, MediaFormat},
        currency,
        make_gif::{self, GifOptions},
        media_provider,
    },
};
use std::{
    io::Read,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("--media") => {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let folder = std::env::temp_dir().join(format!(
                "oxbox-provider-smoke-{}-{stamp}",
                std::process::id()
            ));
            std::fs::create_dir(&folder)?;
            let input = folder.join("synthetic.mp4");
            let token = CancelToken::default();
            let ffmpeg = media_provider::find_executable("ffmpeg")?;
            let args = [
                "-hide_banner",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "color=c=orange:s=160x120:r=10:d=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-shortest",
                "-c:v",
                "mpeg4",
                "-c:a",
                "aac",
            ]
            .into_iter()
            .map(String::from)
            .chain(std::iter::once(input.to_string_lossy().into_owned()))
            .collect::<Vec<_>>();
            media_provider::run_text(&ffmpeg, &args, &token, Duration::from_secs(60), 65536)?;
            let output = folder.join("converted.webm");
            convert_media::convert(&input, &output, MediaFormat::Webm, &token, &mut |_| {})?;
            if convert_media::convert(&input, &output, MediaFormat::Webm, &token, &mut |_| {})
                .is_ok()
            {
                return Err("An existing media output was unexpectedly replaced".into());
            }
            let gif = folder.join("animation.gif");
            make_gif::make(
                &input,
                &gif,
                &GifOptions {
                    start_seconds: 0.0,
                    seconds: 1.0,
                    fps: 10,
                    width: 160,
                },
                &token,
                &mut |_| {},
            )?;
            let mut header = [0u8; 6];
            std::fs::File::open(&gif)?.read_exact(&mut header)?;
            if &header != b"GIF87a" && &header != b"GIF89a" {
                return Err("Invalid GIF header".into());
            }
            println!(
                "Synthetic media conversion, GIF creation and overwrite refusal passed. Fixtures retained at {}",
                folder.display()
            );
        }
        Some("--currency-network") => {
            let book = currency::download(true)?;
            let usd = currency::convert(1.0, "EUR", "USD", &book)?;
            println!(
                "Fixed ECB HTTPS provider parsed {} dated currency rates ({}) and converted a reference amount: {usd}",
                book.rates.len(),
                book.date
            );
        }
        _ => println!(
            "Explicit checks: --media (synthetic fixtures, no devices), or --currency-network (opt-in ECB request, no saved preferences)."
        ),
    }
    Ok(())
}

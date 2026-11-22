//! Offline, flattened screenshot/image edits; originals are never modified.

use crate::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use image::{ImageEncoder, Rgba, RgbaImage};
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

const MAX_PIXELS: u64 = 40_000_000;
const MAX_DIMENSION: u32 = 32_768;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub enum Edit {
    Crop(PixelRect),
    Redact {
        rect: PixelRect,
        color: [u8; 3],
    },
    Arrow {
        from: (u32, u32),
        to: (u32, u32),
        width: u32,
        color: [u8; 3],
    },
    Text {
        at: (u32, u32),
        text: String,
        scale: u32,
        color: [u8; 3],
    },
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn validate_image(image: &RgbaImage) -> CoreResult<()> {
    let (width, height) = image.dimensions();
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err(invalid(
            "image editing is limited to 40 million pixels and 32768 pixels per side",
        ));
    }
    Ok(())
}
fn validate_rect(image: &RgbaImage, rect: PixelRect) -> CoreResult<()> {
    if rect.width == 0
        || rect.height == 0
        || rect
            .x
            .checked_add(rect.width)
            .is_none_or(|end| end > image.width())
        || rect
            .y
            .checked_add(rect.height)
            .is_none_or(|end| end > image.height())
    {
        return Err(invalid(
            "choose a nonempty rectangle entirely inside the image",
        ));
    }
    Ok(())
}
fn validate_point(image: &RgbaImage, point: (u32, u32)) -> CoreResult<()> {
    if point.0 >= image.width() || point.1 >= image.height() {
        return Err(invalid("annotation coordinates must be inside the image"));
    }
    Ok(())
}

/// Return a new flattened bitmap. Coordinates always refer to the current bitmap;
/// cropping changes the coordinate origin for subsequent edits. RGB and alpha of
/// every redacted pixel are replaced, without retaining layers or source metadata.
pub fn apply(image: &RgbaImage, edit: &Edit, cancel: &CancelToken) -> CoreResult<RgbaImage> {
    cancel.check()?;
    validate_image(image)?;
    match edit {
        Edit::Crop(rect) | Edit::Redact { rect, .. } => validate_rect(image, *rect)?,
        Edit::Arrow {
            from, to, width, ..
        } => {
            validate_point(image, *from)?;
            validate_point(image, *to)?;
            if from == to || !(1..=32).contains(width) {
                return Err(invalid(
                    "choose distinct arrow endpoints and a stroke width between 1 and 32 pixels",
                ));
            }
        }
        Edit::Text {
            at, text, scale, ..
        } => {
            validate_point(image, *at)?;
            if text.is_empty()
                || text.len() > 256
                || !(1..=8).contains(scale)
                || text.chars().any(|c| c != '\n' && !(' '..='~').contains(&c))
            {
                return Err(invalid(
                    "labels support 1–256 printable ASCII characters/newlines and scale 1–8; Unicode labels are unavailable",
                ));
            }
            let lines: Vec<_> = text.split('\n').collect();
            let columns = lines.iter().map(|line| line.len()).max().unwrap_or(0) as u32;
            let width = columns
                .saturating_mul(6)
                .saturating_sub(1)
                .saturating_mul(*scale);
            let height = (lines.len() as u32)
                .saturating_mul(8)
                .saturating_sub(1)
                .saturating_mul(*scale);
            if lines.len() > 12
                || at
                    .0
                    .checked_add(width)
                    .is_none_or(|end| end > image.width())
                || at
                    .1
                    .checked_add(height)
                    .is_none_or(|end| end > image.height())
            {
                return Err(invalid(
                    "the complete text label must fit inside the image (at most 12 lines)",
                ));
            }
        }
    }
    if let Edit::Crop(rect) = edit {
        let output =
            image::imageops::crop_imm(image, rect.x, rect.y, rect.width, rect.height).to_image();
        cancel.check()?;
        return Ok(output);
    }
    let mut output = image.clone();
    match edit {
        Edit::Crop(_) => {}
        Edit::Redact { rect, color } => {
            for y in rect.y..rect.y + rect.height {
                cancel.check()?;
                for x in rect.x..rect.x + rect.width {
                    output.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
                }
            }
        }
        Edit::Arrow {
            from,
            to,
            width,
            color,
        } => {
            let from = (i64::from(from.0), i64::from(from.1));
            let to = (i64::from(to.0), i64::from(to.1));
            draw_line(&mut output, from, to, *width, *color, cancel)?;
            let dx = (to.0 - from.0) as f64;
            let dy = (to.1 - from.1) as f64;
            let length = dx.hypot(dy);
            let head = (f64::from(*width) * 5.0 + 8.0).min(length * 0.45);
            for side in [-1.0, 1.0] {
                let tip = (
                    (to.0 as f64 - dx / length * head + side * dy / length * head * 0.55).round()
                        as i64,
                    (to.1 as f64 - dy / length * head - side * dx / length * head * 0.55).round()
                        as i64,
                );
                draw_line(&mut output, to, tip, *width, *color, cancel)?;
            }
        }
        Edit::Text {
            at,
            text,
            scale,
            color,
        } => {
            for (line, value) in text.split('\n').enumerate() {
                for (column, value) in value.chars().enumerate() {
                    cancel.check()?;
                    for (row, bits) in glyph(value).into_iter().enumerate() {
                        for bit in 0..5 {
                            if bits & (1 << (4 - bit)) == 0 {
                                continue;
                            }
                            for sy in 0..*scale {
                                for sx in 0..*scale {
                                    let x = at.0 + (column as u32 * 6 + bit) * scale + sx;
                                    let y = at.1 + (line as u32 * 8 + row as u32) * scale + sy;
                                    output.put_pixel(
                                        x,
                                        y,
                                        Rgba([color[0], color[1], color[2], 255]),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    cancel.check()?;
    Ok(output)
}

fn draw_line(
    image: &mut RgbaImage,
    mut from: (i64, i64),
    to: (i64, i64),
    width: u32,
    color: [u8; 3],
    cancel: &CancelToken,
) -> CoreResult<()> {
    let dx = (to.0 - from.0).abs();
    let dy = -(to.1 - from.1).abs();
    let sx = if from.0 < to.0 { 1 } else { -1 };
    let sy = if from.1 < to.1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        cancel.check()?;
        let start = -(i64::from(width) / 2);
        for oy in start..start + i64::from(width) {
            for ox in start..start + i64::from(width) {
                let (x, y) = (from.0 + ox, from.1 + oy);
                if x >= 0 && y >= 0 && x < i64::from(image.width()) && y < i64::from(image.height())
                {
                    image.put_pixel(
                        x as u32,
                        y as u32,
                        Rgba([color[0], color[1], color[2], 255]),
                    );
                }
            }
        }
        if from == to {
            break;
        }
        let doubled = 2 * error;
        if doubled >= dy {
            error += dy;
            from.0 += sx;
        }
        if doubled <= dx {
            error += dx;
            from.1 += sy;
        }
    }
    Ok(())
}

// Original hand-authored 5×7 bitmap alphabet; no external font data or loader.
fn glyph(value: char) -> [u8; 7] {
    match value {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        'a' => [0, 0, 14, 1, 15, 17, 15],
        'b' => [16, 16, 22, 25, 17, 17, 30],
        'c' => [0, 0, 14, 17, 16, 17, 14],
        'd' => [1, 1, 13, 19, 17, 17, 15],
        'e' => [0, 0, 14, 17, 31, 16, 14],
        'f' => [6, 9, 8, 28, 8, 8, 8],
        'g' => [0, 15, 17, 17, 15, 1, 14],
        'h' => [16, 16, 22, 25, 17, 17, 17],
        'i' => [4, 0, 12, 4, 4, 4, 14],
        'j' => [2, 0, 6, 2, 2, 18, 12],
        'k' => [16, 16, 17, 18, 28, 18, 17],
        'l' => [12, 4, 4, 4, 4, 4, 14],
        'm' => [0, 0, 26, 21, 21, 17, 17],
        'n' => [0, 0, 22, 25, 17, 17, 17],
        'o' => [0, 0, 14, 17, 17, 17, 14],
        'p' => [0, 30, 17, 17, 30, 16, 16],
        'q' => [0, 15, 17, 17, 15, 1, 1],
        'r' => [0, 0, 22, 25, 16, 16, 16],
        's' => [0, 0, 15, 16, 14, 1, 30],
        't' => [8, 8, 28, 8, 8, 9, 6],
        'u' => [0, 0, 17, 17, 17, 19, 13],
        'v' => [0, 0, 17, 17, 17, 10, 4],
        'w' => [0, 0, 17, 17, 21, 21, 10],
        'x' => [0, 0, 17, 10, 4, 10, 17],
        'y' => [0, 17, 17, 17, 15, 1, 14],
        'z' => [0, 0, 31, 2, 4, 8, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '"' => [10, 10, 10, 0, 0, 0, 0],
        '#' => [10, 31, 10, 10, 31, 10, 0],
        '$' => [4, 15, 20, 14, 5, 30, 4],
        '%' => [24, 25, 2, 4, 8, 19, 3],
        '&' => [12, 18, 20, 8, 21, 18, 13],
        '\'' => [4, 4, 8, 0, 0, 0, 0],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '*' => [0, 21, 14, 31, 14, 21, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        ',' => [0, 0, 0, 0, 0, 4, 8],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0, 4],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        ':' => [0, 4, 0, 0, 4, 0, 0],
        ';' => [0, 4, 0, 0, 4, 4, 8],
        '<' => [1, 2, 4, 8, 4, 2, 1],
        '=' => [0, 0, 31, 0, 31, 0, 0],
        '>' => [16, 8, 4, 2, 4, 8, 16],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '@' => [14, 17, 23, 21, 23, 16, 14],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        '\\' => [16, 8, 8, 4, 2, 2, 1],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        '^' => [4, 10, 17, 0, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '`' => [8, 4, 2, 0, 0, 0, 0],
        '{' => [2, 4, 4, 8, 4, 4, 2],
        '|' => [4, 4, 4, 4, 4, 4, 4],
        '}' => [8, 4, 4, 2, 4, 4, 8],
        '~' => [0, 0, 8, 21, 2, 0, 0],
        _ => [0; 7],
    }
}

struct CancelWriter<'a, W> {
    inner: W,
    cancel: &'a CancelToken,
}
impl<W: Write> Write for CancelWriter<'_, W> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.cancel
            .check()
            .map_err(|_| std::io::Error::other("cancelled"))?;
        self.inner.write(data)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.cancel
            .check()
            .map_err(|_| std::io::Error::other("cancelled"))?;
        self.inner.flush()
    }
}

/// Encode only flattened RGBA pixels into a new PNG. Never replaces any existing
/// file, including the original. Removes this operation's incomplete output.
pub fn save_png_exclusive(
    image: &RgbaImage,
    output: &Path,
    cancel: &CancelToken,
) -> CoreResult<()> {
    cancel.check()?;
    validate_image(image)?;
    crate::fs_safe::validate_output_filename(
        output
            .file_name()
            .ok_or_else(|| invalid("choose a new PNG filename"))?,
    )?;
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    crate::fs_safe::reject_symlink_components(parent)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(output)?;
    let result = (|| {
        let mut writer = CancelWriter {
            inner: BufWriter::new(file),
            cancel,
        };
        image::codecs::png::PngEncoder::new(&mut writer)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| invalid(format!("PNG export failed: {error}")))?;
        writer.flush()?;
        writer.inner.get_ref().sync_all()?;
        cancel.check()
    })();
    if result.is_err() {
        if let Err(error) = fs::remove_file(output) {
            return Err(invalid(format!(
                "incomplete PNG cleanup failed at {}: {error}",
                output.display()
            )));
        }
        cancel.check()?;
    }
    result
}

/// Explicitly recognize the current flattened image using the existing offline
/// OCR backend. A private temporary PNG is removed before text is returned;
/// failures to clean up are reported with its directory for manual recovery.
pub fn recognize(
    image: &RgbaImage,
    provider: &str,
    language: &str,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<String> {
    recognize_using(image, &std::env::temp_dir(), cancel, |path| {
        super::ocr::recognize(path, provider, language, cancel, progress)
    })
}

fn recognize_using(
    image: &RgbaImage,
    parent: &Path,
    cancel: &CancelToken,
    recognize: impl FnOnce(&Path) -> CoreResult<String>,
) -> CoreResult<String> {
    cancel.check()?;
    validate_image(image)?;
    let mut folder = super::media_provider::TemporaryFolder::new(parent)?;
    let path = folder.file("flattened-ocr.png")?;
    let result = (|| {
        save_png_exclusive(image, &path, cancel)?;
        cancel.check()?;
        let text = recognize(&path)?;
        cancel.check()?;
        Ok(text)
    })();
    let file_cleanup = fs::remove_file(&path).or_else(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(error)
        }
    });
    let directory_cleanup = fs::remove_dir(folder.path());
    if let Err(error) = file_cleanup.and(directory_cleanup) {
        return Err(invalid(format!(
            "temporary OCR image cleanup failed; check {}: {error}",
            folder.path().display()
        )));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoreError, jobs::CancelToken};
    use image::{Rgba, RgbaImage};

    fn generated_image() -> RgbaImage {
        RgbaImage::from_fn(8, 6, |x, y| Rgba([x as u8 * 11, y as u8 * 17, 99, 37]))
    }

    #[test]
    fn crop_keeps_exact_pixels_and_preserves_the_original() {
        let original = generated_image();
        let before = original.clone();
        let cropped = apply(
            &original,
            &Edit::Crop(PixelRect {
                x: 2,
                y: 1,
                width: 4,
                height: 3,
            }),
            &CancelToken::default(),
        )
        .unwrap();
        assert_eq!(cropped.dimensions(), (4, 3));
        assert_eq!(cropped.get_pixel(0, 0), original.get_pixel(2, 1));
        assert_eq!(cropped.get_pixel(3, 2), original.get_pixel(5, 3));
        assert_eq!(original, before);
    }

    #[test]
    fn crop_and_redaction_reject_zero_outside_and_overflowing_bounds() {
        let original = generated_image();
        for rect in [
            PixelRect {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
            PixelRect {
                x: 7,
                y: 0,
                width: 2,
                height: 1,
            },
            PixelRect {
                x: 0,
                y: 5,
                width: 1,
                height: 2,
            },
            PixelRect {
                x: u32::MAX,
                y: 0,
                width: 2,
                height: 1,
            },
        ] {
            for edit in [
                Edit::Crop(rect),
                Edit::Redact {
                    rect,
                    color: [0, 0, 0],
                },
            ] {
                assert_eq!(
                    apply(&original, &edit, &CancelToken::default())
                        .unwrap_err()
                        .code(),
                    "invalid_input"
                );
            }
        }
    }

    #[test]
    fn solid_redaction_overwrites_all_channels_and_png_exports_only_flattened_pixels() {
        let directory = tempfile::tempdir().unwrap();
        let original = generated_image();
        let before = original.clone();
        let rect = PixelRect {
            x: 1,
            y: 1,
            width: 4,
            height: 3,
        };
        let flattened = apply(
            &original,
            &Edit::Redact {
                rect,
                color: [3, 4, 5],
            },
            &CancelToken::default(),
        )
        .unwrap();
        for (x, y, pixel) in flattened.enumerate_pixels() {
            if (1..5).contains(&x) && (1..4).contains(&y) {
                assert_eq!(
                    pixel.0,
                    [3, 4, 5, 255],
                    "redaction must destroy RGB and alpha under every covered pixel"
                );
            } else {
                assert_eq!(pixel, original.get_pixel(x, y));
            }
        }
        let output = directory.path().join("flattened.png");
        save_png_exclusive(&flattened, &output, &CancelToken::default()).unwrap();
        let decoded = image::open(&output).unwrap().into_rgba8();
        assert_eq!(decoded, flattened);
        assert!(save_png_exclusive(&original, &output, &CancelToken::default()).is_err());
        assert_eq!(image::open(&output).unwrap().into_rgba8(), flattened);
        assert_eq!(original, before);
    }

    #[test]
    fn arrow_and_bitmap_text_draw_real_distinct_pixels_with_validated_bounds() {
        let original = RgbaImage::from_pixel(128, 32, Rgba([255, 255, 255, 255]));
        let arrow = apply(
            &original,
            &Edit::Arrow {
                from: (2, 2),
                to: (30, 20),
                width: 2,
                color: [255, 0, 0],
            },
            &CancelToken::default(),
        )
        .unwrap();
        assert_eq!(arrow.get_pixel(2, 2).0, [255, 0, 0, 255]);
        assert_eq!(arrow.get_pixel(30, 20).0, [255, 0, 0, 255]);
        let text = |value: &str| Edit::Text {
            at: (2, 2),
            text: value.into(),
            scale: 2,
            color: [0, 0, 0],
        };
        let a = apply(&original, &text("A"), &CancelToken::default()).unwrap();
        let b = apply(&original, &text("B"), &CancelToken::default()).unwrap();
        assert_ne!(a, original);
        assert_ne!(a, b, "labels must render their actual content");
        assert_eq!(
            apply(&original, &text("東京"), &CancelToken::default())
                .unwrap_err()
                .code(),
            "invalid_input"
        );
        assert!(
            apply(
                &original,
                &Edit::Text {
                    at: (127, 0),
                    text: "A".into(),
                    scale: 2,
                    color: [0, 0, 0]
                },
                &CancelToken::default()
            )
            .is_err()
        );
        assert!(
            apply(
                &original,
                &Edit::Arrow {
                    from: (2, 2),
                    to: (128, 20),
                    width: 2,
                    color: [0, 0, 0]
                },
                &CancelToken::default()
            )
            .is_err()
        );
    }

    #[test]
    fn cancelled_edit_and_export_create_no_file() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("cancelled.png");
        let cancel = CancelToken::default();
        cancel.cancel();
        let image = generated_image();
        assert_eq!(
            apply(
                &image,
                &Edit::Crop(PixelRect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1
                }),
                &cancel
            )
            .unwrap_err()
            .code(),
            "cancelled"
        );
        assert_eq!(
            save_png_exclusive(&image, &output, &cancel)
                .unwrap_err()
                .code(),
            "cancelled"
        );
        assert!(!output.exists());
    }

    #[test]
    fn explicit_ocr_uses_flattened_private_image_and_removes_its_owned_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let image = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255]));
        let mut used = None;
        let text = recognize_using(&image, directory.path(), &CancelToken::default(), |path| {
            assert_eq!(image::open(path).unwrap().into_rgba8(), image);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
                assert_eq!(
                    std::fs::metadata(path.parent().unwrap())
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o700
                );
            }
            used = Some(path.to_path_buf());
            Ok("Generated recognition".to_string())
        })
        .unwrap();
        assert_eq!(text, "Generated recognition");
        assert!(!used.unwrap().exists());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn ocr_failure_and_late_cancel_clean_up_without_publishing_text() {
        let directory = tempfile::tempdir().unwrap();
        let image = generated_image();
        let token = CancelToken::default();
        let result = recognize_using(&image, directory.path(), &token, |_| {
            token.cancel();
            Ok("Never publish this generated stale text".to_string())
        });
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        let error = recognize_using(&image, directory.path(), &CancelToken::default(), |_| {
            Err(CoreError::InvalidInput("Generated OCR refusal".into()))
        })
        .unwrap_err();
        assert!(error.to_string().contains("Generated OCR refusal"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn ocr_cleanup_failure_is_visible_and_does_not_recursively_delete_unowned_files() {
        let directory = tempfile::tempdir().unwrap();
        let mut retained = None;
        let error = recognize_using(
            &generated_image(),
            directory.path(),
            &CancelToken::default(),
            |path| {
                let unexpected = path.parent().unwrap().join("generated-unowned-marker.txt");
                std::fs::write(&unexpected, b"preserve this generated marker").unwrap();
                retained = Some(unexpected);
                Ok("Do not publish success while temporary cleanup failed".into())
            },
        )
        .unwrap_err();
        assert!(error.to_string().to_lowercase().contains("cleanup"));
        assert_eq!(
            std::fs::read(retained.unwrap()).unwrap(),
            b"preserve this generated marker"
        );
    }
}

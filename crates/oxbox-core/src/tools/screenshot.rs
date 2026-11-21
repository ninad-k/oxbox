//! Screen capture and HiDPI selection geometry.

use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use image::ImageEncoder;
pub use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult};

static CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn capture_after_delay<T>(
    delay: std::time::Duration,
    cancel: &crate::jobs::CancelToken,
    capture: impl FnOnce() -> CoreResult<T>,
) -> CoreResult<T> {
    cancel.check()?;
    let until = std::time::Instant::now()
        .checked_add(delay)
        .ok_or_else(|| CoreError::InvalidInput("capture delay exceeds the clock range".into()))?;
    while let Some(remaining) = until.checked_duration_since(std::time::Instant::now()) {
        cancel.check()?;
        std::thread::sleep(remaining.min(std::time::Duration::from_millis(50)));
    }
    cancel.check()?;
    capture()
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Selection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Convert logical overlay coordinates into a pixel crop, rounding outwards.
pub fn selection_pixels(
    selection: Selection,
    scale: f64,
    width: u32,
    height: u32,
) -> CoreResult<PixelRect> {
    let values = [
        selection.x,
        selection.y,
        selection.width,
        selection.height,
        scale,
    ];
    if values.iter().any(|value| !value.is_finite())
        || scale <= 0.0
        || selection.width <= 0.0
        || selection.height <= 0.0
    {
        return Err(CoreError::InvalidInput(
            "select a nonempty area with a valid display scale".into(),
        ));
    }
    let x = (selection.x * scale).floor().clamp(0.0, f64::from(width)) as u32;
    let y = (selection.y * scale).floor().clamp(0.0, f64::from(height)) as u32;
    let right = ((selection.x + selection.width) * scale)
        .ceil()
        .clamp(0.0, f64::from(width)) as u32;
    let bottom = ((selection.y + selection.height) * scale)
        .ceil()
        .clamp(0.0, f64::from(height)) as u32;
    if right <= x || bottom <= y {
        return Err(CoreError::InvalidInput(
            "the selection is outside the display".into(),
        ));
    }
    Ok(PixelRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

pub fn crop_selection(
    image: &RgbaImage,
    selection: Selection,
    scale: f64,
) -> CoreResult<RgbaImage> {
    let crop = selection_pixels(selection, scale, image.width(), image.height())?;
    Ok(image::imageops::crop_imm(image, crop.x, crop.y, crop.width, crop.height).to_image())
}

pub fn capture_filename(timestamp_ms: u128, sequence: u64) -> String {
    format!("Oxbox-{timestamp_ms}-{sequence}.png")
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureKind {
    Monitor,
    Window,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CaptureTarget {
    pub kind: CaptureKind,
    pub id: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub kind: CaptureKind,
    pub id: u32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

fn capture_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("screen capture failed: {error}"))
}

pub fn targets() -> CoreResult<Vec<TargetInfo>> {
    let mut targets = Vec::new();
    for monitor in xcap::Monitor::all().map_err(capture_error)? {
        let info = (|| -> CoreResult<TargetInfo> {
            Ok(TargetInfo {
                kind: CaptureKind::Monitor,
                id: monitor.id().map_err(capture_error)?,
                name: monitor.name().map_err(capture_error)?,
                x: monitor.x().map_err(capture_error)?,
                y: monitor.y().map_err(capture_error)?,
                width: monitor.width().map_err(capture_error)?,
                height: monitor.height().map_err(capture_error)?,
                scale_factor: monitor.scale_factor().map_err(capture_error)?,
            })
        })();
        // A monitor can disappear between enumeration and reading its metadata.
        if let Ok(info) = info {
            targets.push(info);
        }
    }
    // A platform can allow screen capture while denying window enumeration.
    if let Ok(windows) = xcap::Window::all() {
        for window in windows {
            if window.is_minimized().unwrap_or(true) {
                continue;
            }
            let Ok(title) = window.title() else {
                continue;
            };
            if title.trim().is_empty() || title == "Oxbox" {
                continue;
            }
            let (Ok(id), Ok(x), Ok(y), Ok(width), Ok(height)) = (
                window.id(),
                window.x(),
                window.y(),
                window.width(),
                window.height(),
            ) else {
                continue;
            };
            if width == 0 || height == 0 {
                continue;
            }
            targets.push(TargetInfo {
                kind: CaptureKind::Window,
                id,
                name: title,
                x,
                y,
                width,
                height,
                scale_factor: 1.0,
            });
        }
    }
    Ok(targets)
}

pub fn capture_image(target: CaptureTarget) -> CoreResult<RgbaImage> {
    match target.kind {
        CaptureKind::Monitor => {
            let monitor = xcap::Monitor::all()
                .map_err(capture_error)?
                .into_iter()
                .find(|monitor| monitor.id().ok() == Some(target.id))
                .ok_or_else(|| {
                    CoreError::InvalidInput("the selected display is no longer available".into())
                })?;
            monitor.capture_image().map_err(capture_error)
        }
        CaptureKind::Window => {
            let window = xcap::Window::all()
                .map_err(capture_error)?
                .into_iter()
                .find(|window| window.id().ok() == Some(target.id))
                .ok_or_else(|| {
                    CoreError::InvalidInput("the selected window is no longer available".into())
                })?;
            window.capture_image().map_err(capture_error)
        }
    }
}

pub fn output_directory() -> CoreResult<PathBuf> {
    let pictures = dirs::picture_dir().ok_or_else(|| {
        CoreError::InvalidInput(
            "the Pictures folder is unavailable; choose an output folder".into(),
        )
    })?;
    Ok(pictures.join("Oxbox"))
}

pub fn encode_png(image: &RgbaImage) -> CoreResult<Vec<u8>> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(capture_error)?;
    Ok(bytes)
}

/// Create-new prevents a capture from replacing an existing file, including a symlink.
pub fn save_capture(image: &RgbaImage, folder: &Path) -> CoreResult<PathBuf> {
    fs::create_dir_all(folder)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(capture_error)?
        .as_millis();
    let sequence = CAPTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = folder.join(capture_filename(timestamp, sequence));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let result = (|| {
        let mut writer = BufWriter::new(file);
        image::codecs::png::PngEncoder::new(&mut writer)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(capture_error)?;
        writer.flush()?;
        Ok::<(), CoreError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
    }
    result?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_media_bounds_cancelled_delay_does_not_capture() {
        let cancel = crate::jobs::CancelToken::default();
        cancel.cancel();
        let called = std::cell::Cell::new(false);
        let result = capture_after_delay(std::time::Duration::ZERO, &cancel, || {
            called.set(true);
            Ok(())
        });
        assert!(
            !called.get(),
            "a cancelled delayed action must never invoke the capture callback"
        );
        assert_eq!(result.unwrap_err().code(), "cancelled");
    }

    #[test]
    fn scaled_selection_rounds_outward_and_clamps() {
        let crop = selection_pixels(
            Selection {
                x: 1.1,
                y: 2.1,
                width: 10.2,
                height: 6.1,
            },
            1.5,
            100,
            100,
        )
        .unwrap();
        assert_eq!(
            crop,
            PixelRect {
                x: 1,
                y: 3,
                width: 16,
                height: 10
            }
        );
        let crop = selection_pixels(
            Selection {
                x: 95.0,
                y: 95.0,
                width: 50.0,
                height: 50.0,
            },
            1.0,
            100,
            100,
        )
        .unwrap();
        assert_eq!(crop.width, 5);
    }

    #[test]
    fn invalid_selection_is_rejected() {
        for selection in [
            Selection {
                x: f64::NAN,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            Selection {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 1.0,
            },
        ] {
            assert!(selection_pixels(selection, 1.5, 100, 100).is_err());
        }
        assert!(
            selection_pixels(
                Selection {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0
                },
                0.0,
                100,
                100
            )
            .is_err()
        );
    }

    #[test]
    fn filenames_are_safe_and_stable() {
        assert_eq!(capture_filename(123456, 2), "Oxbox-123456-2.png");
    }
}

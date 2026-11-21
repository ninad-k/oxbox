//! Safe batch image resizing and conversion.

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, imageops::FilterType};
use serde::{Deserialize, Serialize};

use crate::jobs::{CancelToken, Progress};
use crate::{CoreError, CoreResult};

const MAX_PIXELS: u64 = 40_000_000;
const MAX_DIMENSION: u32 = 32_768;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ResizeMode {
    Percent { percent: f64 },
    Fit { width: u32, height: u32 },
    Exact { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Jpeg,
    Webp,
}

impl OutputFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }
    fn image_format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Webp => ImageFormat::WebP,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeRequest {
    pub paths: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub mode: ResizeMode,
    pub format: OutputFormat,
    pub jpeg_quality: u8,
}

fn validate_dimensions(width: u32, height: u32) -> CoreResult<(u32, u32)> {
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err(CoreError::InvalidInput(
            "image dimensions must be positive and at most 40 million pixels".into(),
        ));
    }
    Ok((width, height))
}

pub fn dimensions(width: u32, height: u32, mode: &ResizeMode) -> CoreResult<(u32, u32)> {
    validate_dimensions(width, height)?;
    let (new_width, new_height) = match *mode {
        ResizeMode::Percent { percent } => {
            if !percent.is_finite() || percent <= 0.0 || percent > 10_000.0 {
                return Err(CoreError::InvalidInput(
                    "percentage must be between 0 and 10,000".into(),
                ));
            }
            (
                (f64::from(width) * percent / 100.0).round().max(1.0) as u32,
                (f64::from(height) * percent / 100.0).round().max(1.0) as u32,
            )
        }
        ResizeMode::Fit {
            width: max_width,
            height: max_height,
        } => {
            validate_dimensions(max_width, max_height)?;
            let ratio = (f64::from(max_width) / f64::from(width))
                .min(f64::from(max_height) / f64::from(height));
            (
                (f64::from(width) * ratio).round().max(1.0) as u32,
                (f64::from(height) * ratio).round().max(1.0) as u32,
            )
        }
        ResizeMode::Exact { width, height } => (width, height),
    };
    validate_dimensions(new_width, new_height)
}

fn image_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("image processing failed: {error}"))
}

pub(crate) fn decode_oriented(path: &Path) -> CoreResult<DynamicImage> {
    let mut reader = ImageReader::new(BufReader::new(File::open(path)?)).with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(192 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(image_error)?;
    let (width, height) = decoder.dimensions();
    validate_dimensions(width, height)?;
    let orientation = decoder.orientation().map_err(image_error)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(image_error)?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// Completed outputs are kept if a later file fails; a partial current output is removed.
pub fn resize_batch(
    request: &ResizeRequest,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(Progress),
) -> CoreResult<Vec<PathBuf>> {
    if request.paths.is_empty()
        || request.paths.len() > 1000
        || !(1..=100).contains(&request.jpeg_quality)
    {
        return Err(CoreError::InvalidInput(
            "choose 1–1000 images and a JPEG quality between 1 and 100".into(),
        ));
    }
    cancel.check()?;
    let output_dir = request.output_dir.canonicalize()?;
    if !output_dir.is_dir() {
        return Err(CoreError::InvalidInput(
            "choose an existing output folder".into(),
        ));
    }
    // Preflight destinations so predictable conflicts never cause a partial batch.
    let mut destinations = std::collections::HashSet::new();
    let mut pairs = Vec::new();
    for input in &request.paths {
        let name = input
            .file_stem()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| CoreError::InvalidInput("the image has no filename".into()))?;
        let mut output_name = name.to_os_string();
        output_name.push(format!("-resized.{}", request.format.extension()));
        crate::fs_safe::validate_output_filename(&output_name)?;
        let output = output_dir.join(output_name);
        if output.exists()
            || fs::symlink_metadata(&output).is_ok()
            || !destinations.insert(output.clone())
        {
            return Err(CoreError::InvalidInput(format!(
                "an output already exists or is duplicated: {}",
                output.display()
            )));
        }
        pairs.push((input, output));
    }
    let mut outputs = Vec::new();
    for (index, (input, output)) in pairs.iter().enumerate() {
        cancel.check()?;
        progress(Progress {
            done: index as u64,
            total: pairs.len() as u64,
            message: format!("Resizing {}", input.display()),
        });
        let image = decode_oriented(input)?;
        let (width, height) = dimensions(image.width(), image.height(), &request.mode)?;
        cancel.check()?;
        let resized = image.resize_exact(width, height, FilterType::Lanczos3);
        cancel.check()?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        let result = (|| {
            let mut writer = BufWriter::new(file);
            if matches!(request.format, OutputFormat::Jpeg) {
                image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut writer,
                    request.jpeg_quality,
                )
                .encode_image(&DynamicImage::ImageRgb8(resized.to_rgb8()))
                .map_err(image_error)?;
            } else {
                resized
                    .write_to(&mut writer, request.format.image_format())
                    .map_err(image_error)?;
            }
            writer.flush()?;
            cancel.check()
        })();
        if result.is_err() {
            let _ = fs::remove_file(output);
        }
        result?;
        outputs.push(output.clone());
    }
    progress(Progress {
        done: pairs.len() as u64,
        total: pairs.len() as u64,
        message: "Images saved".into(),
    });
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_cover_all_modes() {
        assert_eq!(
            dimensions(200, 100, &ResizeMode::Percent { percent: 50.0 }).unwrap(),
            (100, 50)
        );
        assert_eq!(
            dimensions(
                200,
                100,
                &ResizeMode::Fit {
                    width: 80,
                    height: 80
                }
            )
            .unwrap(),
            (80, 40)
        );
        assert_eq!(
            dimensions(
                200,
                100,
                &ResizeMode::Exact {
                    width: 80,
                    height: 80
                }
            )
            .unwrap(),
            (80, 80)
        );
        assert!(dimensions(0, 10, &ResizeMode::Percent { percent: 50.0 }).is_err());
        assert!(
            dimensions(
                10,
                10,
                &ResizeMode::Percent {
                    percent: f64::INFINITY
                }
            )
            .is_err()
        );
        assert!(
            dimensions(
                10,
                10,
                &ResizeMode::Exact {
                    width: u32::MAX,
                    height: u32::MAX
                }
            )
            .is_err()
        );
    }

    #[test]
    fn batch_preserves_original_and_refuses_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("original.png");
        image::RgbaImage::new(8, 4).save(&input).unwrap();
        let output = dir.path().join("out");
        std::fs::create_dir(&output).unwrap();
        let request = ResizeRequest {
            paths: vec![input.clone()],
            output_dir: output,
            mode: ResizeMode::Fit {
                width: 4,
                height: 4,
            },
            format: OutputFormat::Png,
            jpeg_quality: 85,
        };
        let paths =
            resize_batch(&request, &crate::jobs::CancelToken::default(), &mut |_| {}).unwrap();
        assert_eq!(image::image_dimensions(&paths[0]).unwrap(), (4, 2));
        assert_eq!(image::image_dimensions(&input).unwrap(), (8, 4));
        assert!(resize_batch(&request, &crate::jobs::CancelToken::default(), &mut |_| {}).is_err());
    }

    #[test]
    fn applies_exif_orientation_before_resizing() {
        use image::ImageEncoder;
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("rotated.jpg");
        let file = File::create(&input).unwrap();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(file);
        // Original TIFF metadata: orientation 6 means a clockwise quarter turn.
        let exif = b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0".to_vec();
        encoder.set_exif_metadata(exif).unwrap();
        encoder.encode_image(&image::RgbImage::new(8, 4)).unwrap();
        assert_eq!(image::image_dimensions(&input).unwrap(), (8, 4));
        let oriented = decode_oriented(&input).unwrap();
        assert_eq!((oriented.width(), oriented.height()), (4, 8));
    }
}

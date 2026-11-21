//! Tool: QR & Barcode. Raster and SVG output share the same module matrix.
use crate::{CoreError, CoreResult};
use image::{GrayImage, Luma};
use qrcode::{Color, EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Cursor, Write},
    path::Path,
};
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodeKind {
    Qr,
    Code128,
    Ean13,
}
pub struct GeneratedCode {
    pub svg: String,
    pub png: Vec<u8>,
}
fn invalid(message: impl ToString) -> CoreError {
    CoreError::InvalidInput(message.to_string())
}
pub fn generate(
    text: &str,
    kind: CodeKind,
    correction: &str,
    size: u32,
) -> CoreResult<GeneratedCode> {
    if text.is_empty() || text.len() > 4096 {
        return Err(invalid("enter between 1 and 4096 bytes"));
    }
    if !(128..=2048).contains(&size) {
        return Err(invalid("image size must be 128–2048 pixels"));
    }
    let ec = match correction {
        "L" => EcLevel::L,
        "M" => EcLevel::M,
        "Q" => EcLevel::Q,
        "H" => EcLevel::H,
        _ => return Err(invalid("invalid QR correction level")),
    };
    let (matrix, width, height, quiet) = match kind {
        CodeKind::Qr => {
            let code = QrCode::with_error_correction_level(text.as_bytes(), ec).map_err(invalid)?;
            let width = code.width();
            (
                code.to_colors()
                    .iter()
                    .map(|c| *c == Color::Dark)
                    .collect::<Vec<_>>(),
                width,
                width,
                4usize,
            )
        }
        CodeKind::Code128 => {
            if !text.bytes().all(|c| (32..=126).contains(&c)) {
                return Err(invalid("Code 128 accepts printable ASCII text"));
            }
            let bits = barcoders::sym::code128::Code128::new(format!("Ɓ{text}"))
                .map_err(invalid)?
                .encode();
            let width = bits.len();
            (bits.into_iter().map(|v| v != 0).collect(), width, 1, 10)
        }
        CodeKind::Ean13 => {
            if text.len() != 13 || !text.bytes().all(|c| c.is_ascii_digit()) {
                return Err(invalid("EAN-13 needs 13 digits including its check digit"));
            }
            let bits = barcoders::sym::ean13::EAN13::new(text)
                .map_err(invalid)?
                .encode();
            let width = bits.len();
            (bits.into_iter().map(|v| v != 0).collect(), width, 1, 11)
        }
    };
    let module = (size as usize / (width + 2 * quiet)).max(1);
    let out_width = (width + 2 * quiet) * module;
    let out_height = if height == 1 {
        (size as usize / 3).max(64)
    } else {
        out_width
    };
    let mut pixels = GrayImage::from_pixel(out_width as u32, out_height as u32, Luma([255]));
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{out_width}\" height=\"{out_height}\" viewBox=\"0 0 {out_width} {out_height}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/><g fill=\"black\">"
    );
    for y in 0..height {
        for x in 0..width {
            if !matrix[y * width + x] {
                continue;
            }
            let px = (x + quiet) * module;
            let py = if height == 1 {
                quiet
            } else {
                (y + quiet) * module
            };
            let block_height = if height == 1 {
                out_height - 2 * quiet
            } else {
                module
            };
            svg.push_str(&format!(
                "<rect x=\"{px}\" y=\"{py}\" width=\"{module}\" height=\"{block_height}\"/>"
            ));
            for iy in py..py + block_height {
                for ix in px..px + module {
                    pixels.put_pixel(ix as u32, iy as u32, Luma([0]));
                }
            }
        }
    }
    svg.push_str("</g></svg>");
    let mut output = Cursor::new(Vec::new());
    pixels
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(invalid)?;
    Ok(GeneratedCode {
        svg,
        png: output.into_inner(),
    })
}
pub fn save(code: &GeneratedCode, path: &Path, svg: bool) -> CoreResult<()> {
    crate::fs_safe::validate_output_filename(
        path.file_name()
            .ok_or_else(|| invalid("choose an output filename"))?,
    )?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(if svg { code.svg.as_bytes() } else { &code.png })?;
    file.flush()?;
    Ok(())
}
pub fn decode_image(code: &GeneratedCode) -> CoreResult<image::RgbaImage> {
    Ok(image::load_from_memory(&code.png)
        .map_err(invalid)?
        .into_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_qr_output_keeps_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("code.svg");
        std::fs::write(&path, b"existing user fixture").unwrap();
        let code = generate("fixture", CodeKind::Qr, "M", 128).unwrap();
        assert!(save(&code, &path, true).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"existing user fixture");
    }
    #[cfg(windows)]
    #[test]
    fn release_qr_output_refuses_ads_and_devices_before_opening() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("existing.bin");
        std::fs::write(&base, b"unchanged fixture").unwrap();
        let code = generate("fixture", CodeKind::Qr, "M", 128).unwrap();
        // ADS goes first so the red stage fails on a disposable file, before any
        // DOS-device case can be reached without the validation guard.
        for name in [
            "existing.bin:code.svg",
            "NUL.svg",
            "CONOUT$.svg",
            "COM¹.svg",
        ] {
            assert_eq!(
                save(&code, &dir.path().join(name), true)
                    .unwrap_err()
                    .code(),
                "invalid_input",
                "{name}"
            );
        }
        assert_eq!(std::fs::read(base).unwrap(), b"unchanged fixture");
    }
    #[test]
    fn qr_roundtrip() {
        let generated = generate("Oxbox: offline utilities", CodeKind::Qr, "M", 512).unwrap();
        let png = image::load_from_memory(&generated.png).unwrap().to_luma8();
        let mut prepared = rqrr::PreparedImage::prepare(png);
        let grids = prepared.detect_grids();
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].decode().unwrap().1, "Oxbox: offline utilities");
        assert!(generated.svg.starts_with("<svg"));
    }
    #[test]
    fn barcode_validation() {
        assert!(generate("4006381333931", CodeKind::Ean13, "M", 512).is_ok());
        assert!(generate("4006381333932", CodeKind::Ean13, "M", 512).is_err());
        assert!(generate("Hello123", CodeKind::Code128, "M", 512).is_ok());
        assert!(generate("", CodeKind::Qr, "M", 256).is_err());
        assert!(generate("abc", CodeKind::Qr, "bad", 256).is_err());
        assert!(generate("abc", CodeKind::Qr, "M", u32::MAX).is_err());
    }
}

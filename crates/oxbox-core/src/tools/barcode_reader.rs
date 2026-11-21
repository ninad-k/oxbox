//! Offline QR and barcode decoding from bounded image files.
use crate::{CoreError, CoreResult};
use serde::Serialize;
use std::path::Path;
#[derive(Debug, Clone, Serialize)]
pub struct BarcodeResult {
    pub text: String,
    pub format: String,
}
pub fn read(path: &Path) -> CoreResult<Vec<BarcodeResult>> {
    let image = super::resize::decode_oriented(path)?.to_luma8();
    let width = image.width();
    let height = image.height();
    if let Ok(results) =
        rxing::helpers::detect_multiple_in_luma(image.as_raw().clone(), width, height)
    {
        return results
            .into_iter()
            .take(100)
            .map(|result| {
                if result.getText().len() > 32768 {
                    return Err(CoreError::InvalidInput(
                        "a decoded payload exceeds the 32 KiB display limit".into(),
                    ));
                }
                Ok(BarcodeResult {
                    text: result.getText().to_string(),
                    format: format!("{:?}", result.getBarcodeFormat()),
                })
            })
            .collect();
    }
    let mut prepared = rqrr::PreparedImage::prepare(image);
    let results: Vec<_> = prepared
        .detect_grids()
        .into_iter()
        .filter_map(|grid| {
            grid.decode().ok().map(|(_, text)| BarcodeResult {
                text,
                format: "QR_CODE".into(),
            })
        })
        .take(100)
        .collect();
    if results.is_empty() {
        return Err(CoreError::InvalidInput(
            "no readable barcode was found; try a sharper, higher-contrast image".into(),
        ));
    }
    Ok(results)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_generated_ean13_and_code128() {
        for (kind, text) in [
            (crate::tools::qr::CodeKind::Ean13, "4006381333931"),
            (crate::tools::qr::CodeKind::Code128, "Oxbox123"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("barcode.png");
            let generated = crate::tools::qr::generate(text, kind, "M", 1024).unwrap();
            std::fs::write(&path, generated.png).unwrap();
            let results = read(&path).unwrap();
            assert!(results.iter().any(|result| result.text == text), "{text}");
        }
    }
    #[test]
    fn decodes_generated_qr_without_devices_or_network() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("qr.png");
        let generated =
            crate::tools::qr::generate("Oxbox reader", crate::tools::qr::CodeKind::Qr, "M", 512)
                .unwrap();
        std::fs::write(&image, generated.png).unwrap();
        let results = read(&image).unwrap();
        assert!(results.iter().any(|result| result.text == "Oxbox reader"));
    }
}

//! Screen/image color sampling and standard color representations.
use crate::{CoreError, CoreResult};
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct ColorValue {
    pub hex: String,
    pub rgb: [u8; 3],
    pub hsl: [f64; 3],
    pub cmyk: [f64; 4],
}
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
pub fn color(red: u8, green: u8, blue: u8) -> ColorValue {
    let (r, g, b) = (
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let light = (max + min) / 2.0;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * light - 1.0).abs())
    };
    let key = 1.0 - max;
    let cmyk = if max == 0.0 {
        [0.0, 0.0, 0.0, 100.0]
    } else {
        [
            round((1.0 - r - key) / max * 100.0),
            round((1.0 - g - key) / max * 100.0),
            round((1.0 - b - key) / max * 100.0),
            round(key * 100.0),
        ]
    };
    ColorValue {
        hex: format!("#{red:02X}{green:02X}{blue:02X}"),
        rgb: [red, green, blue],
        hsl: [round(hue), round(saturation * 100.0), round(light * 100.0)],
        cmyk,
    }
}
pub fn sample_image(image: &image::RgbaImage, x: u32, y: u32) -> CoreResult<ColorValue> {
    if x >= image.width() || y >= image.height() {
        return Err(CoreError::InvalidInput(
            "the sample point is outside the image/display".into(),
        ));
    }
    let pixel = image.get_pixel(x, y);
    Ok(color(pixel[0], pixel[1], pixel[2]))
}
pub fn sample_monitor(id: u32, x: u32, y: u32) -> CoreResult<ColorValue> {
    let image = super::screenshot::capture_image(super::screenshot::CaptureTarget {
        kind: super::screenshot::CaptureKind::Monitor,
        id,
    })?;
    sample_image(&image, x, y)
}
#[cfg(target_os = "windows")]
pub fn sample_cursor() -> CoreResult<ColorValue> {
    let mut point = windows_sys::Win32::Foundation::POINT::default();
    // SAFETY: the output pointer references a live initialized POINT.
    if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let monitor = xcap::Monitor::from_point(point.x, point.y)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let x = point.x.saturating_sub(
        monitor
            .x()
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
    );
    let y = point.y.saturating_sub(
        monitor
            .y()
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
    );
    if x < 0 || y < 0 {
        return Err(CoreError::InvalidInput(
            "cursor is outside the selected display".into(),
        ));
    }
    sample_image(
        &monitor
            .capture_image()
            .map_err(|error| CoreError::InvalidInput(error.to_string()))?,
        x as u32,
        y as u32,
    )
}
#[cfg(not(target_os = "windows"))]
pub fn sample_cursor() -> CoreResult<ColorValue> {
    Err(CoreError::Unsupported(
        "this OS cursor sampler (choose a display and pixel coordinates instead)",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_cover_rgb_hsl_and_cmyk_edges() {
        let red = color(255, 0, 0);
        assert_eq!(red.hex, "#FF0000");
        assert_eq!(red.hsl, [0.0, 100.0, 50.0]);
        assert_eq!(color(0, 0, 0).cmyk, [0.0, 0.0, 0.0, 100.0]);
        let image = image::RgbaImage::new(1, 1);
        assert!(sample_image(&image, 1, 0).is_err());
    }
}

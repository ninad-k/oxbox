//! Physical-to-logical geometry for click-through dimming overlays.
use super::window_manager::Rect;
use crate::{CoreError, CoreResult};
use serde::Serialize;
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Cutout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
pub fn cutout(monitor: Rect, window: Rect, scale: f64) -> CoreResult<Option<Cutout>> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(CoreError::InvalidInput("invalid monitor scaling".into()));
    }
    let left = i64::from(monitor.x).max(i64::from(window.x));
    let top = i64::from(monitor.y).max(i64::from(window.y));
    let right = (i64::from(monitor.x) + i64::from(monitor.width))
        .min(i64::from(window.x) + i64::from(window.width));
    let bottom = (i64::from(monitor.y) + i64::from(monitor.height))
        .min(i64::from(window.y) + i64::from(window.height));
    if right <= left || bottom <= top {
        return Ok(None);
    }
    Ok(Some(Cutout {
        x: (left - i64::from(monitor.x)) as f64 / scale,
        y: (top - i64::from(monitor.y)) as f64 / scale,
        width: (right - left) as f64 / scale,
        height: (bottom - top) as f64 / scale,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intersects_and_scales_window_cutout() {
        let result = cutout(
            Rect {
                x: -100,
                y: 0,
                width: 1000,
                height: 800,
            },
            Rect {
                x: 0,
                y: 100,
                width: 300,
                height: 200,
            },
            1.5,
        )
        .unwrap()
        .unwrap();
        assert!((result.x - 100.0 / 1.5).abs() < 1e-8);
        assert_eq!(result.width, 200.0);
        assert!(
            cutout(
                Rect {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100
                },
                Rect {
                    x: 200,
                    y: 0,
                    width: 10,
                    height: 10
                },
                1.0
            )
            .unwrap()
            .is_none()
        );
        assert!(
            cutout(
                Rect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1
                },
                Rect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1
                },
                f64::NAN
            )
            .is_err()
        );
    }
}

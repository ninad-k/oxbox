//! Deterministic half, third and quarter window placement.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Snap {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    LeftThird,
    CenterThird,
    RightThird,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Full,
}
pub fn snap_rect(area: Rect, snap: Snap) -> CoreResult<Rect> {
    if area.width == 0 || area.height == 0 || area.width > 65536 || area.height > 65536 {
        return Err(CoreError::InvalidInput("invalid monitor work area".into()));
    }
    let (col, cols, row, rows) = match snap {
        Snap::LeftHalf => (0, 2, 0, 1),
        Snap::RightHalf => (1, 2, 0, 1),
        Snap::TopHalf => (0, 1, 0, 2),
        Snap::BottomHalf => (0, 1, 1, 2),
        Snap::LeftThird => (0, 3, 0, 1),
        Snap::CenterThird => (1, 3, 0, 1),
        Snap::RightThird => (2, 3, 0, 1),
        Snap::TopLeft => (0, 2, 0, 2),
        Snap::TopRight => (1, 2, 0, 2),
        Snap::BottomLeft => (0, 2, 1, 2),
        Snap::BottomRight => (1, 2, 1, 2),
        Snap::Full => (0, 1, 0, 1),
    };
    let left = area.width * col / cols;
    let right = area.width * (col + 1) / cols;
    let top = area.height * row / rows;
    let bottom = area.height * (row + 1) / rows;
    let x = i32::try_from(i64::from(area.x) + i64::from(left))
        .map_err(|_| CoreError::InvalidInput("monitor position overflow".into()))?;
    let y = i32::try_from(i64::from(area.y) + i64::from(top))
        .map_err(|_| CoreError::InvalidInput("monitor position overflow".into()))?;
    Ok(Rect {
        x,
        y,
        width: right - left,
        height: bottom - top,
    })
}
pub fn snap(id: u64, pid: u32, placement: Snap) -> CoreResult<Rect> {
    super::window_top::window(id, pid)?;
    let area = crate::platform::desktop::work_area(id)?;
    let target = snap_rect(area, placement)?;
    crate::platform::desktop::move_window(id, target)?;
    Ok(target)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partitions_cover_odd_work_areas_without_gaps() {
        let area = Rect {
            x: -100,
            y: 10,
            width: 1001,
            height: 701,
        };
        let a = snap_rect(area, Snap::LeftThird).unwrap();
        let b = snap_rect(area, Snap::CenterThird).unwrap();
        let c = snap_rect(area, Snap::RightThird).unwrap();
        assert_eq!(a.width + b.width + c.width, 1001);
        assert_eq!(a.x + a.width as i32, b.x);
        assert_eq!(b.x + b.width as i32, c.x);
        assert_eq!(snap_rect(area, Snap::BottomRight).unwrap().y, 360);
        assert!(
            snap_rect(
                Rect {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 1
                },
                Snap::LeftHalf
            )
            .is_err()
        );
    }
}

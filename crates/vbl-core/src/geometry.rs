use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Resolution {
    pub width: i32,
    pub height: i32,
}

impl Resolution {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct NormalizedPoint {
    pub nx: f64,
    pub ny: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PixelPoint {
    pub x: i32,
    pub y: i32,
}

impl NormalizedPoint {
    pub fn new(nx: f64, ny: f64) -> Self {
        Self { nx, ny }
    }

    pub fn to_pixel(&self, res: Resolution) -> PixelPoint {
        PixelPoint {
            x: (self.nx * res.width as f64).round() as i32,
            y: (self.ny * res.height as f64).round() as i32,
        }
    }
}

impl PixelPoint {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn to_normalized(&self, res: Resolution) -> NormalizedPoint {
        NormalizedPoint {
            nx: self.x as f64 / res.width as f64,
            ny: self.y as f64 / res.height as f64,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Region {
    pub w: u32,
    pub h: u32,
}

impl Default for Region {
    fn default() -> Self {
        Self::single()
    }
}

impl Region {
    pub fn single() -> Self {
        Self { w: 1, h: 1 }
    }

    pub fn new(w: u32, h: u32) -> Self {
        Self { w, h }
    }

    pub fn points_around(self, center: PixelPoint) -> Vec<PixelPoint> {
        let w = self.w.max(1) as i32;
        let h = self.h.max(1) as i32;
        let x0 = center.x - (w - 1) / 2;
        let y0 = center.y - (h - 1) / 2;
        let mut points = Vec::with_capacity((w * h) as usize);
        for dy in 0..h {
            for dx in 0..w {
                points.push(PixelPoint::new(x0 + dx, y0 + dy));
            }
        }
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_exact_at_same_resolution() {
        let res = Resolution::new(1920, 1080);
        for p in [
            PixelPoint::new(942, 1003),
            PixelPoint::new(1030, 903),
            PixelPoint::new(0, 0),
            PixelPoint::new(1919, 1079),
        ] {
            assert_eq!(p.to_normalized(res).to_pixel(res), p);
        }
    }

    #[test]
    fn normalized_point_scales_across_resolutions() {
        let center = NormalizedPoint::new(0.5, 0.5);
        assert_eq!(
            center.to_pixel(Resolution::new(1920, 1080)),
            PixelPoint::new(960, 540)
        );
        assert_eq!(
            center.to_pixel(Resolution::new(1600, 900)),
            PixelPoint::new(800, 450)
        );
    }

    #[test]
    fn region_1x1_is_just_the_center() {
        assert_eq!(
            Region::single().points_around(PixelPoint::new(5, 7)),
            vec![PixelPoint::new(5, 7)]
        );
    }

    #[test]
    fn region_3x3_is_centered() {
        let pts = Region::new(3, 3).points_around(PixelPoint::new(10, 10));
        assert_eq!(pts.len(), 9);
        assert_eq!(pts[0], PixelPoint::new(9, 9));
        assert_eq!(pts[4], PixelPoint::new(10, 10)); // center
        assert_eq!(pts[8], PixelPoint::new(11, 11));
    }
}

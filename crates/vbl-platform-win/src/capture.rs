use vbl_core::color::Rgb;
use vbl_core::geometry::Resolution;
use vbl_core::traits::{CapturePoint, PixelSample, ScreenCapture};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

fn colorref_to_rgb(c: u32) -> Rgb {
    Rgb::new(
        (c & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        ((c >> 16) & 0xFF) as u8,
    )
}

pub fn cursor_pixel() -> Option<(i32, i32, Rgb)> {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt).is_err() {
            return None;
        }
        let hdc = GetDC(HWND::default());
        let color = GetPixel(hdc, pt.x, pt.y);
        ReleaseDC(HWND::default(), hdc);
        if color.0 == 0xFFFF_FFFF {
            return None;
        }
        Some((pt.x, pt.y, colorref_to_rgb(color.0)))
    }
}

#[derive(Default)]
pub struct GdiCapture;

impl GdiCapture {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapture for GdiCapture {
    fn sample(&self, points: &[CapturePoint]) -> Vec<PixelSample> {
        let hdc = unsafe { GetDC(HWND::default()) };
        let mut out = Vec::with_capacity(points.len());
        for p in points {
            let pixels = p.region.points_around(p.point);
            let samples: Vec<Rgb> = pixels
                .iter()
                .map(|pt| {
                    let color = unsafe { GetPixel(hdc, pt.x, pt.y) };
                    if color.0 == 0xFFFF_FFFF {
                        Rgb::new(0, 0, 0)
                    } else {
                        colorref_to_rgb(color.0)
                    }
                })
                .collect();
            let matched = p.tolerance.matches_region(&samples, p.target);
            out.push(PixelSample {
                key: p.key,
                point: p.point,
                rgb: samples[samples.len() / 2],
                matched,
            });
        }
        unsafe {
            ReleaseDC(HWND::default(), hdc);
        }
        out
    }

    fn current_resolution(&self) -> Resolution {
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        Resolution::new(w, h)
    }
}

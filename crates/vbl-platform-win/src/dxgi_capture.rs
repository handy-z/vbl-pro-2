use std::sync::Mutex;

use vbl_core::color::{Rgb, Tolerance};
use vbl_core::geometry::{PixelPoint, Region, Resolution};
use vbl_core::state::StateKey;
use vbl_core::traits::{CapturePoint, PixelSample, ScreenCapture};
use windows::core::{Error, Interface, Result as WinResult};
use windows::Win32::Foundation::{E_FAIL, HMODULE};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{
    IDXGIAdapter, IDXGIDevice, IDXGIOutput, IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource,
    DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
};

use crate::capture::GdiCapture;

const ACQUIRE_TIMEOUT_MS: u32 = 12;

const VALIDATE_TOLERANCE: u8 = 8;

pub struct DxgiCapture {
    gdi: GdiCapture,
    inner: Mutex<Option<DxgiInner>>,
}

unsafe impl Send for DxgiCapture {}
unsafe impl Sync for DxgiCapture {}

impl DxgiCapture {
    pub fn new() -> Self {
        let inner = unsafe { DxgiInner::init() }.ok();
        let capture = Self {
            gdi: GdiCapture::new(),
            inner: Mutex::new(inner),
        };
        if !capture.validate() {
            if let Ok(mut guard) = capture.inner.lock() {
                *guard = None;
            }
        }
        capture
    }

    pub fn backend(&self) -> &'static str {
        match self.inner.lock() {
            Ok(g) if g.is_some() => "DXGI",
            _ => "GDI",
        }
    }

    fn validate(&self) -> bool {
        let probes = {
            let mut guard = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let Some(inner) = guard.as_mut() else {
                return false;
            };
            unsafe { inner.warmup() };
            let (w, h) = (inner.width as i32, inner.height as i32);
            let pts: Vec<CapturePoint> = [(w / 2, h / 2), (w / 4, h / 4), (3 * w / 4, 3 * h / 4)]
                .into_iter()
                .map(|(x, y)| probe_point(inner.left + x, inner.top + y))
                .collect();
            match unsafe { inner.sample(&pts) } {
                Some(dxgi) => pts.into_iter().zip(dxgi).collect::<Vec<_>>(),
                None => return false,
            }
        };

        probes.iter().all(|(point, dxgi)| {
            let gdi = self.gdi.sample(std::slice::from_ref(point));
            gdi.first().is_some_and(|g| close(g.rgb, dxgi.rgb))
        })
    }
}

impl Default for DxgiCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapture for DxgiCapture {
    fn sample(&self, points: &[CapturePoint]) -> Vec<PixelSample> {
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(inner) = guard.as_mut() {
                if let Some(samples) = unsafe { inner.sample(points) } {
                    return samples;
                }
            }
        }
        self.gdi.sample(points)
    }

    fn current_resolution(&self) -> Resolution {
        if let Ok(guard) = self.inner.lock() {
            if let Some(inner) = guard.as_ref() {
                return Resolution::new(inner.width as i32, inner.height as i32);
            }
        }
        self.gdi.current_resolution()
    }
}

struct DxgiInner {
    context: ID3D11DeviceContext,
    dupl: IDXGIOutputDuplication,
    staging: ID3D11Texture2D,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    frame: Vec<u8>,
    pitch: usize,
    have_frame: bool,
}

impl DxgiInner {
    unsafe fn init() -> WinResult<DxgiInner> {
        let (device, context) = create_device()?;
        let dxgi_device: IDXGIDevice = device.cast()?;
        let adapter: IDXGIAdapter = dxgi_device.GetAdapter()?;
        let output: IDXGIOutput = adapter.EnumOutputs(0)?;

        let desc = output.GetDesc()?;
        let rect = desc.DesktopCoordinates;
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;

        let output1: IDXGIOutput1 = output.cast()?;
        let dupl = output1.DuplicateOutput(&device)?;

        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            ..Default::default()
        };
        let mut staging: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&staging_desc, None, Some(&mut staging))?;
        let staging = staging.ok_or_else(|| Error::from(E_FAIL))?;

        Ok(DxgiInner {
            context,
            dupl,
            staging,
            width,
            height,
            left: rect.left,
            top: rect.top,
            frame: Vec::new(),
            pitch: 0,
            have_frame: false,
        })
    }

    unsafe fn sample(&mut self, points: &[CapturePoint]) -> Option<Vec<PixelSample>> {
        self.refresh().ok()?;
        if !self.have_frame {
            return None;
        }
        let mut out = Vec::with_capacity(points.len());
        for p in points {
            let pixels = p.region.points_around(p.point);
            let mut samples = Vec::with_capacity(pixels.len());
            for pt in &pixels {
                samples.push(self.pixel(pt.x - self.left, pt.y - self.top)?);
            }
            out.push(PixelSample {
                key: p.key,
                point: p.point,
                rgb: samples[samples.len() / 2],
                matched: p.tolerance.matches_region(&samples, p.target),
            });
        }
        Some(out)
    }

    unsafe fn warmup(&mut self) {
        for _ in 0..40 {
            if self.refresh().is_ok() && self.have_frame {
                return;
            }
        }
    }

    unsafe fn refresh(&mut self) -> WinResult<()> {
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<IDXGIResource> = None;
        match self
            .dupl
            .AcquireNextFrame(ACQUIRE_TIMEOUT_MS, &mut info, &mut resource)
        {
            Ok(()) => {}

            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => {
                return if self.have_frame { Ok(()) } else { Err(e) };
            }
            Err(e) => return Err(e),
        }

        let result = self.copy_acquired(resource.as_ref());
        let _ = self.dupl.ReleaseFrame();
        result
    }

    unsafe fn copy_acquired(&mut self, resource: Option<&IDXGIResource>) -> WinResult<()> {
        let frame_tex: ID3D11Texture2D = resource.ok_or_else(|| Error::from(E_FAIL))?.cast()?;
        self.context.CopyResource(&self.staging, &frame_tex);

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        self.context
            .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

        let pitch = mapped.RowPitch as usize;
        let size = pitch * self.height as usize;
        if self.frame.len() != size {
            self.frame.resize(size, 0);
        }
        std::ptr::copy_nonoverlapping(mapped.pData as *const u8, self.frame.as_mut_ptr(), size);
        self.context.Unmap(&self.staging, 0);

        self.pitch = pitch;
        self.have_frame = true;
        Ok(())
    }

    fn pixel(&self, x: i32, y: i32) -> Option<Rgb> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        let idx = y as usize * self.pitch + x as usize * 4;
        let b = *self.frame.get(idx)?;
        let g = *self.frame.get(idx + 1)?;
        let r = *self.frame.get(idx + 2)?;
        Some(Rgb::new(r, g, b))
    }
}

unsafe fn create_device() -> WinResult<(ID3D11Device, ID3D11DeviceContext)> {
    for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        let ok = D3D11CreateDevice(
            None,
            driver,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        );
        if ok.is_ok() {
            if let (Some(device), Some(context)) = (device, context) {
                return Ok((device, context));
            }
        }
    }
    Err(Error::from(E_FAIL))
}

fn probe_point(x: i32, y: i32) -> CapturePoint {
    CapturePoint {
        key: StateKey::GameOnGround,
        point: PixelPoint::new(x, y),
        target: Rgb::new(0, 0, 0),
        tolerance: Tolerance::exact(),
        region: Region::single(),
    }
}

fn close(a: Rgb, b: Rgb) -> bool {
    let d = |x: u8, y: u8| x.abs_diff(y) <= VALIDATE_TOLERANCE;
    d(a.r, b.r) && d(a.g, b.g) && d(a.b, b.b)
}

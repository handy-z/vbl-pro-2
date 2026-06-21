use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use vbl_core::color::Rgb;
use vbl_core::traits::Overlay;
use windows::core::{Error, Result as WinResult, PCWSTR};
use windows::Win32::Foundation::{
    COLORREF, E_FAIL, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, DIB_RGB_COLORS, HDC, HGDIOBJ, RGBQUAD,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW,
    PostThreadMessageW, RegisterClassExW, ShowWindow, TranslateMessage, UpdateLayeredWindow, MSG,
    PM_REMOVE, SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_QUIT, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const CANVAS: i32 = 128;

#[derive(Clone, Copy)]
struct Shared {
    visible: bool,
    cx: i32,
    cy: i32,
    r: u8,
    g: u8,
    b: u8,
    opacity: u8,
    scale: f64,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    visible: false,
    cx: 0,
    cy: 0,
    r: 255,
    g: 255,
    b: 255,
    opacity: 255,
    scale: 1.0,
});
static THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static THREAD_ID: AtomicU32 = AtomicU32::new(0);
static RUNNING: AtomicBool = AtomicBool::new(false);

pub struct WinOverlay;

impl WinOverlay {
    pub fn start() -> Self {
        if !RUNNING.swap(true, Ordering::SeqCst) {
            let handle = thread::Builder::new()
                .name("vbl-overlay".into())
                .spawn(overlay_thread)
                .expect("spawn overlay thread");
            if let Ok(mut guard) = THREAD.lock() {
                *guard = Some(handle);
            }
        }
        WinOverlay
    }

    pub fn set(&self, visible: bool, cx: i32, cy: i32, color: Rgb, opacity: f64, scale: f64) {
        if let Ok(mut s) = SHARED.lock() {
            s.visible = visible;
            s.cx = cx;
            s.cy = cy;
            s.r = color.r;
            s.g = color.g;
            s.b = color.b;
            s.opacity = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
            s.scale = scale.max(0.1);
        }
    }

    pub fn stop(&self) {
        if !RUNNING.swap(false, Ordering::SeqCst) {
            return;
        }
        let mut tid = THREAD_ID.load(Ordering::SeqCst);
        for _ in 0..50 {
            if tid != 0 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
            tid = THREAD_ID.load(Ordering::SeqCst);
        }
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        let handle = THREAD.lock().ok().and_then(|mut g| g.take());
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

impl Overlay for WinOverlay {
    fn show(&self) {
        if let Ok(mut s) = SHARED.lock() {
            s.visible = true;
        }
    }
    fn update(&self) {}
    fn hide(&self) {
        if let Ok(mut s) = SHARED.lock() {
            s.visible = false;
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}

fn overlay_thread() {
    unsafe {
        THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);

        let hinstance: HINSTANCE = GetModuleHandleW(None).map(|m| m.into()).unwrap_or_default();
        let class_name: Vec<u16> = "VblProOverlayWindow\0".encode_utf16().collect();

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let ex_style =
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;

        let hwnd = match CreateWindowExW(
            ex_style,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            0,
            0,
            CANVAS,
            CANVAS,
            None,
            None,
            hinstance,
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(_) => {
                THREAD_ID.store(0, Ordering::SeqCst);
                return;
            }
        };

        let screen = GetDC(HWND::default());
        let memdc = CreateCompatibleDC(screen);

        let mut bits: *mut c_void = null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: CANVAS,
                biHeight: -CANVAS,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default(); 1],
        };
        let dib = match CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(dib) => dib,
            Err(_) => {
                let _ = DeleteDC(memdc);
                ReleaseDC(HWND::default(), screen);
                let _ = DestroyWindow(hwnd);
                THREAD_ID.store(0, Ordering::SeqCst);
                return;
            }
        };
        let old = SelectObject(memdc, HGDIOBJ(dib.0));
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        let d2d = create_dc_target().ok();
        let target = d2d.as_ref().map(|(_, t)| t);

        let mut msg = MSG::default();
        'pump: loop {
            while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break 'pump;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            render(hwnd, screen, memdc, bits as *mut u32, target);
            thread::sleep(Duration::from_millis(16));
        }

        SelectObject(memdc, old);
        let _ = DeleteObject(dib);
        let _ = DeleteDC(memdc);
        ReleaseDC(HWND::default(), screen);
        let _ = DestroyWindow(hwnd);
        THREAD_ID.store(0, Ordering::SeqCst);
    }
}

unsafe fn render(
    hwnd: HWND,
    screen: HDC,
    memdc: HDC,
    bits: *mut u32,
    d2d: Option<&ID2D1DCRenderTarget>,
) {
    let s = match SHARED.lock() {
        Ok(g) => *g,
        Err(_) => return,
    };
    if !s.visible {
        let _ = ShowWindow(hwnd, SW_HIDE);
        return;
    }

    let drew_d2d = match d2d {
        Some(rt) => draw_crosshair_d2d(rt, memdc, bits, &s).is_ok(),
        None => false,
    };
    if !drew_d2d {
        draw_crosshair(bits, s.r, s.g, s.b, s.scale);
    }

    let size = SIZE {
        cx: CANVAS,
        cy: CANVAS,
    };
    let src = POINT { x: 0, y: 0 };
    let dst = POINT {
        x: s.cx - CANVAS / 2,
        y: s.cy - CANVAS / 2,
    };

    let blend = BLENDFUNCTION {
        BlendOp: 0,
        BlendFlags: 0,
        SourceConstantAlpha: s.opacity,
        AlphaFormat: 1,
    };

    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    let _ = UpdateLayeredWindow(
        hwnd,
        screen,
        Some(&dst),
        Some(&size),
        memdc,
        Some(&src),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );
}

unsafe fn create_dc_target() -> WinResult<(ID2D1Factory, ID2D1DCRenderTarget)> {
    let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
    let props = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        usage: D2D1_RENDER_TARGET_USAGE_GDI_COMPATIBLE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    };
    let target = factory.CreateDCRenderTarget(&props)?;
    Ok((factory, target))
}

unsafe fn draw_crosshair_d2d(
    rt: &ID2D1DCRenderTarget,
    memdc: HDC,
    bits: *mut u32,
    s: &Shared,
) -> WinResult<()> {
    let rect = RECT {
        left: 0,
        top: 0,
        right: CANVAS,
        bottom: CANVAS,
    };
    rt.BindDC(memdc, &rect)?;
    rt.BeginDraw();
    rt.Clear(Some(&D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }));

    let color = D2D1_COLOR_F {
        r: s.r as f32 / 255.0,
        g: s.g as f32 / 255.0,
        b: s.b as f32 / 255.0,
        a: 1.0,
    };
    let brush = rt.CreateSolidColorBrush(&color, None)?;

    let c = CANVAS as f32 / 2.0;
    let scale = s.scale as f32;
    let gap = 4.0 * scale;
    let len = 18.0 * scale;
    let half = (1.0 * scale).max(0.5);

    let arms = [
        D2D_RECT_F {
            left: c - half,
            top: c - gap - len,
            right: c + half,
            bottom: c - gap,
        },
        D2D_RECT_F {
            left: c - half,
            top: c + gap,
            right: c + half,
            bottom: c + gap + len,
        },
        D2D_RECT_F {
            left: c - gap - len,
            top: c - half,
            right: c - gap,
            bottom: c + half,
        },
        D2D_RECT_F {
            left: c + gap,
            top: c - half,
            right: c + gap + len,
            bottom: c + half,
        },
    ];
    for arm in &arms {
        rt.FillRectangle(arm, &brush);
    }
    rt.FillEllipse(
        &D2D1_ELLIPSE {
            point: D2D_POINT_2F { x: c, y: c },
            radiusX: half,
            radiusY: half,
        },
        &brush,
    );

    rt.EndDraw(None, None)?;

    if any_visible(bits) {
        Ok(())
    } else {
        Err(Error::from(E_FAIL))
    }
}

unsafe fn any_visible(bits: *mut u32) -> bool {
    (0..(CANVAS * CANVAS) as usize).any(|i| (*bits.add(i)) & 0xFF00_0000 != 0)
}

unsafe fn draw_crosshair(bits: *mut u32, r: u8, g: u8, b: u8, scale: f64) {
    let color = 0xFF00_0000u32 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    for i in 0..(CANVAS * CANVAS) as usize {
        *bits.add(i) = 0;
    }
    let c = CANVAS / 2;
    let gap = ((4.0 * scale) as i32).max(1);
    let len = ((18.0 * scale) as i32).max(2);
    let th = ((1.0 * scale) as i32).max(0);

    let near = (c - gap - len)..(c - gap);
    let far = (c + gap + 1)..=(c + gap + len);

    for y in 0..CANVAS {
        if near.contains(&y) || far.contains(&y) {
            for tx in -th..=th {
                set_px(bits, c + tx, y, color);
            }
        }
    }
    for x in 0..CANVAS {
        if near.contains(&x) || far.contains(&x) {
            for ty in -th..=th {
                set_px(bits, x, c + ty, color);
            }
        }
    }
}

unsafe fn set_px(bits: *mut u32, x: i32, y: i32, color: u32) {
    if (0..CANVAS).contains(&x) && (0..CANVAS).contains(&y) {
        *bits.add((y * CANVAS + x) as usize) = color;
    }
}

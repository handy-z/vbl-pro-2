use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::AppState;

pub const TRAY_ID: &str = "vbl-tray";
const SIZE: u32 = 32;

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show VBL Pro 2", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Toggle armed", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &toggle, &sep, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(status_icon(false, false))
        .tooltip("VBL Pro 2 — idle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "toggle" => {
                if let Some(state) = app.try_state::<AppState>() {
                    crate::with_runtime(&state, |r| r.toggle_armed());
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn update<R: Runtime>(app: &AppHandle<R>, armed: bool, focused: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(status_icon(armed, focused)));
        let tip = match (armed, focused) {
            (true, true) => "VBL Pro 2 — armed (active)",
            (true, false) => "VBL Pro 2 — armed (waiting for Roblox focus)",
            (false, _) => "VBL Pro 2 — idle",
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}

fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn status_icon(armed: bool, focused: bool) -> Image<'static> {
    let (r, g, b) = match (armed, focused) {
        (true, true) => (63u8, 185u8, 80u8),
        (true, false) => (210, 153, 34),
        (false, _) => (110, 118, 129),
    };
    let center = (SIZE as f64 - 1.0) / 2.0;
    let radius = 13.0;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let coverage = (radius - dist + 0.5).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let i = ((y * SIZE + x) * 4) as usize;
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = (coverage * 255.0) as u8;
            }
        }
    }
    Image::new_owned(rgba, SIZE, SIZE)
}

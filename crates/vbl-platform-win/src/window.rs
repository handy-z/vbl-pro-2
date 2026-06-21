use vbl_core::traits::{ClientRect, TargetWindow, WindowTracker};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetForegroundWindow, GetWindowThreadProcessId,
};

pub struct WinWindowTracker {
    target_processes: Vec<String>,
}

impl WinWindowTracker {
    pub fn new(target_processes: Vec<String>) -> Self {
        Self { target_processes }
    }

    pub fn for_roblox() -> Self {
        Self::new(vec!["RobloxPlayerBeta.exe".to_string()])
    }

    fn foreground_target(&self) -> Option<HWND> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return None;
        }
        let name = process_name_for_window(hwnd)?;
        self.target_processes
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&name))
            .then_some(hwnd)
    }
}

impl WindowTracker for WinWindowTracker {
    fn target_window(&self) -> Option<TargetWindow> {
        let hwnd = self.foreground_target()?;
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect).ok()? };
        let mut origin = POINT { x: 0, y: 0 };
        unsafe {
            let _ = ClientToScreen(hwnd, &mut origin);
        }
        Some(TargetWindow {
            client: ClientRect {
                x: origin.x,
                y: origin.y,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            },
        })
    }

    fn is_target_focused(&self) -> bool {
        self.foreground_target().is_some()
    }
}

fn process_name_for_window(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
    }
    if pid == 0 {
        return None;
    }

    let handle: HANDLE =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };

    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = CloseHandle(handle);
    }
    result.ok()?;

    let full = String::from_utf16_lossy(&buf[..len as usize]);
    let name = full.rsplit(['\\', '/']).next().unwrap_or(&full).to_string();
    Some(name)
}

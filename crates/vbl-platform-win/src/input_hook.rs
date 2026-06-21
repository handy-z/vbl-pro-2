use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use vbl_core::input::MouseButton;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_XBUTTONDOWN, WM_XBUTTONUP,
};

#[derive(Clone, Copy, Debug)]
pub enum RawInputEvent {
    Key { vk: u16, down: bool },
    MouseButton { button: MouseButton, down: bool },
}

type Callback = Box<dyn Fn(RawInputEvent) + Send>;

static CALLBACK: Mutex<Option<Callback>> = Mutex::new(None);
static HOOK_THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start_hook(callback: Callback) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(mut guard) = CALLBACK.lock() {
        *guard = Some(callback);
    }

    let handle = thread::Builder::new()
        .name("vbl-input-hook".into())
        .spawn(|| unsafe {
            HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);
            let kb =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), HINSTANCE::default(), 0);
            let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            if let Ok(h) = kb {
                let _ = UnhookWindowsHookEx(h);
            }
            if let Ok(h) = ms {
                let _ = UnhookWindowsHookEx(h);
            }
            HOOK_THREAD_ID.store(0, Ordering::SeqCst);
        })
        .expect("spawn input-hook thread");

    if let Ok(mut guard) = HOOK_THREAD.lock() {
        *guard = Some(handle);
    }
}

pub fn stop_hook() {
    if !RUNNING.swap(false, Ordering::SeqCst) {
        return;
    }

    let mut tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
    for _ in 0..50 {
        if tid != 0 {
            break;
        }
        thread::sleep(Duration::from_millis(2));
        tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
    }
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
    let handle = HOOK_THREAD.lock().ok().and_then(|mut g| g.take());
    if let Some(handle) = handle {
        let _ = handle.join();
    }
    if let Ok(mut guard) = CALLBACK.lock() {
        *guard = None;
    }
}

fn dispatch(event: RawInputEvent) {
    if let Ok(guard) = CALLBACK.lock() {
        if let Some(cb) = guard.as_ref() {
            cb(event);
        }
    }
}

unsafe extern "system" fn keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let msg = wparam.0 as u32;
        let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
        if down || up {
            let data = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            dispatch(RawInputEvent::Key {
                vk: data.vkCode as u16,
                down,
            });
        }
    }
    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_XBUTTONDOWN || msg == WM_XBUTTONUP {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let xbutton = ((data.mouseData >> 16) & 0xffff) as u16;
            let button = match xbutton {
                1 => Some(MouseButton::X1),
                2 => Some(MouseButton::X2),
                _ => None,
            };
            if let Some(button) = button {
                dispatch(RawInputEvent::MouseButton {
                    button,
                    down: msg == WM_XBUTTONDOWN,
                });
            }
        }
    }
    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

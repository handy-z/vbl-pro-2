#![cfg(windows)]

mod capture;
mod clock;
mod dxgi_capture;
mod input_hook;
mod input_sink;
mod keymap;
mod overlay;
mod window;

pub use capture::{cursor_pixel, GdiCapture};
pub use clock::WinClock;
pub use dxgi_capture::DxgiCapture;
pub use input_hook::{start_hook, stop_hook, RawInputEvent};
pub use input_sink::WinInputSink;
pub use keymap::vk_to_key;
pub use overlay::WinOverlay;
pub use window::WinWindowTracker;

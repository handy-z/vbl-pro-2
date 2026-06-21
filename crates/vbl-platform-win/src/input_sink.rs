use std::collections::HashSet;
use std::sync::Mutex;

use vbl_core::input::{Key, KeyAction, MouseButton};
use vbl_core::traits::InputSink;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

use crate::keymap::{key_to_vk, send_key, send_mouse};

#[derive(Default)]
pub struct WinInputSink {
    keys_down: Mutex<HashSet<u16>>,
    buttons_down: Mutex<HashSet<MouseButton>>,
}

impl WinInputSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl InputSink for WinInputSink {
    fn key(&self, key: &Key, action: KeyAction) {
        let Some(vk) = key_to_vk(key.as_str()) else {
            return;
        };
        let down = action == KeyAction::Press;
        send_key(vk, down);
        if let Ok(mut set) = self.keys_down.lock() {
            if down {
                set.insert(vk.0);
            } else {
                set.remove(&vk.0);
            }
        }
    }

    fn mouse_button(&self, button: MouseButton, action: KeyAction) {
        let down = action == KeyAction::Press;
        send_mouse(button, down);
        if let Ok(mut set) = self.buttons_down.lock() {
            if down {
                set.insert(button);
            } else {
                set.remove(&button);
            }
        }
    }

    fn release_all(&self) {
        if let Ok(mut set) = self.keys_down.lock() {
            for vk in set.drain() {
                send_key(VIRTUAL_KEY(vk), false);
            }
        }
        if let Ok(mut set) = self.buttons_down.lock() {
            for button in set.drain() {
                send_mouse(button, false);
            }
        }
    }
}

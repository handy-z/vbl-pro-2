use vbl_core::input::{Key, MouseButton};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MAPVK_VK_TO_VSC_EX,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    MOUSE_EVENT_FLAGS, VIRTUAL_KEY, VK_ADD, VK_APPS, VK_BACK, VK_CAPITAL, VK_DECIMAL, VK_DELETE,
    VK_DIVIDE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4,
    VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK, VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2,
    VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7, VK_NUMPAD8, VK_NUMPAD9, VK_PAUSE,
    VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL,
    VK_SNAPSHOT, VK_SPACE, VK_SUBTRACT, VK_TAB, VK_UP,
};

pub(crate) fn key_to_vk(key: &str) -> Option<VIRTUAL_KEY> {
    if key.len() == 1 {
        let b = key.as_bytes()[0];
        if b.is_ascii_alphanumeric() {
            return Some(VIRTUAL_KEY(b.to_ascii_uppercase() as u16));
        }
    }
    let vk = match key {
        "escape" => VK_ESCAPE,
        "backspace" => VK_BACK,
        "tab" => VK_TAB,
        "enter" => VK_RETURN,
        "lctrl" => VK_LCONTROL,
        "rctrl" => VK_RCONTROL,
        "lshift" => VK_LSHIFT,
        "rshift" => VK_RSHIFT,
        "lalt" => VK_LMENU,
        "ralt" => VK_RMENU,
        "lwin" => VK_LWIN,
        "rwin" => VK_RWIN,
        "space" => VK_SPACE,
        "capslock" => VK_CAPITAL,
        "numlock" => VK_NUMLOCK,
        "scrolllock" => VK_SCROLL,
        "insert" => VK_INSERT,
        "delete" => VK_DELETE,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "left" => VK_LEFT,
        "right" => VK_RIGHT,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "f1" => VK_F1,
        "f2" => VK_F2,
        "f3" => VK_F3,
        "f4" => VK_F4,
        "f5" => VK_F5,
        "f6" => VK_F6,
        "f7" => VK_F7,
        "f8" => VK_F8,
        "f9" => VK_F9,
        "f10" => VK_F10,
        "f11" => VK_F11,
        "f12" => VK_F12,
        "num0" => VK_NUMPAD0,
        "num1" => VK_NUMPAD1,
        "num2" => VK_NUMPAD2,
        "num3" => VK_NUMPAD3,
        "num4" => VK_NUMPAD4,
        "num5" => VK_NUMPAD5,
        "num6" => VK_NUMPAD6,
        "num7" => VK_NUMPAD7,
        "num8" => VK_NUMPAD8,
        "num9" => VK_NUMPAD9,
        "numadd" => VK_ADD,
        "numsub" => VK_SUBTRACT,
        "nummul" => VK_MULTIPLY,
        "numdiv" => VK_DIVIDE,
        "numdecimal" => VK_DECIMAL,
        "pause" => VK_PAUSE,
        "apps" => VK_APPS,
        "printscreen" => VK_SNAPSHOT,
        "-" => VIRTUAL_KEY(0xBD),
        "=" => VIRTUAL_KEY(0xBB),
        "[" => VIRTUAL_KEY(0xDB),
        "]" => VIRTUAL_KEY(0xDD),
        "\\" => VIRTUAL_KEY(0xDC),
        ";" => VIRTUAL_KEY(0xBA),
        "'" => VIRTUAL_KEY(0xDE),
        "`" => VIRTUAL_KEY(0xC0),
        "," => VIRTUAL_KEY(0xBC),
        "." => VIRTUAL_KEY(0xBE),
        "/" => VIRTUAL_KEY(0xBF),
        _ => return None,
    };
    Some(vk)
}

fn is_extended(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_RCONTROL
            | VK_RMENU
            | VK_INSERT
            | VK_DELETE
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_LEFT
            | VK_RIGHT
            | VK_UP
            | VK_DOWN
            | VK_RWIN
            | VK_LWIN
            | VK_DIVIDE
            | VK_APPS
            | VK_SNAPSHOT
    )
}

pub(crate) fn send_key(vk: VIRTUAL_KEY, down: bool) {
    unsafe {
        let mapped = MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC_EX);
        let scan = (mapped & 0xff) as u16;
        let extended = (mapped & 0x100) != 0 || is_extended(vk);

        let mut flags = KEYEVENTF_SCANCODE;
        if extended {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        if !down {
            flags |= KEYEVENTF_KEYUP;
        }

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

pub(crate) fn send_mouse(button: MouseButton, down: bool) {
    let (flags, data): (MOUSE_EVENT_FLAGS, u32) = match (button, down) {
        (MouseButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
        (MouseButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
        (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
        (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
        (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
        (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
        (MouseButton::X1, true) => (MOUSEEVENTF_XDOWN, 0x0001),
        (MouseButton::X1, false) => (MOUSEEVENTF_XUP, 0x0001),
        (MouseButton::X2, true) => (MOUSEEVENTF_XDOWN, 0x0002),
        (MouseButton::X2, false) => (MOUSEEVENTF_XUP, 0x0002),
    };

    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

const NAMED_VKS: &[(VIRTUAL_KEY, &str)] = &[
    (VK_ESCAPE, "escape"),
    (VK_BACK, "backspace"),
    (VK_TAB, "tab"),
    (VK_RETURN, "enter"),
    (VK_LCONTROL, "lctrl"),
    (VK_RCONTROL, "rctrl"),
    (VK_LSHIFT, "lshift"),
    (VK_RSHIFT, "rshift"),
    (VK_LMENU, "lalt"),
    (VK_RMENU, "ralt"),
    (VK_LWIN, "lwin"),
    (VK_RWIN, "rwin"),
    (VK_SPACE, "space"),
    (VK_CAPITAL, "capslock"),
    (VK_NUMLOCK, "numlock"),
    (VK_SCROLL, "scrolllock"),
    (VK_INSERT, "insert"),
    (VK_DELETE, "delete"),
    (VK_HOME, "home"),
    (VK_END, "end"),
    (VK_PRIOR, "pageup"),
    (VK_NEXT, "pagedown"),
    (VK_LEFT, "left"),
    (VK_RIGHT, "right"),
    (VK_UP, "up"),
    (VK_DOWN, "down"),
    (VK_F1, "f1"),
    (VK_F2, "f2"),
    (VK_F3, "f3"),
    (VK_F4, "f4"),
    (VK_F5, "f5"),
    (VK_F6, "f6"),
    (VK_F7, "f7"),
    (VK_F8, "f8"),
    (VK_F9, "f9"),
    (VK_F10, "f10"),
    (VK_F11, "f11"),
    (VK_F12, "f12"),
    (VK_NUMPAD0, "num0"),
    (VK_NUMPAD1, "num1"),
    (VK_NUMPAD2, "num2"),
    (VK_NUMPAD3, "num3"),
    (VK_NUMPAD4, "num4"),
    (VK_NUMPAD5, "num5"),
    (VK_NUMPAD6, "num6"),
    (VK_NUMPAD7, "num7"),
    (VK_NUMPAD8, "num8"),
    (VK_NUMPAD9, "num9"),
    (VK_ADD, "numadd"),
    (VK_SUBTRACT, "numsub"),
    (VK_MULTIPLY, "nummul"),
    (VK_DIVIDE, "numdiv"),
    (VK_DECIMAL, "numdecimal"),
    (VK_PAUSE, "pause"),
    (VK_APPS, "apps"),
    (VK_SNAPSHOT, "printscreen"),
];

pub fn vk_to_key(vk: u16) -> Option<Key> {
    if (0x41..=0x5A).contains(&vk) {
        return Key::parse(&(vk as u8 as char).to_ascii_lowercase().to_string());
    }
    if (0x30..=0x39).contains(&vk) {
        return Key::parse(&(vk as u8 as char).to_string());
    }
    let name = NAMED_VKS.iter().find(|(k, _)| k.0 == vk).map(|(_, n)| *n)?;
    Key::parse(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_keys() {
        assert_eq!(key_to_vk("a"), Some(VIRTUAL_KEY(0x41)));
        assert_eq!(key_to_vk("z"), Some(VIRTUAL_KEY(0x5A)));
        assert_eq!(key_to_vk("5"), Some(VIRTUAL_KEY(0x35)));
        assert_eq!(key_to_vk("escape"), Some(VK_ESCAPE));
        assert_eq!(key_to_vk("space"), Some(VK_SPACE));
        assert_eq!(key_to_vk("lctrl"), Some(VK_LCONTROL));
        assert_eq!(key_to_vk("f2"), Some(VK_F2));
        assert_eq!(key_to_vk("not-a-key"), None);
    }

    #[test]
    fn vk_round_trips_through_key() {
        for name in [
            "a", "z", "5", "escape", "space", "lctrl", "f1", "f12", "num7",
        ] {
            let vk = key_to_vk(name).unwrap();
            let back = vk_to_key(vk.0).unwrap();
            assert_eq!(back.as_str(), name, "round trip failed for {name}");
        }
    }
}

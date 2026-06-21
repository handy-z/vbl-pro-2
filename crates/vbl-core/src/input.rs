use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyAction {
    Press,
    Release,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Key(String);

impl Key {
    pub fn parse(raw: &str) -> Option<Key> {
        normalize_macro_key(raw).map(Key)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_modifier(&self) -> bool {
        is_modifier_key(&self.0)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct KeyCombo {
    pub modifiers: Vec<Key>,
    pub key: Key,
}

impl KeyCombo {
    pub fn parse(raw: &str) -> Option<KeyCombo> {
        let canonical = normalize_macro_combo(raw)?;
        let mut parts: Vec<Key> = canonical.split('+').map(|p| Key(p.to_string())).collect();
        let key = parts.pop()?;
        Some(KeyCombo {
            modifiers: parts,
            key,
        })
    }

    pub fn canonical(&self) -> String {
        if self.modifiers.is_empty() {
            return self.key.0.clone();
        }
        let mut out = String::new();
        for m in &self.modifiers {
            out.push_str(&m.0);
            out.push('+');
        }
        out.push_str(&self.key.0);
        out
    }
}

pub fn is_modifier_key(key: &str) -> bool {
    matches!(
        key,
        "lctrl" | "rctrl" | "lalt" | "ralt" | "lshift" | "rshift" | "lwin" | "rwin"
    )
}

fn modifier_order(key: &str) -> u8 {
    match key {
        "lctrl" => 0,
        "rctrl" => 1,
        "lalt" => 2,
        "ralt" => 3,
        "lshift" => 4,
        "rshift" => 5,
        "lwin" => 6,
        "rwin" => 7,
        _ => 8,
    }
}

pub fn normalize_macro_key(value: &str) -> Option<String> {
    let lower = value.trim().to_ascii_lowercase();
    if lower.len() == 1 {
        let byte = lower.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Some(lower);
        }
    }

    match lower.as_str() {
        "esc" => Some("escape".to_string()),
        "control" | "ctrl" | "lcontrol" | "leftcontrol" | "lctrl" | "leftctrl" => {
            Some("lctrl".to_string())
        }
        "rcontrol" | "rightcontrol" | "rctrl" | "rightctrl" => Some("rctrl".to_string()),
        "shift" | "lshift" | "leftshift" => Some("lshift".to_string()),
        "rshift" | "rightshift" => Some("rshift".to_string()),
        "alt" | "lalt" | "leftalt" => Some("lalt".to_string()),
        "ralt" | "rightalt" | "altgraph" => Some("ralt".to_string()),
        "win" | "meta" | "super" | "lwin" | "leftwin" => Some("lwin".to_string()),
        "rwin" | "rightwin" => Some("rwin".to_string()),
        "return" => Some("enter".to_string()),
        "numpad0" => Some("num0".to_string()),
        "numpad1" => Some("num1".to_string()),
        "numpad2" => Some("num2".to_string()),
        "numpad3" => Some("num3".to_string()),
        "numpad4" => Some("num4".to_string()),
        "numpad5" => Some("num5".to_string()),
        "numpad6" => Some("num6".to_string()),
        "numpad7" => Some("num7".to_string()),
        "numpad8" => Some("num8".to_string()),
        "numpad9" => Some("num9".to_string()),
        "numpadadd" => Some("numadd".to_string()),
        "numpadsubtract" | "numpadsub" => Some("numsub".to_string()),
        "numpadmultiply" | "numpadmul" => Some("nummul".to_string()),
        "numpaddivide" | "numpaddiv" => Some("numdiv".to_string()),
        "numpaddecimal" | "numpaddec" => Some("numdecimal".to_string()),
        "contextmenu" | "menu" => Some("apps".to_string()),
        "printscreen" | "printscr" | "prtsc" => Some("printscreen".to_string()),
        "escape" | "backspace" | "tab" | "enter" | "space" | "capslock" | "numlock"
        | "scrolllock" | "insert" | "delete" | "home" | "end" | "pageup" | "pagedown" | "left"
        | "right" | "up" | "down" | "f1" | "f2" | "f3" | "f4" | "f5" | "f6" | "f7" | "f8"
        | "f9" | "f10" | "f11" | "f12" | "num0" | "num1" | "num2" | "num3" | "num4" | "num5"
        | "num6" | "num7" | "num8" | "num9" | "numadd" | "numsub" | "nummul" | "numdiv"
        | "numdecimal" | "pause" | "apps" | "-" | "=" | "[" | "]" | "\\" | ";" | "'" | "`"
        | "," | "." | "/" => Some(lower),
        _ => None,
    }
}

pub fn normalize_macro_combo(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.contains('+') {
        return normalize_macro_key(trimmed);
    }

    let mut modifiers: Vec<String> = Vec::new();
    let mut main_key: Option<String> = None;
    for part in trimmed.split('+') {
        let key = normalize_macro_key(part)?;
        if is_modifier_key(&key) {
            if !modifiers.iter().any(|m| m == &key) {
                modifiers.push(key);
            }
        } else if main_key.is_none() {
            main_key = Some(key);
        } else {
            return None;
        }
    }

    let main_key = main_key?;
    if modifiers.is_empty() {
        return Some(main_key);
    }

    modifiers.sort_by_key(|k| modifier_order(k));
    modifiers.push(main_key);
    Some(modifiers.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_keys_normalize() {
        assert_eq!(normalize_macro_key("ESC").as_deref(), Some("escape"));
        assert_eq!(normalize_macro_key("Ctrl").as_deref(), Some("lctrl"));
        assert_eq!(normalize_macro_key("A").as_deref(), Some("a"));
        assert_eq!(normalize_macro_key("a").as_deref(), Some("a"));
        assert_eq!(normalize_macro_key("5").as_deref(), Some("5"));
        assert_eq!(normalize_macro_key("F2").as_deref(), Some("f2"));
        assert_eq!(normalize_macro_key("numpad3").as_deref(), Some("num3"));
        assert_eq!(normalize_macro_key("Return").as_deref(), Some("enter"));
        assert_eq!(normalize_macro_key("not-a-key"), None);
    }

    #[test]
    fn combos_dedup_and_order() {
        assert_eq!(
            normalize_macro_combo("shift+ctrl+a").as_deref(),
            Some("lctrl+lshift+a")
        );
        assert_eq!(
            normalize_macro_combo("ctrl+ctrl+a").as_deref(),
            Some("lctrl+a")
        );

        assert_eq!(normalize_macro_combo("a+b"), None);
    }

    #[test]
    fn key_and_combo_types() {
        let k = Key::parse("Space").unwrap();
        assert_eq!(k.as_str(), "space");
        assert!(!k.is_modifier());
        assert!(Key::parse("ctrl").unwrap().is_modifier());

        let combo = KeyCombo::parse("ctrl+shift+a").unwrap();
        assert_eq!(combo.canonical(), "lctrl+lshift+a");
        assert_eq!(combo.key.as_str(), "a");
        assert_eq!(combo.modifiers.len(), 2);
    }
}

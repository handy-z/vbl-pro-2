use serde::{Deserialize, Serialize};

use crate::color::{Aggregate, Rgb, Tolerance};
use crate::geometry::{NormalizedPoint, PixelPoint, Region, Resolution};
use crate::state::StateKey;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum SkillMode {
    #[default]
    Normal,
    Boomjump,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct MacroKeybinds {
    pub enabled: bool,
    pub jumpset_key: String,
    pub skill_key: String,
    pub toggle_ultimate_key: String,
    pub respawn_key: String,

    #[serde(default = "default_kill_switch_key")]
    pub kill_switch_key: String,
}

fn default_kill_switch_key() -> String {
    "f8".to_string()
}

impl Default for MacroKeybinds {
    fn default() -> Self {
        Self {
            enabled: true,
            jumpset_key: "e".to_string(),
            skill_key: "lctrl".to_string(),
            toggle_ultimate_key: "f2".to_string(),
            respawn_key: "f1".to_string(),
            kill_switch_key: default_kill_switch_key(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CaptureState {
    pub key: StateKey,
    pub point: NormalizedPoint,
    pub target: Rgb,
    pub tolerance: Tolerance,
    #[serde(default)]
    pub region: Region,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CrosshairOffset {
    pub x: i32,
    pub y: i32,
}

impl Default for CrosshairOffset {
    fn default() -> Self {
        Self { x: 0, y: -200 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CrosshairConfig {
    pub enabled: bool,

    pub color: String,
    pub offset: CrosshairOffset,
    pub scale: f64,
    pub opacity: f64,
}

impl Default for CrosshairConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#FFFFFF".to_string(),
            offset: CrosshairOffset::default(),
            scale: 1.0,
            opacity: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct VblSettings {
    pub skill: SkillMode,
    pub macro_keys: MacroKeybinds,

    pub tap_ms: u64,
    pub capture: Vec<CaptureState>,
    pub crosshair: CrosshairConfig,

    #[serde(default)]
    pub script: Option<String>,

    #[serde(default)]
    pub dsl: Option<String>,

    #[serde(default)]
    pub unfocused_panic_ms: Option<u64>,
    /// Arm the engine automatically when the app launches.
    #[serde(default = "default_true")]
    pub arm_on_start: bool,
}

fn default_true() -> bool {
    true
}

pub fn default_capture() -> Vec<CaptureState> {
    let res = Resolution::new(1920, 1080);
    vec![
        CaptureState {
            key: StateKey::GameOnGround,
            point: PixelPoint::new(942, 1003).to_normalized(res),
            target: Rgb::new(255, 225, 148),
            tolerance: Tolerance {
                per_channel: 0,
                aggregate: Aggregate::All,
            },
            region: Region::single(),
        },
        CaptureState {
            key: StateKey::GameUltimateReady,
            point: PixelPoint::new(1030, 903).to_normalized(res),
            target: Rgb::new(255, 255, 255),
            tolerance: Tolerance {
                per_channel: 0,
                aggregate: Aggregate::All,
            },
            region: Region::single(),
        },
    ]
}

impl Default for VblSettings {
    fn default() -> Self {
        Self {
            skill: SkillMode::Normal,
            macro_keys: MacroKeybinds::default(),
            tap_ms: 35,
            capture: default_capture(),
            crosshair: CrosshairConfig::default(),
            script: None,
            dsl: None,
            unfocused_panic_ms: None,
            arm_on_start: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_v1() {
        let v = VblSettings::default();
        assert_eq!(v.skill, SkillMode::Normal);
        assert_eq!(v.macro_keys.jumpset_key, "e");
        assert_eq!(v.macro_keys.skill_key, "lctrl");
        assert_eq!(v.macro_keys.toggle_ultimate_key, "f2");
        assert_eq!(v.macro_keys.respawn_key, "f1");
        assert!(v.macro_keys.enabled);
        assert_eq!(v.tap_ms, 35);
    }

    #[test]
    fn default_capture_maps_back_to_v1_pixels_at_1080p() {
        let res = Resolution::new(1920, 1080);
        let v = VblSettings::default();

        let on_ground = v
            .capture
            .iter()
            .find(|c| c.key == StateKey::GameOnGround)
            .unwrap();
        assert_eq!(on_ground.point.to_pixel(res), PixelPoint::new(942, 1003));
        assert_eq!(on_ground.target, Rgb::new(255, 225, 148));

        let ult = v
            .capture
            .iter()
            .find(|c| c.key == StateKey::GameUltimateReady)
            .unwrap();
        assert_eq!(ult.point.to_pixel(res), PixelPoint::new(1030, 903));
        assert_eq!(ult.target, Rgb::new(255, 255, 255));
    }

    #[test]
    fn settings_round_trip_through_json() {
        let v = VblSettings::default();
        let json = serde_json::to_string(&v).unwrap();
        let back: VblSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.macro_keys, v.macro_keys);
        assert_eq!(back.tap_ms, v.tap_ms);
        assert_eq!(back.capture.len(), v.capture.len());
    }
}

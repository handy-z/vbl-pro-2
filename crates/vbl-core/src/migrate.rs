use serde::Deserialize;

use crate::color::{Aggregate, Rgb, Tolerance};
use crate::geometry::{PixelPoint, Region, Resolution};
use crate::profile::{
    CaptureState, CrosshairConfig, CrosshairOffset, MacroKeybinds, SkillMode, VblSettings,
};
use crate::state::StateKey;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LegacyConfig {
    skill: Option<String>,
    #[serde(rename = "macro")]
    macro_keys: Option<LegacyMacro>,
    crosshair: Option<LegacyCrosshair>,
    #[serde(alias = "watcher")]
    capture: Option<LegacyCapture>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LegacyMacro {
    enabled: Option<bool>,
    jumpset_key: Option<String>,
    skill_key: Option<String>,
    toggle_ultimate_key: Option<String>,
    respawn_key: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LegacyCrosshair {
    enabled: Option<bool>,
    color: Option<String>,
    offset: Option<LegacyOffset>,
    scale: Option<f64>,
    opacity: Option<f64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LegacyOffset {
    x: i32,
    y: i32,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LegacyCapture {
    resolutions: Vec<LegacyRes>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct LegacyRes {
    width: i32,
    height: i32,
    configs: Vec<LegacyPixel>,
}

#[derive(Deserialize)]
struct LegacyPixel {
    key: StateKey,
    point: [i32; 2],
    target: [i32; 3],
    #[serde(default)]
    tolerance: i32,
}

pub fn migrate_v1(config_json: &str) -> Option<VblSettings> {
    let legacy: LegacyConfig = serde_json::from_str(config_json).ok()?;
    let mut settings = VblSettings::default();

    if let Some(skill) = legacy.skill.as_deref() {
        if skill.eq_ignore_ascii_case("boomjump") {
            settings.skill = SkillMode::Boomjump;
        } else {
            settings.skill = SkillMode::Normal;
        }
    }

    if let Some(m) = legacy.macro_keys {
        let d = MacroKeybinds::default();
        settings.macro_keys = MacroKeybinds {
            enabled: m.enabled.unwrap_or(d.enabled),
            jumpset_key: m.jumpset_key.unwrap_or(d.jumpset_key),
            skill_key: m.skill_key.unwrap_or(d.skill_key),
            toggle_ultimate_key: m.toggle_ultimate_key.unwrap_or(d.toggle_ultimate_key),
            respawn_key: m.respawn_key.unwrap_or(d.respawn_key),
            kill_switch_key: d.kill_switch_key,
        };
    }

    if let Some(c) = legacy.crosshair {
        let d = CrosshairConfig::default();
        settings.crosshair = CrosshairConfig {
            enabled: c.enabled.unwrap_or(d.enabled),
            color: c.color.unwrap_or(d.color),
            offset: c
                .offset
                .map(|o| CrosshairOffset { x: o.x, y: o.y })
                .unwrap_or(d.offset),
            scale: c.scale.unwrap_or(d.scale),
            opacity: c.opacity.unwrap_or(d.opacity),
        };
    }

    if let Some(cap) = legacy.capture {
        if let Some(res0) = cap.resolutions.into_iter().find(|r| !r.configs.is_empty()) {
            let resolution = Resolution::new(res0.width, res0.height);
            if resolution.is_valid() {
                let states: Vec<CaptureState> = res0
                    .configs
                    .into_iter()
                    .map(|p| CaptureState {
                        key: p.key,
                        point: PixelPoint::new(p.point[0], p.point[1]).to_normalized(resolution),
                        target: Rgb::new(
                            p.target[0].clamp(0, 255) as u8,
                            p.target[1].clamp(0, 255) as u8,
                            p.target[2].clamp(0, 255) as u8,
                        ),
                        tolerance: Tolerance {
                            per_channel: p.tolerance.clamp(0, 255) as u8,
                            aggregate: Aggregate::All,
                        },
                        region: Region::single(),
                    })
                    .collect();
                if !states.is_empty() {
                    settings.capture = states;
                }
            }
        }
    }

    Some(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_CONFIG: &str = r##"{
        "skill": "boomjump",
        "macro": {
            "enabled": true,
            "jumpsetKey": "q",
            "skillKey": "lctrl",
            "toggleUltimateKey": "f3",
            "respawnKey": "f1"
        },
        "crosshair": {
            "enabled": false,
            "customImage": null,
            "color": "#FF0000",
            "offset": { "x": 5, "y": -150 },
            "scale": 1.5,
            "opacity": 0.8
        },
        "capture": {
            "resolutions": [
                {
                    "width": 1920,
                    "height": 1080,
                    "configs": [
                        { "key": "GameOnGround", "point": [942, 1003], "target": [255, 225, 148], "tolerance": 0 },
                        { "key": "GameUltimateReady", "point": [1030, 903], "target": [255, 255, 255], "tolerance": 4 }
                    ]
                },
                { "width": 1600, "height": 900, "configs": [] }
            ]
        },
        "window": { "alwaysOnTop": true }
    }"##;

    #[test]
    fn migrates_scalars_and_keys() {
        let s = migrate_v1(V1_CONFIG).unwrap();
        assert_eq!(s.skill, SkillMode::Boomjump);
        assert_eq!(s.macro_keys.jumpset_key, "q");
        assert_eq!(s.macro_keys.toggle_ultimate_key, "f3");
        assert_eq!(s.tap_ms, 35);
        assert!(!s.crosshair.enabled);
        assert_eq!(s.crosshair.color, "#FF0000");
        assert_eq!(s.crosshair.offset.y, -150);
        assert_eq!(s.crosshair.scale, 1.5);
    }

    #[test]
    fn migrates_capture_to_normalized_points() {
        let s = migrate_v1(V1_CONFIG).unwrap();
        let res = Resolution::new(1920, 1080);
        let ground = s
            .capture
            .iter()
            .find(|c| c.key == StateKey::GameOnGround)
            .unwrap();

        assert_eq!(ground.point.to_pixel(res), PixelPoint::new(942, 1003));
        assert_eq!(ground.target, Rgb::new(255, 225, 148));

        let ult = s
            .capture
            .iter()
            .find(|c| c.key == StateKey::GameUltimateReady)
            .unwrap();
        assert_eq!(ult.tolerance.per_channel, 4);
    }

    #[test]
    fn empty_or_partial_config_falls_back_to_defaults() {
        let s = migrate_v1("{}").unwrap();
        let d = VblSettings::default();
        assert_eq!(s.skill, d.skill);
        assert_eq!(s.macro_keys, d.macro_keys);
        assert_eq!(s.capture.len(), d.capture.len());
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(migrate_v1("not json").is_none());
    }
}

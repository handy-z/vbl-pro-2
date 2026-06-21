use serde::Serialize;
use serde_json::Value;

use crate::profile::VblSettings;

pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("invalid profile JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("profile schema v{found} is newer than supported v{supported}")]
    TooNew { found: u32, supported: u32 },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileDoc<'a> {
    schema_version: u32,
    #[serde(flatten)]
    settings: &'a VblSettings,
}

pub fn serialize_profile(settings: &VblSettings) -> Result<String, PersistError> {
    let doc = ProfileDoc {
        schema_version: PROFILE_SCHEMA_VERSION,
        settings,
    };
    Ok(serde_json::to_string_pretty(&doc)?)
}

pub fn deserialize_profile(json: &str) -> Result<VblSettings, PersistError> {
    let value: Value = serde_json::from_str(json)?;
    let version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(1) as u32;

    if version > PROFILE_SCHEMA_VERSION {
        return Err(PersistError::TooNew {
            found: version,
            supported: PROFILE_SCHEMA_VERSION,
        });
    }

    let value = migrate(value, version)?;

    Ok(serde_json::from_value(value)?)
}

fn migrate(value: Value, version: u32) -> Result<Value, PersistError> {
    let mut value = value;
    for from in version..PROFILE_SCHEMA_VERSION {
        value = upgrade_step(value, from);
    }
    Ok(value)
}

fn upgrade_step(value: Value, from: u32) -> Value {
    let _ = from;
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SkillMode;

    #[test]
    fn serialized_document_is_stamped() {
        let json = serialize_profile(&VblSettings::default()).unwrap();
        let value: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value.get("schemaVersion").and_then(Value::as_u64), Some(1));

        assert!(value.get("macroKeys").is_some());
        assert!(value.get("tapMs").is_some());
    }

    #[test]
    fn round_trip_preserves_settings() {
        let original = VblSettings {
            skill: SkillMode::Boomjump,
            tap_ms: 42,
            ..Default::default()
        };
        let json = serialize_profile(&original).unwrap();
        let back = deserialize_profile(&json).unwrap();
        assert_eq!(back.skill, SkillMode::Boomjump);
        assert_eq!(back.tap_ms, 42);
        assert_eq!(back.macro_keys, original.macro_keys);
        assert_eq!(back.capture.len(), original.capture.len());
    }

    #[test]
    fn legacy_document_without_version_loads_as_v1() {
        let legacy = r##"{
            "skill": "normal",
            "macroKeys": { "enabled": true, "jumpsetKey": "e", "skillKey": "lctrl",
                           "toggleUltimateKey": "f2", "respawnKey": "f1" },
            "tapMs": 35,
            "capture": [],
            "crosshair": { "enabled": true, "color": "#FFFFFF",
                           "offset": { "x": 0, "y": -200 }, "scale": 1.0, "opacity": 1.0 }
        }"##;
        let settings = deserialize_profile(legacy).unwrap();
        assert_eq!(settings.tap_ms, 35);
        assert_eq!(settings.macro_keys.kill_switch_key, "f8");
        assert!(settings.script.is_none());
    }

    #[test]
    fn future_schema_is_rejected() {
        let json = r#"{ "schemaVersion": 999, "tapMs": 35 }"#;
        assert!(matches!(
            deserialize_profile(json),
            Err(PersistError::TooNew { found: 999, .. })
        ));
    }

    #[test]
    fn malformed_json_errors() {
        assert!(deserialize_profile("{ not json").is_err());
    }
}

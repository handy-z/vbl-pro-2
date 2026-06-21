use serde::{Deserialize, Serialize};
use vbl_core::state::EngineState;

pub use vbl_core::event::{LogEntry, LogKind};
pub use vbl_core::state::StateKey;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub armed: bool,
    pub target_focused: bool,
    pub capture_matched: bool,
    pub game_on_ground: bool,
    pub game_ultimate_ready: bool,
    pub x1_held: bool,
    pub x2_held: bool,
    pub skill_enabled: bool,
    pub resolution: Option<(i32, i32)>,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PixelPick {
    pub x: i32,
    pub y: i32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CaptureSampleDto {
    pub key: StateKey,
    pub x: i32,
    pub y: i32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub matched: bool,
}

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct InjectionDto {
    pub ts_ms: u64,
    pub label: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub capture_micros: u64,

    pub capture_p50_micros: u64,

    pub capture_p95_micros: u64,

    pub capture_max_micros: u64,

    pub capture_samples: u64,

    pub injections: u64,

    pub poll_count: u64,
}

impl RuntimeStatus {
    pub fn from_state(state: &EngineState) -> Self {
        Self {
            armed: state.armed,
            target_focused: state.target_focused,
            capture_matched: state.capture_matched,
            game_on_ground: state.game_on_ground,
            game_ultimate_ready: state.game_ultimate_ready,
            x1_held: state.x1_held,
            x2_held: state.x2_held,
            skill_enabled: state.skill_enabled,
            resolution: state.resolution.map(|r| (r.width, r.height)),
        }
    }
}

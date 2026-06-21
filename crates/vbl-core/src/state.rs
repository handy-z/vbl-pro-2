use serde::{Deserialize, Serialize};

use crate::geometry::Resolution;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum StateKey {
    #[serde(rename = "GameOnGround")]
    GameOnGround,
    #[serde(rename = "GameUltimateReady", alias = "GameSkillReady")]
    GameUltimateReady,
    #[serde(rename = "X1Held")]
    X1Held,
    #[serde(rename = "X2Held")]
    X2Held,
    #[serde(rename = "skillEnabled")]
    SkillEnabled,
    #[serde(rename = "robloxFocused")]
    RobloxFocused,
}

impl StateKey {
    pub const ALL: [StateKey; 6] = [
        StateKey::GameOnGround,
        StateKey::GameUltimateReady,
        StateKey::X1Held,
        StateKey::X2Held,
        StateKey::SkillEnabled,
        StateKey::RobloxFocused,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StateKey::GameOnGround => "GameOnGround",
            StateKey::GameUltimateReady => "GameUltimateReady",
            StateKey::X1Held => "X1Held",
            StateKey::X2Held => "X2Held",
            StateKey::SkillEnabled => "skillEnabled",
            StateKey::RobloxFocused => "robloxFocused",
        }
    }

    pub fn from_name(name: &str) -> Option<StateKey> {
        StateKey::ALL
            .into_iter()
            .find(|k| k.name() == name)
            .or(match name {
                "GameSkillReady" => Some(StateKey::GameUltimateReady),
                _ => None,
            })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EngineState {
    pub armed: bool,

    pub target_focused: bool,

    pub capture_matched: bool,

    pub resolution: Option<Resolution>,
    pub game_on_ground: bool,
    pub game_ultimate_ready: bool,
    pub x1_held: bool,
    pub x2_held: bool,

    pub skill_enabled: bool,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            armed: false,
            target_focused: false,
            capture_matched: false,
            resolution: None,
            game_on_ground: false,
            game_ultimate_ready: false,
            x1_held: false,
            x2_held: false,

            skill_enabled: true,
        }
    }
}

impl EngineState {
    pub fn get(&self, key: StateKey) -> bool {
        match key {
            StateKey::GameOnGround => self.game_on_ground,
            StateKey::GameUltimateReady => self.game_ultimate_ready,
            StateKey::X1Held => self.x1_held,
            StateKey::X2Held => self.x2_held,
            StateKey::SkillEnabled => self.skill_enabled,
            StateKey::RobloxFocused => self.target_focused,
        }
    }

    pub fn set(&mut self, key: StateKey, value: bool) {
        match key {
            StateKey::GameOnGround => self.game_on_ground = value,
            StateKey::GameUltimateReady => self.game_ultimate_ready = value,
            StateKey::X1Held => self.x1_held = value,
            StateKey::X2Held => self.x2_held = value,
            StateKey::SkillEnabled => self.skill_enabled = value,
            StateKey::RobloxFocused => self.target_focused = value,
        }
    }

    pub fn macro_allowed(&self, macro_enabled: bool) -> bool {
        self.armed && self.target_focused && macro_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_enabled_defaults_on() {
        assert!(EngineState::default().skill_enabled);
    }

    #[test]
    fn three_gate_requires_all() {
        let mut s = EngineState::default();
        assert!(!s.macro_allowed(true));
        s.armed = true;
        assert!(!s.macro_allowed(true));
        s.target_focused = true;
        assert!(s.macro_allowed(true));
        assert!(!s.macro_allowed(false));
    }

    #[test]
    fn get_set_round_trip() {
        let mut s = EngineState::default();
        s.set(StateKey::GameOnGround, true);
        assert!(s.get(StateKey::GameOnGround));
        s.set(StateKey::RobloxFocused, true);
        assert!(s.target_focused);
        assert!(s.get(StateKey::RobloxFocused));
    }
}

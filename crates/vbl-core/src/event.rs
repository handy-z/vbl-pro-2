use serde::{Deserialize, Serialize};

use crate::state::StateKey;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum LogKind {
    Log,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: u64,
    pub kind: LogKind,
    pub timestamp_ms: u64,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum EngineCommand {
    Arm,
    Disarm,
    ToggleArmed,
    LoadProfile(String),
    ReloadScript,
    Shutdown,
}

#[derive(Clone, Debug)]
pub enum EngineEvent {
    StateChanged { key: StateKey, value: bool },
    Log(LogEntry),
    Error(String),
}

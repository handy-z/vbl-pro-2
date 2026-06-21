use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("capture error: {0}")]
    Capture(String),

    #[error("input error: {0}")]
    Input(String),

    #[error("overlay error: {0}")]
    Overlay(String),

    #[error("script error: {0}")]
    Script(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("profile not found: {0}")]
    ProfileNotFound(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;

#![forbid(unsafe_code)]

pub mod actions;
pub mod color;
pub mod dsl;
pub mod error;
pub mod event;
pub mod executor;
pub mod geometry;
pub mod input;
pub mod macros;
pub mod migrate;
pub mod persist;
pub mod profile;
pub mod state;
pub mod testing;
pub mod time;
pub mod traits;
pub mod vbl;

pub use error::{EngineError, Result};
pub use state::{EngineState, StateKey};
pub use vbl::Vbl;

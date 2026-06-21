#[cfg(windows)]
mod driver;
#[cfg(windows)]
mod runtime;
mod store;

#[cfg(windows)]
pub use runtime::{Command, Runtime, RuntimeUpdate};
pub use store::Store;

pub use vbl_ipc::{CaptureSampleDto, InjectionDto, LogEntry, Metrics, PixelPick, RuntimeStatus};

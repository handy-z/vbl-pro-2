use crate::color::{Rgb, Tolerance};
use crate::geometry::{PixelPoint, Region, Resolution};
use crate::input::{Key, KeyAction, MouseButton};
use crate::state::StateKey;

pub trait InputSink: Send + Sync {
    fn key(&self, key: &Key, action: KeyAction);
    fn mouse_button(&self, button: MouseButton, action: KeyAction);

    fn release_all(&self);
}

#[derive(Clone, Debug)]
pub struct CapturePoint {
    pub key: StateKey,
    pub point: PixelPoint,
    pub target: Rgb,
    pub tolerance: Tolerance,
    pub region: Region,
}

#[derive(Clone, Debug)]
pub struct PixelSample {
    pub key: StateKey,
    pub point: PixelPoint,
    pub rgb: Rgb,
    pub matched: bool,
}

pub trait ScreenCapture: Send + Sync {
    fn sample(&self, points: &[CapturePoint]) -> Vec<PixelSample>;
    fn current_resolution(&self) -> Resolution;
}

#[derive(Clone, Copy, Debug)]
pub struct ClientRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct TargetWindow {
    pub client: ClientRect,
}

pub trait WindowTracker: Send + Sync {
    fn target_window(&self) -> Option<TargetWindow>;
    fn is_target_focused(&self) -> bool;
}

pub trait Overlay: Send + Sync {
    fn show(&self);
    fn update(&self);
    fn hide(&self);
}

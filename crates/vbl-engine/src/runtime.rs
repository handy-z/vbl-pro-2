use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vbl_core::color::Rgb;
use vbl_core::geometry::Resolution;
use vbl_core::input::{Key, KeyAction, MouseButton};
use vbl_core::profile::{CaptureState, CrosshairConfig, VblSettings};
use vbl_core::state::StateKey;
use vbl_core::traits::{CapturePoint, InputSink, ScreenCapture, TargetWindow, WindowTracker};
use vbl_ipc::{
    CaptureSampleDto, InjectionDto, LogEntry, LogKind, Metrics, PixelPick, RuntimeStatus,
};

use crate::driver::Driver;
use vbl_platform_win::{
    cursor_pixel, start_hook, stop_hook, vk_to_key, DxgiCapture, GdiCapture, RawInputEvent,
    WinClock, WinInputSink, WinOverlay, WinWindowTracker,
};

const MAX_INJECTIONS: usize = 200;

const CAPTURE_WINDOW: usize = 240;

#[derive(Clone, Default)]
struct Telemetry {
    injections: Arc<Mutex<VecDeque<InjectionDto>>>,
    inject_count: Arc<AtomicU64>,

    capture_micros: Arc<AtomicU64>,

    capture_window: Arc<Mutex<VecDeque<u32>>>,
    capture_samples: Arc<AtomicU64>,
    poll_count: Arc<AtomicU64>,
}

impl Telemetry {
    fn record_capture(&self, micros: u64) {
        self.capture_micros.store(micros, Ordering::Relaxed);
        self.capture_samples.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut w) = self.capture_window.lock() {
            w.push_back(micros.min(u32::MAX as u64) as u32);
            while w.len() > CAPTURE_WINDOW {
                w.pop_front();
            }
        }
    }
}

fn percentile(sorted: &[u32], pct: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (pct * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank.min(sorted.len() - 1)] as u64
}

struct RecordingSink {
    inner: Arc<dyn InputSink>,
    telemetry: Telemetry,
}

impl RecordingSink {
    fn record(&self, label: String) {
        #[cfg(debug_assertions)]
        eprintln!("[inject] {label}");
        self.telemetry.inject_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut q) = self.telemetry.injections.lock() {
            q.push_back(InjectionDto {
                ts_ms: now_ms(),
                label,
            });
            while q.len() > MAX_INJECTIONS {
                q.pop_front();
            }
        }
    }
}

impl InputSink for RecordingSink {
    fn key(&self, key: &Key, action: KeyAction) {
        self.inner.key(key, action);
        self.record(format!("{} {}", action_label(action), key.as_str()));
    }
    fn mouse_button(&self, button: MouseButton, action: KeyAction) {
        self.inner.mouse_button(button, action);
        self.record(format!(
            "{} {} mouse",
            action_label(action),
            button_label(button)
        ));
    }
    fn release_all(&self) {
        self.inner.release_all();
        self.record("release all".to_string());
    }
}

fn action_label(action: KeyAction) -> &'static str {
    match action {
        KeyAction::Press => "press",
        KeyAction::Release => "release",
    }
}

fn button_label(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "left",
        MouseButton::Right => "right",
        MouseButton::Middle => "middle",
        MouseButton::X1 => "x1",
        MouseButton::X2 => "x2",
    }
}

const POLL_INTERVAL: Duration = Duration::from_millis(8);
const MAX_IDLE: Duration = Duration::from_millis(50);

const LOOP_POLL: Duration = Duration::from_millis(2);

struct ReleaseGuard(Arc<dyn InputSink>);

impl Drop for ReleaseGuard {
    fn drop(&mut self) {
        self.0.release_all();
    }
}

#[derive(Clone, Debug)]
pub enum RuntimeUpdate {
    Status(RuntimeStatus),
    Log(LogEntry),
}

pub enum Command {
    Arm,
    Disarm,
    ToggleArmed,
    UpdateSettings(Box<VblSettings>),

    ReloadScript,
    Shutdown,
}

enum Message {
    Input(RawInputEvent),
    Command(Command),
}

pub struct Runtime {
    tx: Sender<Message>,
    handle: Option<JoinHandle<()>>,
    status: Arc<Mutex<RuntimeStatus>>,
    settings: Arc<Mutex<VblSettings>>,
    telemetry: Telemetry,
}

impl Runtime {
    pub fn start(
        settings: VblSettings,
        on_update: impl Fn(RuntimeUpdate) + Send + 'static,
    ) -> Runtime {
        let (tx, rx) = mpsc::channel::<Message>();

        let hook_tx = tx.clone();
        start_hook(Box::new(move |ev| {
            let _ = hook_tx.send(Message::Input(ev));
        }));

        let status = Arc::new(Mutex::new(RuntimeStatus::default()));
        let settings_shared = Arc::new(Mutex::new(settings.clone()));
        let telemetry = Telemetry::default();

        let status_thread = status.clone();
        let settings_thread = settings_shared.clone();
        let telemetry_thread = telemetry.clone();
        let handle = thread::Builder::new()
            .name("vbl-runtime".into())
            .spawn(move || {
                run(
                    settings,
                    rx,
                    Box::new(on_update),
                    status_thread,
                    settings_thread,
                    telemetry_thread,
                )
            })
            .expect("spawn runtime thread");

        Runtime {
            tx,
            handle: Some(handle),
            status,
            settings: settings_shared,
            telemetry,
        }
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn settings(&self) -> VblSettings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn arm(&self) {
        let _ = self.tx.send(Message::Command(Command::Arm));
    }

    pub fn disarm(&self) {
        let _ = self.tx.send(Message::Command(Command::Disarm));
    }

    pub fn toggle_armed(&self) {
        let _ = self.tx.send(Message::Command(Command::ToggleArmed));
    }

    pub fn update_settings(&self, settings: VblSettings) {
        let _ = self
            .tx
            .send(Message::Command(Command::UpdateSettings(Box::new(
                settings,
            ))));
    }

    pub fn reload_script(&self) {
        let _ = self.tx.send(Message::Command(Command::ReloadScript));
    }

    pub fn pick_pixel(&self) -> Option<PixelPick> {
        cursor_pixel().map(|(x, y, rgb)| PixelPick {
            x,
            y,
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        })
    }

    pub fn sample_capture(&self) -> Vec<CaptureSampleDto> {
        let settings = self.settings();
        let capture = GdiCapture::new();
        let res = capture.current_resolution();
        if !res.is_valid() {
            return Vec::new();
        }
        let points: Vec<CapturePoint> = settings
            .capture
            .iter()
            .map(|c| CapturePoint {
                key: c.key,
                point: c.point.to_pixel(res),
                target: c.target,
                tolerance: c.tolerance,
                region: c.region,
            })
            .collect();
        capture
            .sample(&points)
            .into_iter()
            .map(|s| CaptureSampleDto {
                key: s.key,
                x: s.point.x,
                y: s.point.y,
                r: s.rgb.r,
                g: s.rgb.g,
                b: s.rgb.b,
                matched: s.matched,
            })
            .collect()
    }

    pub fn recent_injections(&self) -> Vec<InjectionDto> {
        self.telemetry
            .injections
            .lock()
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn metrics(&self) -> Metrics {
        let mut window: Vec<u32> = self
            .telemetry
            .capture_window
            .lock()
            .map(|w| w.iter().copied().collect())
            .unwrap_or_default();
        window.sort_unstable();
        Metrics {
            capture_micros: self.telemetry.capture_micros.load(Ordering::Relaxed),
            capture_p50_micros: percentile(&window, 0.50),
            capture_p95_micros: percentile(&window, 0.95),
            capture_max_micros: window.last().copied().map(u64::from).unwrap_or(0),
            capture_samples: self.telemetry.capture_samples.load(Ordering::Relaxed),
            injections: self.telemetry.inject_count.load(Ordering::Relaxed),
            poll_count: self.telemetry.poll_count.load(Ordering::Relaxed),
        }
    }

    pub fn command(&self, command: Command) {
        let _ = self.tx.send(Message::Command(command));
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let _ = self.tx.send(Message::Command(Command::Shutdown));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        stop_hook();
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.stop();
        }
    }
}

fn run(
    settings: VblSettings,
    rx: Receiver<Message>,
    on_update: Box<dyn Fn(RuntimeUpdate) + Send>,
    status_shared: Arc<Mutex<RuntimeStatus>>,
    settings_shared: Arc<Mutex<VblSettings>>,
    telemetry: Telemetry,
) {
    let clock = Arc::new(WinClock::new());
    let real_sink: Arc<dyn InputSink> = Arc::new(WinInputSink::new());
    let sink: Arc<dyn InputSink> = Arc::new(RecordingSink {
        inner: real_sink,
        telemetry: telemetry.clone(),
    });

    let _release_guard = ReleaseGuard(sink.clone());

    let tracker = WinWindowTracker::for_roblox();

    let capture = DxgiCapture::new();
    let overlay = WinOverlay::start();

    let mut crosshair = settings.crosshair.clone();
    let mut capture_cfg = settings.capture.clone();
    let mut kill_switch = parse_key_opt(&settings.macro_keys.kill_switch_key);
    let mut panic_ms = settings.unfocused_panic_ms;
    let mut active_program = program_key(&settings);
    let (mut driver, load_err) = Driver::new(clock.clone(), sink.clone(), settings);

    let mut last_poll = Duration::ZERO;
    let mut last_res: Option<(i32, i32)> = None;
    let mut last_status: Option<RuntimeStatus> = None;
    let mut unfocused_since: Option<Instant> = None;
    let mut log_id: u64 = 0;

    driver.set_focused(tracker.target_window().is_some());
    emit_log(&*on_update, &mut log_id, "Engine started");
    emit_log(
        &*on_update,
        &mut log_id,
        &format!("Capture backend: {}", capture.backend()),
    );
    if let Some(label) = driver.active_label() {
        emit_log(&*on_update, &mut log_id, &format!("{label} active"));
    }
    if let Some(err) = load_err {
        emit_error(&*on_update, &mut log_id, &err);
    }
    publish(
        &mut last_status,
        &status_shared,
        &*on_update,
        &driver,
        last_res,
        &mut log_id,
    );

    loop {
        let now = driver.now();
        let until_wake = driver
            .next_wake()
            .map(|w| w.saturating_sub(now))
            .unwrap_or(MAX_IDLE);
        let until_poll = POLL_INTERVAL.saturating_sub(now.saturating_sub(last_poll));

        let ceiling = if driver.loop_active() {
            LOOP_POLL
        } else {
            MAX_IDLE
        };
        let timeout = until_wake
            .min(until_poll)
            .min(ceiling)
            .max(Duration::from_millis(1));

        match rx.recv_timeout(timeout) {
            Ok(Message::Input(event)) => {
                if killswitch_hit(&event, kill_switch.as_ref()) {
                    driver.set_armed(false);
                    sink.release_all();
                    emit_log(&*on_update, &mut log_id, "Kill switch — disarmed");
                } else {
                    handle_input(&mut driver, event);
                }
            }
            Ok(Message::Command(Command::Shutdown)) => break,
            Ok(Message::Command(Command::UpdateSettings(settings))) => {
                crosshair = settings.crosshair.clone();
                capture_cfg = settings.capture.clone();
                let next_program = program_key(&settings);
                if next_program == active_program {
                    driver.apply_settings((*settings).clone());
                } else {
                    sink.release_all();
                    active_program = next_program;
                    let (next, err) = Driver::new(clock.clone(), sink.clone(), (*settings).clone());
                    driver = next;
                    driver.set_armed(last_status.as_ref().is_some_and(|s| s.armed));
                    driver.set_focused(tracker.target_window().is_some());
                    log_active(&*on_update, &mut log_id, &driver, err);
                }
                kill_switch = parse_key_opt(&settings.macro_keys.kill_switch_key);
                panic_ms = settings.unfocused_panic_ms;
                if let Ok(mut guard) = settings_shared.lock() {
                    *guard = *settings;
                }
                emit_log(&*on_update, &mut log_id, "Settings updated");
            }
            Ok(Message::Command(Command::ReloadScript)) => {
                sink.release_all();
                let settings = settings_shared
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_default();
                kill_switch = parse_key_opt(&settings.macro_keys.kill_switch_key);
                panic_ms = settings.unfocused_panic_ms;
                active_program = program_key(&settings);
                let armed = last_status.as_ref().is_some_and(|s| s.armed);
                let (next, err) = Driver::new(clock.clone(), sink.clone(), settings);
                driver = next;
                driver.set_armed(armed);
                driver.set_focused(tracker.target_window().is_some());
                emit_log(&*on_update, &mut log_id, "Macro program reloaded");
                log_active(&*on_update, &mut log_id, &driver, err);
            }
            Ok(Message::Command(command)) => handle_command(&mut driver, command),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let now = driver.now();
        if now.saturating_sub(last_poll) >= POLL_INTERVAL {
            telemetry.poll_count.fetch_add(1, Ordering::Relaxed);
            let target = tracker.target_window();
            driver.set_focused(target.is_some());

            if target.is_some() {
                unfocused_since = None;
            } else if let Some(limit) = panic_ms.filter(|_| driver.state().armed) {
                let since = *unfocused_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(limit) {
                    driver.set_armed(false);
                    sink.release_all();
                    emit_log(&*on_update, &mut log_id, "Unfocused failsafe — disarmed");
                    unfocused_since = None;
                }
            }

            let res = capture.current_resolution();
            last_res = res.is_valid().then_some((res.width, res.height));

            let st = driver.state();
            if st.armed && st.target_focused {
                let t0 = Instant::now();
                poll_capture(&mut driver, &capture, &capture_cfg, res);
                telemetry.record_capture(t0.elapsed().as_micros() as u64);
            }
            update_overlay(&overlay, &crosshair, st.armed, target);
            last_poll = now;
        }

        driver.pump();
        driver.advance(driver.now());
        let _ = driver.take_events();
        for line in driver.take_logs() {
            emit_log(&*on_update, &mut log_id, &line);
        }
        for err in driver.take_errors() {
            emit_error(&*on_update, &mut log_id, &err);
        }
        publish(
            &mut last_status,
            &status_shared,
            &*on_update,
            &driver,
            last_res,
            &mut log_id,
        );
    }

    driver.set_armed(false);
    overlay.stop();
    let _ = driver.take_events();
    emit_log(&*on_update, &mut log_id, "Engine stopped");
    publish(
        &mut last_status,
        &status_shared,
        &*on_update,
        &driver,
        last_res,
        &mut log_id,
    );
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn program_key(settings: &VblSettings) -> (Option<String>, Option<String>) {
    (settings.script.clone(), settings.dsl.clone())
}

fn parse_key_opt(raw: &str) -> Option<Key> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| Key::parse(trimmed)).flatten()
}

fn killswitch_hit(event: &RawInputEvent, kill: Option<&Key>) -> bool {
    let Some(kill) = kill else {
        return false;
    };
    matches!(event, RawInputEvent::Key { vk, down: true } if vk_to_key(*vk).as_ref() == Some(kill))
}

fn log_active(
    on_update: &dyn Fn(RuntimeUpdate),
    log_id: &mut u64,
    driver: &Driver,
    err: Option<String>,
) {
    match err {
        Some(e) => emit_error(on_update, log_id, &e),
        None => {
            let label = driver.active_label().unwrap_or("Built-in profile");
            emit_log(on_update, log_id, &format!("{label} active"));
        }
    }
}

fn emit_log(on_update: &dyn Fn(RuntimeUpdate), log_id: &mut u64, message: &str) {
    emit(on_update, log_id, LogKind::Log, message);
}

fn emit_error(on_update: &dyn Fn(RuntimeUpdate), log_id: &mut u64, message: &str) {
    emit(on_update, log_id, LogKind::Error, message);
}

fn emit(on_update: &dyn Fn(RuntimeUpdate), log_id: &mut u64, kind: LogKind, message: &str) {
    *log_id += 1;
    on_update(RuntimeUpdate::Log(LogEntry {
        id: *log_id,
        kind,
        timestamp_ms: now_ms(),
        message: message.to_string(),
    }));
}

fn publish(
    last: &mut Option<RuntimeStatus>,
    shared: &Arc<Mutex<RuntimeStatus>>,
    on_update: &dyn Fn(RuntimeUpdate),
    driver: &Driver,
    res: Option<(i32, i32)>,
    log_id: &mut u64,
) {
    let mut snap = RuntimeStatus::from_state(&driver.state());
    snap.resolution = res;
    if last.as_ref() == Some(&snap) {
        return;
    }

    if let Some(prev) = last.as_ref() {
        if prev.armed != snap.armed {
            emit_log(
                on_update,
                log_id,
                if snap.armed { "Armed" } else { "Disarmed" },
            );
        }
        if prev.target_focused != snap.target_focused {
            emit_log(
                on_update,
                log_id,
                if snap.target_focused {
                    "Roblox focused"
                } else {
                    "Roblox not focused"
                },
            );
        }
        if prev.skill_enabled != snap.skill_enabled {
            emit_log(
                on_update,
                log_id,
                if snap.skill_enabled {
                    "Ultimate enabled"
                } else {
                    "Ultimate disabled"
                },
            );
        }
    }

    if let Ok(mut guard) = shared.lock() {
        *guard = snap.clone();
    }
    on_update(RuntimeUpdate::Status(snap.clone()));
    *last = Some(snap);
}

fn handle_input(driver: &mut Driver, event: RawInputEvent) {
    match event {
        RawInputEvent::MouseButton {
            button: MouseButton::X1,
            down,
        } => {
            if down {
                driver.press_x1();
            } else {
                driver.press_x1_up();
            }
        }
        RawInputEvent::MouseButton {
            button: MouseButton::X2,
            down,
        } => {
            if down {
                driver.press_x2();
            } else {
                driver.press_x2_up();
            }
        }
        RawInputEvent::MouseButton { .. } => {}
        RawInputEvent::Key { vk, down: true } => {
            if let Some(key) = vk_to_key(vk) {
                driver.key_down(&key);
            }
        }
        RawInputEvent::Key { down: false, .. } => {}
    }
}

fn handle_command(driver: &mut Driver, command: Command) {
    match command {
        Command::Arm => driver.set_armed(true),
        Command::Disarm => driver.set_armed(false),
        Command::ToggleArmed => {
            let armed = driver.state().armed;
            driver.set_armed(!armed);
        }
        Command::UpdateSettings(_) | Command::ReloadScript | Command::Shutdown => {}
    }
}

fn poll_capture(driver: &mut Driver, capture: &DxgiCapture, cfg: &[CaptureState], res: Resolution) {
    if !res.is_valid() {
        return;
    }
    let points: Vec<CapturePoint> = cfg
        .iter()
        .map(|c| CapturePoint {
            key: c.key,
            point: c.point.to_pixel(res),
            target: c.target,
            tolerance: c.tolerance,
            region: c.region,
        })
        .collect();

    for sample in capture.sample(&points) {
        match sample.key {
            StateKey::GameOnGround => driver.set_on_ground(sample.matched),
            StateKey::GameUltimateReady => driver.set_ultimate_ready(sample.matched),
            _ => {}
        }
    }
}

fn update_overlay(
    overlay: &WinOverlay,
    crosshair: &CrosshairConfig,
    armed: bool,
    target: Option<TargetWindow>,
) {
    let white = Rgb::new(255, 255, 255);
    match target {
        Some(tw) if armed && crosshair.enabled => {
            let cx = tw.client.x + tw.client.width / 2 + crosshair.offset.x;
            let cy = tw.client.y + tw.client.height / 2 + crosshair.offset.y;
            let color = Rgb::from_hex(&crosshair.color).unwrap_or(white);
            overlay.set(true, cx, cy, color, crosshair.opacity, crosshair.scale);
        }
        _ => overlay.set(false, 0, 0, white, 1.0, 1.0),
    }
}

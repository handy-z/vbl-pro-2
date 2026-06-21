#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, State, WindowEvent};
use tauri_specta::{collect_commands, collect_events, Builder, Event};
use vbl_core::color::{Rgb, Tolerance};
use vbl_core::profile::VblSettings;
use vbl_engine::{
    CaptureSampleDto, InjectionDto, LogEntry, Metrics, PixelPick, Runtime, RuntimeStatus,
    RuntimeUpdate, Store,
};

struct AppState {
    runtime: Mutex<Option<Runtime>>,
    store: Store,
    active: Mutex<String>,
}

#[derive(Clone, Serialize, Deserialize, Type, Event)]
pub struct StatusEvent(pub RuntimeStatus);

#[derive(Clone, Serialize, Deserialize, Type, Event)]
pub struct LogEvent(pub LogEntry);

fn with_runtime(state: &State<'_, AppState>, f: impl FnOnce(&Runtime)) {
    if let Ok(guard) = state.runtime.lock() {
        if let Some(runtime) = guard.as_ref() {
            f(runtime);
        }
    }
}

fn current_settings(state: &State<'_, AppState>) -> Option<VblSettings> {
    let guard = state.runtime.lock().ok()?;
    guard.as_ref().map(|r| r.settings())
}

#[tauri::command]
#[specta::specta]
fn get_status(state: State<'_, AppState>) -> RuntimeStatus {
    state
        .runtime
        .lock()
        .ok()
        .and_then(|r| r.as_ref().map(|r| r.status()))
        .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
fn get_config(state: State<'_, AppState>) -> VblSettings {
    current_settings(&state).unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
fn update_config(config: VblSettings, state: State<'_, AppState>) {
    with_runtime(&state, |r| r.update_settings(config.clone()));
    if let Ok(active) = state.active.lock() {
        let _ = state.store.save_profile(&active, &config);
    }
}

#[tauri::command]
#[specta::specta]
fn list_profiles(state: State<'_, AppState>) -> Vec<String> {
    state.store.list_profiles()
}

#[tauri::command]
#[specta::specta]
fn active_profile(state: State<'_, AppState>) -> String {
    state.active.lock().map(|a| a.clone()).unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
fn switch_profile(name: String, state: State<'_, AppState>) -> Option<VblSettings> {
    let settings = state.store.load_profile(&name)?;
    with_runtime(&state, |r| r.update_settings(settings.clone()));
    let _ = state.store.set_active(&name);
    if let Ok(mut active) = state.active.lock() {
        *active = name;
    }
    Some(settings)
}

#[tauri::command]
#[specta::specta]
fn save_profile_as(name: String, state: State<'_, AppState>) {
    if let Some(settings) = current_settings(&state) {
        let _ = state.store.save_profile(&name, &settings);
        let _ = state.store.set_active(&name);
        if let Ok(mut active) = state.active.lock() {
            *active = name;
        }
    }
}

#[tauri::command]
#[specta::specta]
fn delete_profile(name: String, state: State<'_, AppState>) -> Option<VblSettings> {
    let _ = state.store.delete_profile(&name);

    let was_active = state.active.lock().map(|a| *a == name).unwrap_or(false);
    if !was_active {
        return None;
    }

    let (next_name, settings) = state
        .store
        .list_profiles()
        .into_iter()
        .find_map(|n| state.store.load_profile(&n).map(|s| (n, s)))
        .unwrap_or_else(|| {
            let settings = VblSettings::default();
            let _ = state.store.save_profile("default", &settings);
            ("default".to_string(), settings)
        });

    with_runtime(&state, |r| r.update_settings(settings.clone()));
    let _ = state.store.set_active(&next_name);
    if let Ok(mut active) = state.active.lock() {
        *active = next_name;
    }
    Some(settings)
}

/// Write the active profile to a JSON file the user picked.
#[tauri::command]
#[specta::specta]
fn export_profile(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let settings = current_settings(&state).ok_or("no active profile")?;
    let json = vbl_core::persist::serialize_profile(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Import a profile from a JSON file, validate it, save it under the file's name, make it active,
/// and load it into the engine. Returns the imported settings.
#[tauri::command]
#[specta::specta]
fn import_profile(path: String, state: State<'_, AppState>) -> Result<VblSettings, String> {
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let settings = vbl_core::persist::deserialize_profile(&json).map_err(|e| e.to_string())?;

    let name = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("imported")
        .to_string();

    state
        .store
        .save_profile(&name, &settings)
        .map_err(|e| e.to_string())?;
    let _ = state.store.set_active(&name);
    if let Ok(mut active) = state.active.lock() {
        *active = name;
    }
    with_runtime(&state, |r| r.update_settings(settings.clone()));
    Ok(settings)
}

/// Compute a tolerance that matches the `on` color but not the `off` sample (auto-calibration).
#[tauri::command]
#[specta::specta]
fn suggest_tolerance(on: Rgb, off: Rgb) -> Tolerance {
    Tolerance::separating(on, off)
}

#[tauri::command]
#[specta::specta]
fn pick_pixel(state: State<'_, AppState>) -> Option<PixelPick> {
    let guard = state.runtime.lock().ok()?;
    guard.as_ref()?.pick_pixel()
}

#[tauri::command]
#[specta::specta]
fn sample_capture(state: State<'_, AppState>) -> Vec<CaptureSampleDto> {
    state
        .runtime
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|r| r.sample_capture()))
        .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
fn recent_injections(state: State<'_, AppState>) -> Vec<InjectionDto> {
    state
        .runtime
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|r| r.recent_injections()))
        .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
fn get_metrics(state: State<'_, AppState>) -> Metrics {
    state
        .runtime
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|r| r.metrics()))
        .unwrap_or_default()
}

#[tauri::command]
#[specta::specta]
fn arm(state: State<'_, AppState>) {
    with_runtime(&state, |r| r.arm());
}

#[tauri::command]
#[specta::specta]
fn disarm(state: State<'_, AppState>) {
    with_runtime(&state, |r| r.disarm());
}

#[tauri::command]
#[specta::specta]
fn toggle_armed(state: State<'_, AppState>) {
    with_runtime(&state, |r| r.toggle_armed());
}

#[tauri::command]
#[specta::specta]
fn reload_script(state: State<'_, AppState>) {
    with_runtime(&state, |r| r.reload_script());
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .dangerously_cast_bigints_to_number()
        .commands(collect_commands![
            get_status,
            get_config,
            update_config,
            list_profiles,
            active_profile,
            switch_profile,
            save_profile_as,
            delete_profile,
            export_profile,
            import_profile,
            suggest_tolerance,
            pick_pixel,
            sample_capture,
            recent_injections,
            get_metrics,
            arm,
            disarm,
            toggle_armed,
            reload_script,
        ])
        .events(collect_events![StatusEvent, LogEvent])
}

fn main() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/bindings.ts"),
        )
        .expect("failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(builder.invoke_handler())

        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {

            builder.mount_events(app);
            tray::build(app.handle())?;

            let store = Store::new(Store::default_dir());
            let (active_name, settings) = store.load_active_or_init();

            let handle = app.handle().clone();
            let runtime = Runtime::start(settings, move |update| match update {
                RuntimeUpdate::Status(status) => {

                    #[cfg(debug_assertions)]
                    eprintln!(
                        "[status] armed={} focused={} ground={} ult={} x1={} x2={} skill={} res={:?}",
                        status.armed,
                        status.target_focused,
                        status.game_on_ground,
                        status.game_ultimate_ready,
                        status.x1_held,
                        status.x2_held,
                        status.skill_enabled,
                        status.resolution,
                    );
                    tray::update(&handle, status.armed, status.target_focused);
                    let _ = StatusEvent(status).emit(&handle);
                }
                RuntimeUpdate::Log(log) => {
                    #[cfg(debug_assertions)]
                    eprintln!("[{:?}] {}", log.kind, log.message);
                    let _ = LogEvent(log).emit(&handle);
                }
            });

            app.manage(AppState {
                runtime: Mutex::new(Some(runtime)),
                store,
                active: Mutex::new(active_name),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running VBL Pro 2");
}

#[cfg(test)]
mod tests {

    #[test]
    fn export_bindings() {
        super::specta_builder()
            .export(
                specta_typescript::Typescript::default(),
                concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/bindings.ts"),
            )
            .expect("failed to export typescript bindings");
    }
}

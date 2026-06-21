#[cfg(windows)]
fn main() {
    use vbl_engine::{Runtime, RuntimeUpdate, Store};

    let store = Store::new(Store::default_dir());
    let (profile, settings) = store.load_active_or_init();

    println!("VBL Pro 2 — headless runner.");
    println!("Loaded profile '{profile}'; engine starting (armed).");
    println!(
        "Bring Roblox to the foreground; Mouse Back/Forward drive the macros, F1/F2 are hotkeys."
    );
    println!("Press Enter to quit.\n");

    let runtime = Runtime::start(settings, |update| match update {
        RuntimeUpdate::Status(s) => println!(
            "[status] armed={} focused={} ground={} ult={} x1={} x2={} skill={}",
            s.armed,
            s.target_focused,
            s.game_on_ground,
            s.game_ultimate_ready,
            s.x1_held,
            s.x2_held,
            s.skill_enabled,
        ),
        RuntimeUpdate::Log(l) => println!("[log] {}", l.message),
    });
    runtime.arm();

    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);

    runtime.shutdown();
    println!("Stopped.");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("vbl-cli runs on Windows only (it drives Win32 input/capture).");
}

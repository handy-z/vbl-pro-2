# 🏐 VBL Pro 2

[![CI](https://github.com/handy-z/vbl-pro-2/actions/workflows/ci.yml/badge.svg)](https://github.com/handy-z/vbl-pro-2/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/handy-z/vbl-pro-2?label=download)](https://github.com/handy-z/vbl-pro-2/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)](#-getting-started)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](#-contributing)

A Windows macro and overlay tool for the Roblox game **Volleyball Legends**. It draws a
click-through crosshair, reads the match state from a few screen pixels, and plays precise
keyboard/mouse macros for you — only while Roblox is focused. Use the built-in macros, tweak them
with a no-code rule editor, or write your own in Luau.

<!-- Add a screenshot or short GIF of the app here. -->

> ⚠️ Automating inputs may violate a game's or platform's terms of service. Use at your own risk.

---

## 📋 Table of Contents

- [Features](#-features)
- [Tech Stack](#%EF%B8%8F-tech-stack)
- [Installation](#-installation)
- [Getting Started](#-getting-started)
  - [Prerequisites](#prerequisites)
  - [Run from source](#run-from-source)
- [Usage](#-usage)
  - [Quick start](#quick-start)
  - [Controls](#controls)
  - [What each macro does](#what-each-macro-does)
  - [Profiles & calibration](#profiles--calibration)
  - [Safety & tray](#safety--tray)
- [Customizing Macros](#-customizing-macros)
  - [Declarative rules (JSON)](#declarative-rules-json)
  - [Luau scripts](#luau-scripts)
- [For Developers](#-for-developers)
- [Running Tests](#-running-tests)
- [Contributing](#-contributing)
- [License](#-license)
- [Contact](#%EF%B8%8F-contact)

---

## ✨ Features

- **Built-in Volleyball Legends macros** – jump-set, spam/spike loop, respawn, and ultimate
  toggle, with *Normal* and *Boomjump* timing modes.
- **Programmable** – customize behavior with a visual rule editor, raw JSON, or sandboxed Luau
  scripts; all three run on the same engine.
- **Resolution-independent capture** – game state is read from normalized pixel points, so it
  works at any resolution after a quick calibration (not just a couple of presets).
- **Low-latency** – GPU-path screen capture (DXGI, with a GDI fallback) and a high-resolution
  scheduler instead of OS sleeps.
- **Crisp overlay** – a click-through, top-most crosshair drawn with Direct2D (GDI fallback).
- **Safe by design** – three-gate activation (armed + focused + enabled), a global kill switch,
  an optional unfocused failsafe, and guaranteed input release on every exit path.
- **Profiles** – save, switch, and delete setups; settings persist and previous-version configs
  are imported automatically.
- **Live monitor** – watch the exact inputs being sent, with capture-latency stats.

---

## 🛠️ Tech Stack

- **Core / Backend:** [Rust](https://www.rust-lang.org/) (a Cargo workspace; pure, testable engine)
- **Desktop shell:** [Tauri 2](https://tauri.app/)
- **Frontend:** [React 19](https://react.dev/), TypeScript, [Vite](https://vitejs.dev/),
  [Bun](https://bun.sh/)
- **Windows APIs:** the [`windows`](https://crates.io/crates/windows) crate (DXGI, Direct2D, GDI,
  low-level hooks, `SendInput`)
- **Scripting:** [Luau](https://luau-lang.org/) via [`mlua`](https://crates.io/crates/mlua)
- **Type-safe IPC:** [`tauri-specta`](https://crates.io/crates/tauri-specta) (Rust → TypeScript
  bindings)
- **DevOps:** GitHub Actions, a Rust `xtask` for build/release

---

## 📥 Installation

Most players don't need to build anything — grab the latest build from the
**[Releases page](https://github.com/handy-z/vbl-pro-2/releases/latest)**:

1. Open the **[latest release](https://github.com/handy-z/vbl-pro-2/releases/latest)**.
2. Under **Assets**, download the Windows installer (e.g. `VBL.Pro.2_x.y.z_x64-setup.exe`).
3. Run the installer, then launch **VBL Pro 2** from the Start menu.

If a release isn't code-signed, Windows SmartScreen may warn you on first launch — click
**More info → Run anyway**.

Then head to [Usage](#-usage). Want to build it yourself instead?
See [Run from source](#run-from-source).

---

## 🚀 Getting Started

### Prerequisites

- Windows 10 or 11 (the app is Windows-only)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Bun](https://bun.sh/)
- Git
- Optional: the [Tauri CLI](https://tauri.app/) (`cargo install tauri-cli`) to build an installer

### Run from source

```bash
# 1. Clone
git clone https://github.com/handy-z/vbl-pro-2.git
cd vbl-pro-2

# 2. Install UI dependencies
bun install --cwd app/ui

# 3a. Start the UI dev server (serves http://127.0.0.1:1420)
bun run --cwd app/ui dev

# 3b. In a second terminal, run the desktop app
cargo run --manifest-path app/src-tauri/Cargo.toml
```

With the Tauri CLI installed, steps 3a/3b collapse into one: `cargo tauri dev`.

To run the engine headless (no UI; arms immediately, Enter to quit):

```bash
cargo run -p vbl-cli
```

---

## 🎮 Usage

### Quick start

1. Launch **VBL Pro 2** and start **Volleyball Legends**.
2. Click **Arm** (top-right) — or right-click the tray icon → **Toggle armed**.
3. Bring Roblox to the foreground. The crosshair appears and macros go live.

> [!NOTE]
> Macros run only when **all three** are true: the app is **armed**, **Roblox is focused**, and
> **macros are enabled** (Config tab). Lose focus and everything pauses; refocus to resume.

### Controls

Mouse side buttons drive the main macros; a few keys are shortcuts — all rebindable in the
**Config** tab.

| Input | Default | What it does |
|:------|:-------:|:-------------|
| 🖱️ Mouse&nbsp;Back&nbsp;(X1) | — | **Jump-set** — hop and hold the set key while held |
| 🖱️ Mouse&nbsp;Forward&nbsp;(X2) | — | **Spam/spike loop** while held; **spike** on release |
| ⌨️ Respawn | <kbd>F1</kbd> | Quick respawn sequence |
| ⌨️ Toggle&nbsp;ultimate | <kbd>F2</kbd> | Turn the ultimate behavior on/off |
| 🛑 Kill&nbsp;switch | <kbd>F8</kbd> | Instantly disarm and release everything, anywhere |

> [!TIP]
> The keys these macros press and the **tap hold time** (default `35&nbsp;ms`) live in the Config
> tab. Two skill modes — **Normal** and **Boomjump** — change a couple of the sequences below.

### What each macro does

> Defaults: **set key** = <kbd>E</kbd> · **skill key** = <kbd>Left Ctrl</kbd>

<details open>
<summary>🖱️ <b>Mouse Back (X1) — jump-set</b></summary>

| Situation | Sequence |
|:----------|:---------|
| On the ground, Mouse Forward **not** held | tap <kbd>Space</kbd> → **hold set key** until release |
| Airborne, or Mouse Forward held | wait (`100 ms` Boomjump + ultimate, else `25 ms`) → **hold set key** until release |

Releasing Mouse Back releases the set key.

</details>

<details open>
<summary>🖱️ <b>Mouse Forward (X2) — spam loop + spike</b></summary>

While held and on the ground, it repeats:

| Mode | Loop |
|:-----|:-----|
| Normal, or ultimate not ready | <kbd>Shift</kbd> → <kbd>Space</kbd> → <kbd>Shift</kbd> |
| Boomjump + ultimate ready | <kbd>Shift</kbd> → skill key → pause → <kbd>Shift</kbd> |

**On release — the spike:** if the ultimate is ready and you're airborne, it taps the skill key
(Normal) or waits briefly (Boomjump), then **left-clicks**.

</details>

<details>
<summary>⌨️ <b>Respawn · Toggle ultimate · Kill switch</b></summary>

- **Respawn** (<kbd>F1</kbd>) — tap <kbd>Esc</kbd> → <kbd>R</kbd> → <kbd>Enter</kbd>.
- **Toggle ultimate** (<kbd>F2</kbd>) — flips ultimate behavior; while off, the loop and spike use
  their non-ultimate variants.
- **Kill switch** (<kbd>F8</kbd>) — disarms and releases everything immediately, even when Roblox
  isn't focused.

</details>

> [!IMPORTANT]
> Every sequence is gated: disarming, losing focus, or disabling macros mid-action stops it and
> releases anything held — so no key is ever left stuck.

### Profiles & calibration

A **profile** bundles everything — skill mode, keybinds, crosshair, capture points, and the macro
program — so sharing a profile shares the whole setup. On the **Profiles** tab you can save under a
new name, switch, or delete; changes save automatically.

If a state isn't detected on your display, recalibrate it on the **Capture** tab:

1. Get into the state in-game (e.g. stand on the ground).
2. Hover the cursor over the on-screen element.
3. Click **Calibrate** — a short countdown samples the pixel under your cursor.
4. Watch the **Match** badge confirm it flips correctly.

> [!TIP]
> Capture points are stored as fractions of the game window, so they adapt to any resolution.

### Safety & tray

The **tray icon** shows status at a glance:

| Tray | Meaning |
|:----:|:--------|
| 🟢 | Armed and Roblox focused — active |
| 🟡 | Armed, waiting for Roblox focus |
| ⚪ | Idle / disarmed |

- **Kill switch** (<kbd>F8</kbd>) — global disarm + release of all held input.
- **Unfocused failsafe** (optional) — auto-disarm if Roblox isn't focused for N milliseconds.
- Held input is always released on disarm, profile switch, focus loss, or quit.
- Right-click the tray for **Show / Toggle armed / Quit**; closing the window hides it to the tray.

---

## 🧩 Customizing Macros

A profile can drive input three ways, in order of precedence: a **Luau script**, then a
**declarative JSON program**, then the **built-in** behavior. Both authoring layers compile to the
same action stream and obey the same safety gates — a macro can never fire while disarmed,
unfocused, or disabled, and all timing runs on the high-resolution scheduler.

Triggers you can handle:

| Trigger | Fires when |
|---------|-----------|
| `X1.down` / `X1.up` | Mouse Back pressed / released |
| `X2.down` / `X2.up` | Mouse Forward pressed / released |
| `<key>.down` | a key is pressed, e.g. `f1.down` |
| `X2Held.held` | repeatedly, while the `X2Held` state is set (a loop) |

States: `GameOnGround`, `GameUltimateReady`, `X1Held`, `X2Held`, `skillEnabled`, `robloxFocused`.

### Declarative rules (JSON)

Each rule is `on` a trigger, `when` a guard holds, `do` a sequence. Edit visually on the Macros
tab, or as raw JSON:

```jsonc
{
  "macros": [
    { "on": "respawn",
      "do": [ {"tap": "escape"}, {"tap": "r"}, {"tap": "enter"} ] },

    { "on": "X1.down",
      "when": { "GameOnGround": true, "X2Held": false },
      "do": [ {"tap": "space"}, {"hold": "$jumpset_key", "until": "X1.up"} ] },

    { "on": "X2Held.held",
      "while": { "GameOnGround": true },
      "do": [ {"tap": "shift"}, {"tap": "space"}, {"tap": "shift"} ] }
  ]
}
```

Actions: `tap` (key, optional `hold_ms`), `hold`, `down`, `up`, `click` (button, optional
`hold_ms`), `wait` (ms), `release_all`, `toggle`, `set_state` (+ `value`), `log`, and `if`
(`then` / `else`). Conditions combine with `all`, `any`, `not`, `eq`, or an inline
`{ "State": true }` map. `$jumpset_key`, `$skill_key`, `$skill`, etc. resolve to the profile's
settings.

### Luau scripts

For logic the rules can't express, write a script. It runs sandboxed (no file or network access)
and under an instruction budget, so a runaway loop can't hang the engine.

```lua
vbl.on("X1.down", function()
  if vbl.state.GameOnGround and not vbl.state.X2Held then
    vbl.tap("space")
    vbl.down(vbl.settings.jumpset_key)
  else
    vbl.wait(25)
    vbl.down(vbl.settings.jumpset_key)
  end
end)

vbl.on("X1.up", function()
  vbl.up(vbl.settings.jumpset_key)
end)

vbl.every("X2Held", function()
  vbl.tap("shift"); vbl.tap("space"); vbl.tap("shift")
  vbl.wait(1)
end)
```

API: `vbl.on(trigger, fn)`, `vbl.every(stateName, fn)`, read-only `vbl.state.<Name>` /
`vbl.settings.<name>`, and actions `vbl.tap(key, hold_ms?)`, `vbl.down(key)`, `vbl.up(key)`,
`vbl.click(button?, hold_ms?)`, `vbl.wait(ms)`, `vbl.release_all()`, `vbl.toggle(state)`,
`vbl.set_state(state, value)`, `vbl.log(msg)`.

---

## 🏗️ For Developers

The macro engine (`vbl-core`) is pure: no OS calls and no `unsafe`. Time and all I/O sit behind
traits (`Clock`, `InputSink`, `ScreenCapture`, `WindowTracker`, `Overlay`), so the whole engine is
driven on a virtual clock in tests and asserts the exact input it would emit — instantly and
deterministically. One runtime thread owns the executor and feeds it from the input hook,
capture/focus polling, commands, and timed wake-ups.

```
crates/
  vbl-core/          engine: state, executor, built-in profile, DSL, persistence
  vbl-ipc/           serde types shared with the UI
  vbl-engine/        runtime orchestration + profile driver
  vbl-scripting/     embedded Luau host
  vbl-platform-win/  Windows backends: capture, overlay, input, hooks, clock
  vbl-cli/           headless runner
app/
  src-tauri/         Tauri shell (commands, events, tray)
  ui/                React + Vite frontend
xtask/               build/release tasks
```

Commands and events are defined once in Rust; the TypeScript bindings in
`app/ui/src/bindings.ts` are generated from them — run `cargo xtask codegen` after changing the
IPC surface (CI fails if the committed bindings drift).

**Build & release** via `xtask`:

```bash
cargo xtask codegen            # regenerate TS bindings
cargo xtask build              # UI + app + installer (needs the Tauri CLI)
cargo xtask build --exe-only   # skip the installer
cargo xtask version --set X.Y.Z
cargo xtask release            # build, collect artifacts, write the update manifest
```

---

## 🧪 Running Tests

```bash
cargo test                                   # unit, property, and golden-trace tests
cargo clippy --all-targets -- -D warnings    # lints
cargo fmt --all -- --check                   # formatting
bun run --cwd app/ui build                   # frontend typecheck + build

# Proves the engine has no OS dependency (pure core):
cargo build -p vbl-core --target x86_64-unknown-linux-gnu
```

CI runs the same checks on Linux (core) and Windows (full suite + frontend).

---

## 🤝 Contributing

Contributions are welcome.

1. Fork the project
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your change — keep it green:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
   If you touched the IPC surface, run `cargo xtask codegen` and commit the updated bindings.
4. Commit and push your branch
5. Open a pull request

---

## 📄 License

Distributed under the MIT License. See [`LICENSE`](LICENSE) for details.

---

## ✉️ Contact

Maintained by **handy-z**.

Project link: https://github.com/handy-z/vbl-pro-2

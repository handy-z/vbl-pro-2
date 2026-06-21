import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import {
  activeProfile,
  arm,
  deleteProfile,
  disarm,
  exportProfile,
  getConfig,
  getStatus,
  importProfile,
  listProfiles,
  onLog,
  onStatus,
  reloadScript,
  saveProfileAs,
  switchProfile,
  updateConfig,
} from "./api";
import { Capture } from "./components/Capture";
import { Config } from "./components/Config";
import { Dashboard } from "./components/Dashboard";
import { Logs } from "./components/Logs";
import { Macros } from "./components/Macros";
import { Monitor } from "./components/Monitor";
import { Profiles } from "./components/Profiles";
import { ResizeHandles, WindowControls } from "./components/WindowControls";
import { DEFAULT_STATUS, type LogLine, type RuntimeStatus, type VblSettings } from "./types";

type Tab = "dashboard" | "config" | "capture" | "macros" | "profiles" | "monitor" | "logs";
const TABS: Tab[] = ["dashboard", "config", "capture", "macros", "profiles", "monitor", "logs"];

export function App() {
  const [status, setStatus] = useState<RuntimeStatus>(DEFAULT_STATUS);
  const [config, setConfig] = useState<VblSettings | null>(null);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [profiles, setProfiles] = useState<string[]>([]);
  const [active, setActive] = useState<string>("");
  const [tab, setTab] = useState<Tab>("dashboard");

  const refreshProfiles = () => {
    listProfiles()
      .then(setProfiles)
      .catch(() => {});
    activeProfile()
      .then(setActive)
      .catch(() => {});
  };

  useEffect(() => {
    let mounted = true;
    getStatus()
      .then((s) => mounted && setStatus(s))
      .catch(() => {});
    getConfig()
      .then((c) => mounted && setConfig(c))
      .catch(() => {});
    refreshProfiles();
    const unStatus = onStatus(setStatus);
    const unLog = onLog((l) => setLogs((prev) => [...prev, l].slice(-200)));
    return () => {
      mounted = false;
      unStatus.then((f) => f());
      unLog.then((f) => f());
    };
  }, []);

  const applyConfig = (next: VblSettings) => {
    setConfig(next);
    updateConfig(next).catch(() => {});
  };

  const handleSwitch = (name: string) => {
    switchProfile(name)
      .then((s) => {
        if (s) setConfig(s);
        setActive(name);
        refreshProfiles();
      })
      .catch(() => {});
  };

  const handleSaveAs = (name: string) => {
    saveProfileAs(name)
      .then(() => {
        setActive(name);
        refreshProfiles();
      })
      .catch(() => {});
  };

  const handleDelete = (name: string) => {
    deleteProfile(name)
      .then((next) => {
        if (next) setConfig(next);
        refreshProfiles();
      })
      .catch(() => {});
  };

  const checkForUpdates = async () => {
    try {
      const update = await check();
      if (!update) {
        alert("You're on the latest version.");
        return;
      }
      if (confirm(`Update ${update.version} is available. Install and restart now?`)) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch {
      alert("Update check failed (no published release or offline).");
    }
  };

  const handleExport = async () => {
    const path = await save({
      defaultPath: `${active || "profile"}.json`,
      filters: [{ name: "VBL profile", extensions: ["json"] }],
    }).catch(() => null);
    if (path) await exportProfile(path).catch(() => {});
  };

  const handleImport = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "VBL profile", extensions: ["json"] }],
    }).catch(() => null);
    if (typeof path === "string") {
      const settings = await importProfile(path).catch(() => null);
      if (settings) {
        setConfig(settings);
        refreshProfiles();
      }
    }
  };

  const loading = (
    <section className="card">
      <div className="meta">Loading…</div>
    </section>
  );

  return (
    <main className="app">
      <ResizeHandles />
      <header className="topbar" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          VBL <span>Pro 2</span>
        </div>
        <nav className="tabs">
          {TABS.map((t) => (
            <button key={t} className={tab === t ? "active" : ""} onClick={() => setTab(t)}>
              {t}
            </button>
          ))}
        </nav>
        <div className="topright">
          <button
            className={`arm ${status.armed ? "armed" : ""}`}
            onClick={() => (status.armed ? disarm() : arm())}
          >
            {status.armed ? "Disarm" : "Arm"}
          </button>
          <WindowControls />
        </div>
      </header>

      {tab === "dashboard" && <Dashboard status={status} />}
      {tab === "config" && (config ? <Config config={config} onChange={applyConfig} /> : loading)}
      {tab === "capture" &&
        (config ? (
          <Capture config={config} onChange={applyConfig} resolution={status.resolution} />
        ) : (
          loading
        ))}
      {tab === "macros" &&
        (config ? (
          <Macros
            config={config}
            onChange={applyConfig}
            onReload={() => reloadScript().catch(() => {})}
          />
        ) : (
          loading
        ))}
      {tab === "profiles" && (
        <Profiles
          profiles={profiles}
          active={active}
          onSwitch={handleSwitch}
          onSaveAs={handleSaveAs}
          onDelete={handleDelete}
          onExport={handleExport}
          onImport={handleImport}
        />
      )}
      {tab === "monitor" && <Monitor />}
      {tab === "logs" && <Logs logs={logs} />}

      <footer className="hint">
        <span>
          Bring Roblox to the foreground, then use Mouse Back / Forward. F1 = reset, F2 = toggle
          ultimate.
        </span>
        <button className="link" onClick={checkForUpdates}>
          Check for updates
        </button>
      </footer>
    </main>
  );
}

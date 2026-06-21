import type { CrosshairConfig, MacroKeybinds, SkillMode, VblSettings } from "../types";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="row">
      <span>{label}</span>
      {children}
    </label>
  );
}

export function Config({
  config,
  onChange,
}: {
  config: VblSettings;
  onChange: (next: VblSettings) => void;
}) {
  const set = (patch: Partial<VblSettings>) => onChange({ ...config, ...patch });
  const setMacro = (patch: Partial<MacroKeybinds>) =>
    set({ macroKeys: { ...config.macroKeys, ...patch } });
  const setCross = (patch: Partial<CrosshairConfig>) =>
    set({ crosshair: { ...config.crosshair, ...patch } });

  return (
    <div className="panels">
      <section className="card">
        <h2>Macro</h2>
        <Row label="Mode">
          <div className="seg">
            {(["normal", "boomjump"] as SkillMode[]).map((mode) => (
              <button
                key={mode}
                className={config.skill === mode ? "active" : ""}
                onClick={() => set({ skill: mode })}
              >
                {mode}
              </button>
            ))}
          </div>
        </Row>
        <Row label="Macros enabled">
          <input
            type="checkbox"
            checked={config.macroKeys.enabled}
            onChange={(e) => setMacro({ enabled: e.target.checked })}
          />
        </Row>
        <Row label="Jumpset key">
          <input
            value={config.macroKeys.jumpsetKey}
            onChange={(e) => setMacro({ jumpsetKey: e.target.value })}
          />
        </Row>
        <Row label="Skill key">
          <input
            value={config.macroKeys.skillKey}
            onChange={(e) => setMacro({ skillKey: e.target.value })}
          />
        </Row>
        <Row label="Respawn key">
          <input
            value={config.macroKeys.respawnKey}
            onChange={(e) => setMacro({ respawnKey: e.target.value })}
          />
        </Row>
        <Row label="Toggle-ultimate key">
          <input
            value={config.macroKeys.toggleUltimateKey}
            onChange={(e) => setMacro({ toggleUltimateKey: e.target.value })}
          />
        </Row>
        <Row label="Kill switch key">
          <input
            value={config.macroKeys.killSwitchKey}
            placeholder="blank = off"
            onChange={(e) => setMacro({ killSwitchKey: e.target.value })}
          />
        </Row>
        <Row label="Tap hold (ms)">
          <input
            type="number"
            min={1}
            max={500}
            value={config.tapMs}
            onChange={(e) => set({ tapMs: Number(e.target.value) })}
          />
        </Row>
        <Row label="Unfocused failsafe (ms)">
          <input
            type="number"
            min={0}
            placeholder="blank = off"
            value={config.unfocusedPanicMs ?? ""}
            onChange={(e) =>
              set({ unfocusedPanicMs: e.target.value === "" ? null : Number(e.target.value) })
            }
          />
        </Row>
      </section>

      <section className="card">
        <h2>Crosshair</h2>
        <Row label="Enabled">
          <input
            type="checkbox"
            checked={config.crosshair.enabled}
            onChange={(e) => setCross({ enabled: e.target.checked })}
          />
        </Row>
        <Row label="Color">
          <input
            type="color"
            value={config.crosshair.color}
            onChange={(e) => setCross({ color: e.target.value })}
          />
        </Row>
        <Row label="Scale">
          <input
            type="number"
            min={0.2}
            max={4}
            step={0.1}
            value={config.crosshair.scale ?? 1}
            onChange={(e) => setCross({ scale: Number(e.target.value) })}
          />
        </Row>
        <Row label="Opacity">
          <input
            type="number"
            min={0}
            max={1}
            step={0.05}
            value={config.crosshair.opacity ?? 1}
            onChange={(e) => setCross({ opacity: Number(e.target.value) })}
          />
        </Row>
        <Row label="Offset X">
          <input
            type="number"
            value={config.crosshair.offset.x}
            onChange={(e) =>
              setCross({ offset: { ...config.crosshair.offset, x: Number(e.target.value) } })
            }
          />
        </Row>
        <Row label="Offset Y">
          <input
            type="number"
            value={config.crosshair.offset.y}
            onChange={(e) =>
              setCross({ offset: { ...config.crosshair.offset, y: Number(e.target.value) } })
            }
          />
        </Row>
      </section>
    </div>
  );
}

import { useEffect, useState } from "react";
import type { VblSettings } from "../types";
import { DslEditor } from "./DslEditor";

export function Macros({
  config,
  onChange,
  onReload,
}: {
  config: VblSettings;
  onChange: (next: VblSettings) => void;
  onReload: () => void;
}) {
  const [script, setScript] = useState(config.script ?? "");
  const [dsl, setDsl] = useState(config.dsl ?? "");
  const [dslMode, setDslMode] = useState<"visual" | "json">("visual");

  useEffect(() => {
    setScript(config.script ?? "");
    setDsl(config.dsl ?? "");
  }, [config.script, config.dsl]);

  const dirty = (config.script ?? "") !== script || (config.dsl ?? "") !== dsl;
  const active =
    script.trim().length > 0
      ? "Luau script"
      : dsl.trim().length > 0
        ? "DSL program"
        : "Built-in profile";

  const apply = () =>
    onChange({
      ...config,
      script: script.trim().length > 0 ? script : null,
      dsl: dsl.trim().length > 0 ? dsl : null,
    });

  return (
    <div className="panels">
      <section className="card">
        <div className="card-head">
          <h2>Macro program</h2>
          <span className="badge ok">{active}</span>
        </div>
        <div className="meta">
          Precedence: a non-empty <b>Luau script</b> wins, else the <b>DSL program</b>, else the
          built-in VBL behavior. Edit, then <b>Apply</b> to compile &amp; hot-swap; <b>Reload</b>{" "}
          re-runs the saved program. Both lower onto the same executor + safety gating.
        </div>

        <h3 className="sub">Luau script — layer 2</h3>
        <textarea
          className="code"
          spellCheck={false}
          rows={10}
          value={script}
          placeholder={'vbl.on("X1.down", function()\n  vbl.tap("space")\nend)'}
          onChange={(e) => setScript(e.target.value)}
        />

        <div className="sub-head">
          <h3 className="sub">Declarative DSL — layer 1</h3>
          <div className="seg small">
            {(["visual", "json"] as const).map((m) => (
              <button
                key={m}
                className={dslMode === m ? "active" : ""}
                onClick={() => setDslMode(m)}
              >
                {m}
              </button>
            ))}
          </div>
        </div>
        {dslMode === "visual" ? (
          <DslEditor value={dsl} onChange={setDsl} />
        ) : (
          <textarea
            className="code"
            spellCheck={false}
            rows={10}
            value={dsl}
            placeholder={'{ "macros": [\n  { "on": "respawn", "do": [ {"tap":"escape"} ] }\n] }'}
            onChange={(e) => setDsl(e.target.value)}
          />
        )}

        <div className="actions">
          <button className="primary" onClick={apply} disabled={!dirty}>
            {dirty ? "Apply" : "Applied"}
          </button>
          <button onClick={onReload}>Reload</button>
        </div>
      </section>
    </div>
  );
}

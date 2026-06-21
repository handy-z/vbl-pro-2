const TRIGGERS = ["X1.down", "X1.up", "X2.down", "X2.up", "X2Held.held", "respawn", "ult"];
const STATES = [
  "GameOnGround",
  "GameUltimateReady",
  "X1Held",
  "X2Held",
  "skillEnabled",
  "robloxFocused",
];
const KEYS = ["space", "e", "r", "enter", "escape", "shift", "lctrl", "f1", "f2", "$jumpset_key", "$skill_key"];
const BUTTONS = ["left", "right", "middle", "x1", "x2"];
const ACTION_KINDS = [
  "tap",
  "hold",
  "click",
  "down",
  "up",
  "wait",
  "toggle",
  "set_state",
  "release_all",
  "log",
] as const;

type ActionKind = (typeof ACTION_KINDS)[number];
type Action = {
  kind: ActionKind;
  key?: string;
  button?: string;
  ms?: number;
  holdMs?: number;
  state?: string;
  value?: boolean;
  message?: string;
};
type Flag = { state: string; value: boolean };
type Rule = { on: string; when: Flag[]; actions: Action[] };

type Parsed = { rules: Rule[]; supported: boolean };

function parse(json: string): Parsed {
  if (!json.trim()) return { rules: [], supported: true };
  let obj: unknown;
  try {
    obj = JSON.parse(json);
  } catch {
    return { rules: [], supported: false };
  }
  const macros = (obj as { macros?: unknown[] })?.macros;
  if (!Array.isArray(macros)) return { rules: [], supported: false };

  let supported = true;
  const rules: Rule[] = [];
  for (const m of macros as Record<string, unknown>[]) {
    const when: Flag[] = [];
    const w = m.when;
    if (w !== undefined) {
      if (w && typeof w === "object" && !Array.isArray(w) && !hasCombinator(w)) {
        for (const [state, value] of Object.entries(w)) when.push({ state, value: Boolean(value) });
      } else {
        supported = false;
      }
    }
    if (m.while !== undefined) supported = false;
    const actions: Action[] = [];
    for (const a of (Array.isArray(m.do) ? m.do : []) as Record<string, unknown>[]) {
      const parsed = parseAction(a);
      if (parsed) actions.push(parsed);
      else supported = false;
    }
    rules.push({ on: String(m.on ?? ""), when, actions });
  }
  return { rules, supported };
}

function hasCombinator(w: object): boolean {
  return ["all", "any", "not", "eq"].some((k) => k in w);
}

function parseAction(a: Record<string, unknown>): Action | null {
  if ("tap" in a) return { kind: "tap", key: String(a.tap), holdMs: numOrUndef(a.hold_ms) };
  if ("hold" in a) return { kind: "hold", key: String(a.hold) };
  if ("click" in a) return { kind: "click", button: String(a.click), holdMs: numOrUndef(a.hold_ms) };
  if ("down" in a) return { kind: "down", key: String(a.down) };
  if ("up" in a) return { kind: "up", key: String(a.up) };
  if ("release_all" in a) return { kind: "release_all" };
  if ("wait" in a) return { kind: "wait", ms: Number(a.wait) || 0 };
  if ("toggle" in a) return { kind: "toggle", state: String(a.toggle) };
  if ("set_state" in a) return { kind: "set_state", state: String(a.set_state), value: Boolean(a.value) };
  if ("log" in a) return { kind: "log", message: String(a.log) };
  return null;
}

function numOrUndef(v: unknown): number | undefined {
  return typeof v === "number" ? v : undefined;
}

function serialize(rules: Rule[]): string {
  const macros = rules.map((r) => {
    const out: Record<string, unknown> = { on: r.on };
    if (r.when.length) out.when = Object.fromEntries(r.when.map((f) => [f.state, f.value]));
    out.do = r.actions.map(actionJson);
    return out;
  });
  return JSON.stringify({ macros }, null, 2);
}

function actionJson(a: Action): Record<string, unknown> {
  switch (a.kind) {
    case "tap":
      return a.holdMs != null ? { tap: a.key, hold_ms: a.holdMs } : { tap: a.key ?? "" };
    case "hold":
      return { hold: a.key ?? "" };
    case "click":
      return a.holdMs != null ? { click: a.button, hold_ms: a.holdMs } : { click: a.button ?? "left" };
    case "down":
      return { down: a.key ?? "" };
    case "up":
      return { up: a.key ?? "" };
    case "wait":
      return { wait: a.ms ?? 0 };
    case "toggle":
      return { toggle: a.state ?? "" };
    case "set_state":
      return { set_state: a.state ?? "", value: a.value ?? false };
    case "release_all":
      return { release_all: true };
    case "log":
      return { log: a.message ?? "" };
  }
}

export function DslEditor({ value, onChange }: { value: string; onChange: (json: string) => void }) {
  const { rules, supported } = parse(value);

  const commit = (next: Rule[]) => onChange(serialize(next));
  const patchRule = (i: number, patch: Partial<Rule>) =>
    commit(rules.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const patchAction = (ri: number, ai: number, patch: Partial<Action>) =>
    patchRule(ri, {
      actions: rules[ri].actions.map((a, j) => (j === ai ? { ...a, ...patch } : a)),
    });

  if (!supported) {
    return (
      <div className="meta warn">
        This program uses advanced constructs (nested <code>if</code>, <code>all</code>/
        <code>any</code>/<code>not</code>/<code>eq</code>, or <code>while</code>) that the visual
        editor can't show. Switch to <b>JSON</b> to edit it.
      </div>
    );
  }

  return (
    <div className="dsl">
      {rules.map((rule, ri) => (
        <div className="rule" key={ri}>
          <div className="rule-head">
            <label className="when-row">
              <span>on</span>
              <input
                list="dsl-triggers"
                value={rule.on}
                onChange={(e) => patchRule(ri, { on: e.target.value })}
              />
            </label>
            <button className="mini danger" onClick={() => commit(rules.filter((_, j) => j !== ri))}>
              ✕ rule
            </button>
          </div>

          <div className="rule-sub">when (all true)</div>
          {rule.when.map((flag, wi) => (
            <div className="when-row" key={wi}>
              <input
                list="dsl-states"
                value={flag.state}
                onChange={(e) =>
                  patchRule(ri, {
                    when: rule.when.map((f, j) => (j === wi ? { ...f, state: e.target.value } : f)),
                  })
                }
              />
              <select
                value={String(flag.value)}
                onChange={(e) =>
                  patchRule(ri, {
                    when: rule.when.map((f, j) =>
                      j === wi ? { ...f, value: e.target.value === "true" } : f,
                    ),
                  })
                }
              >
                <option value="true">is true</option>
                <option value="false">is false</option>
              </select>
              <button
                className="mini danger"
                onClick={() => patchRule(ri, { when: rule.when.filter((_, j) => j !== wi) })}
              >
                ✕
              </button>
            </div>
          ))}
          <button
            className="mini"
            onClick={() => patchRule(ri, { when: [...rule.when, { state: STATES[0], value: true }] })}
          >
            + condition
          </button>

          <div className="rule-sub">do (in order)</div>
          {rule.actions.map((action, ai) => (
            <div className="act-row" key={ai}>
              <select
                value={action.kind}
                onChange={(e) => patchAction(ri, ai, { kind: e.target.value as ActionKind })}
              >
                {ACTION_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
              {actionFields(action, (patch) => patchAction(ri, ai, patch))}
              <button
                className="mini danger"
                onClick={() => patchRule(ri, { actions: rule.actions.filter((_, j) => j !== ai) })}
              >
                ✕
              </button>
            </div>
          ))}
          <button
            className="mini"
            onClick={() => patchRule(ri, { actions: [...rule.actions, { kind: "tap", key: "space" }] })}
          >
            + action
          </button>
        </div>
      ))}

      <button
        className="mini"
        onClick={() => commit([...rules, { on: TRIGGERS[0], when: [], actions: [] }])}
      >
        + rule
      </button>

      <datalist id="dsl-triggers">
        {TRIGGERS.map((t) => (
          <option key={t} value={t} />
        ))}
      </datalist>
      <datalist id="dsl-states">
        {STATES.map((s) => (
          <option key={s} value={s} />
        ))}
      </datalist>
      <datalist id="dsl-keys">
        {KEYS.map((k) => (
          <option key={k} value={k} />
        ))}
      </datalist>
    </div>
  );
}

function actionFields(a: Action, patch: (p: Partial<Action>) => void) {
  const keyInput = (
    <input
      list="dsl-keys"
      placeholder="key"
      value={a.key ?? ""}
      onChange={(e) => patch({ key: e.target.value })}
    />
  );
  const holdMs = (
    <input
      type="number"
      className="num"
      placeholder="hold ms"
      value={a.holdMs ?? ""}
      onChange={(e) => patch({ holdMs: e.target.value === "" ? undefined : Number(e.target.value) })}
    />
  );
  switch (a.kind) {
    case "tap":
      return (
        <>
          {keyInput}
          {holdMs}
        </>
      );
    case "hold":
    case "down":
    case "up":
      return keyInput;
    case "click":
      return (
        <>
          <select value={a.button ?? "left"} onChange={(e) => patch({ button: e.target.value })}>
            {BUTTONS.map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
          {holdMs}
        </>
      );
    case "wait":
      return (
        <input
          type="number"
          className="num"
          placeholder="ms"
          value={a.ms ?? 0}
          onChange={(e) => patch({ ms: Number(e.target.value) })}
        />
      );
    case "toggle":
      return (
        <input
          list="dsl-states"
          placeholder="state"
          value={a.state ?? ""}
          onChange={(e) => patch({ state: e.target.value })}
        />
      );
    case "set_state":
      return (
        <>
          <input
            list="dsl-states"
            placeholder="state"
            value={a.state ?? ""}
            onChange={(e) => patch({ state: e.target.value })}
          />
          <select value={String(a.value ?? false)} onChange={(e) => patch({ value: e.target.value === "true" })}>
            <option value="true">true</option>
            <option value="false">false</option>
          </select>
        </>
      );
    case "log":
      return (
        <input
          placeholder="message"
          value={a.message ?? ""}
          onChange={(e) => patch({ message: e.target.value })}
        />
      );
    case "release_all":
      return <span className="meta">(releases everything held)</span>;
  }
}

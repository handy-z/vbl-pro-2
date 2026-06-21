import { useState } from "react";

export function Profiles({
  profiles,
  active,
  onSwitch,
  onSaveAs,
  onDelete,
  onExport,
  onImport,
}: {
  profiles: string[];
  active: string;
  onSwitch: (name: string) => void;
  onSaveAs: (name: string) => void;
  onDelete: (name: string) => void;
  onExport: () => void;
  onImport: () => void;
}) {
  const [name, setName] = useState("");

  const save = () => {
    const trimmed = name.trim();
    if (trimmed) {
      onSaveAs(trimmed);
      setName("");
    }
  };

  return (
    <section className="card">
      <h2>Profiles</h2>
      <div className="meta">
        Settings are saved automatically to the active profile. Save the current settings under
        a new name, or switch between profiles.
      </div>

      <ul className="profiles">
        {profiles.length === 0 && <li className="meta">No profiles yet.</li>}
        {profiles.map((p) => (
          <li key={p} className={p === active ? "active" : ""}>
            <span className="name">{p}</span>
            <span className="row-actions">
              {p === active ? (
                <span className="badge ok">active</span>
              ) : (
                <button className="mini" onClick={() => onSwitch(p)}>
                  Switch
                </button>
              )}
              <button
                className="mini danger"
                title="Delete profile"
                onClick={() => {
                  if (confirm(`Delete profile "${p}"?`)) onDelete(p);
                }}
              >
                Delete
              </button>
            </span>
          </li>
        ))}
      </ul>

      <div className="saveas">
        <input
          placeholder="New profile name…"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && save()}
        />
        <button className="mini" onClick={save} disabled={!name.trim()}>
          Save as
        </button>
      </div>

      <div className="actions">
        <button onClick={onImport}>Import…</button>
        <button onClick={onExport}>Export…</button>
      </div>
    </section>
  );
}

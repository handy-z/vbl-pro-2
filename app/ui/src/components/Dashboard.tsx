import type { RuntimeStatus } from "../types";

function Pill({ label, on }: { label: string; on: boolean }) {
  return <div className={`pill ${on ? "on" : "off"}`}>{label}</div>;
}

export function Dashboard({ status }: { status: RuntimeStatus }) {
  return (
    <section className="card">
      <h2>Runtime state</h2>
      <div className="grid">
        <Pill label="Armed" on={status.armed} />
        <Pill label="Roblox focused" on={status.targetFocused} />
        <Pill label="On ground" on={status.gameOnGround} />
        <Pill label="Ultimate ready" on={status.gameUltimateReady} />
        <Pill label="Skill enabled" on={status.skillEnabled} />
        <Pill label="X1 held" on={status.x1Held} />
        <Pill label="X2 held" on={status.x2Held} />
      </div>
      <div className="meta">
        Resolution:{" "}
        {status.resolution ? `${status.resolution[0]}×${status.resolution[1]}` : "—"}
      </div>
    </section>
  );
}

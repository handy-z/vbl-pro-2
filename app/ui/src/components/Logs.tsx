import type { LogLine } from "../types";

function time(ms: number) {
  return new Date(ms).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function Logs({ logs }: { logs: LogLine[] }) {
  return (
    <section className="card">
      <h2>Logs</h2>
      {logs.length === 0 ? (
        <div className="meta">No activity yet.</div>
      ) : (
        <ul className="logs">
          {logs.map((l) => (
            <li key={l.id} className={l.kind}>
              <span className="ts">{time(l.timestampMs)}</span>
              <span className="msg">{l.message}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

import { useEffect, useRef, useState } from "react";
import { getMetrics, recentInjections } from "../api";
import type { InjectionDto, Metrics } from "../types";

const EMPTY_METRICS: Metrics = {
  captureMicros: 0,
  captureP50Micros: 0,
  captureP95Micros: 0,
  captureMaxMicros: 0,
  captureSamples: 0,
  injections: 0,
  pollCount: 0,
};

const SPARK_LEN = 80;

function time(ms: number) {
  return new Date(ms).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function Sparkline({ values }: { values: number[] }) {
  const w = 240;
  const h = 40;
  if (values.length < 2) {
    return <div className="meta">Collecting samples… (arm + focus Roblox)</div>;
  }
  const max = Math.max(...values, 1);
  const step = w / (SPARK_LEN - 1);
  const points = values
    .map((v, i) => {
      const x = (i + (SPARK_LEN - values.length)) * step;
      const y = h - (v / max) * (h - 2) - 1;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <svg className="spark" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none">
      <polyline points={points} fill="none" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

export function Monitor() {
  const [injections, setInjections] = useState<InjectionDto[]>([]);
  const [metrics, setMetrics] = useState<Metrics>(EMPTY_METRICS);
  const [spark, setSpark] = useState<number[]>([]);

  const lastSamples = useRef(0);

  useEffect(() => {
    let active = true;
    const tick = () => {
      recentInjections()
        .then((l) => active && setInjections(l))
        .catch(() => {});
      getMetrics()
        .then((m) => {
          if (!active) return;
          setMetrics(m);
          if (m.captureSamples !== lastSamples.current) {
            lastSamples.current = m.captureSamples;
            setSpark((prev) => [...prev, m.captureMicros].slice(-SPARK_LEN));
          }
        })
        .catch(() => {});
    };
    tick();
    const id = setInterval(tick, 250);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, []);

  const recent = [...injections].reverse();

  return (
    <section className="card">
      <h2>Monitor</h2>
      <div className="stats">
        <div className="stat">
          <span className="num">{metrics.injections}</span>
          <span className="lbl">injections</span>
        </div>
        <div className="stat">
          <span className="num">{metrics.pollCount}</span>
          <span className="lbl">poll ticks</span>
        </div>
        <div className="stat">
          <span className="num">{metrics.captureSamples}</span>
          <span className="lbl">capture samples</span>
        </div>
        <div className="stat">
          <span className="num">{recent.length}</span>
          <span className="lbl">buffered</span>
        </div>
      </div>

      <h3 className="sub">Capture latency</h3>
      <div className="stats">
        <div className="stat">
          <span className="num">{metrics.captureMicros} µs</span>
          <span className="lbl">last</span>
        </div>
        <div className="stat">
          <span className="num">{metrics.captureP50Micros} µs</span>
          <span className="lbl">p50</span>
        </div>
        <div className="stat">
          <span className="num">{metrics.captureP95Micros} µs</span>
          <span className="lbl">p95</span>
        </div>
        <div className="stat">
          <span className="num">{metrics.captureMaxMicros} µs</span>
          <span className="lbl">max</span>
        </div>
      </div>
      <Sparkline values={spark} />

      <h3 className="sub">Injected input (live)</h3>
      {recent.length === 0 ? (
        <div className="meta">No injected input yet — arm the engine and use the macros in-game.</div>
      ) : (
        <ul className="logs">
          {recent.map((inj, i) => (
            <li key={`${inj.tsMs}-${i}`}>
              <span className="ts">{time(inj.tsMs)}</span>
              <span className="msg">{inj.label}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

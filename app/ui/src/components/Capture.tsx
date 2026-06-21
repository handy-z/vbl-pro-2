import { useEffect, useState } from "react";
import { pickPixel, sampleCapture, suggestTolerance } from "../api";
import type { CaptureSampleDto, VblSettings } from "../types";

type Pending = { key: string; count: number; mode: "point" | "tol" };

const LABELS: Record<string, string> = {
  GameOnGround: "On ground",
  GameUltimateReady: "Ultimate ready",
};

function swatch(r: number, g: number, b: number) {
  return { background: `rgb(${r}, ${g}, ${b})` };
}

export function Capture({
  config,
  onChange,
  resolution,
}: {
  config: VblSettings;
  onChange: (next: VblSettings) => void;
  resolution: [number, number] | null;
}) {
  const [samples, setSamples] = useState<Record<string, CaptureSampleDto>>({});
  const [pending, setPending] = useState<Pending | null>(null);

  useEffect(() => {
    let active = true;
    const tick = () =>
      sampleCapture()
        .then((list) => {
          if (!active) return;
          const map: Record<string, CaptureSampleDto> = {};
          for (const s of list) map[s.key] = s;
          setSamples(map);
        })
        .catch(() => {});
    tick();
    const id = setInterval(tick, 300);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, []);

  const applyPick = (key: string) => {
    pickPixel()
      .then((p) => {
        if (!p || !resolution) return;
        const [w, h] = resolution;
        const point = { nx: p.x / w, ny: p.y / h };
        onChange({
          ...config,
          capture: config.capture.map((c) =>
            c.key === key ? { ...c, point, target: { r: p.r, g: p.g, b: p.b } } : c,
          ),
        });
      })
      .catch(() => {});
  };

  // Auto-tolerance: sample the OFF-state pixel and compute a tolerance that separates it from
  // the stored target (ON) color.
  const applyAutoTol = (key: string) => {
    const current = config.capture.find((c) => c.key === key);
    if (!current) return;
    pickPixel()
      .then((off) => {
        if (!off) return;
        return suggestTolerance(current.target, { r: off.r, g: off.g, b: off.b }).then((tol) =>
          setTolerance(key, tol.perChannel),
        );
      })
      .catch(() => {});
  };

  const startCountdown = (key: string, mode: "point" | "tol") => {
    if (pending) return;
    let count = 3;
    setPending({ key, count, mode });
    const id = setInterval(() => {
      count -= 1;
      if (count <= 0) {
        clearInterval(id);
        setPending(null);
        if (mode === "point") applyPick(key);
        else applyAutoTol(key);
      } else {
        setPending({ key, count, mode });
      }
    }, 1000);
  };

  const setTolerance = (key: string, perChannel: number) => {
    onChange({
      ...config,
      capture: config.capture.map((c) =>
        c.key === key ? { ...c, tolerance: { ...c.tolerance, perChannel } } : c,
      ),
    });
  };

  const [w, h] = resolution ?? [0, 0];

  return (
    <section className="card">
      <h2>Capture &amp; calibration</h2>
      <div className="meta">
        Resolution: {resolution ? `${w}×${h}` : "—"}. Hover the in-game element and click{" "}
        <b>Calibrate</b> (a 3s countdown samples the pixel under your cursor). To set tolerance
        automatically, hover an <i>off-state</i> pixel and click <b>Auto-tol</b>.
      </div>
      <table className="cap">
        <thead>
          <tr>
            <th>State</th>
            <th>Point</th>
            <th>Target</th>
            <th>Live</th>
            <th>Tol</th>
            <th>Match</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {config.capture.map((c) => {
            const live = samples[c.key];
            const counting = pending?.key === c.key;
            return (
              <tr key={c.key}>
                <td>{LABELS[c.key] ?? c.key}</td>
                <td className="mono">
                  {Math.round((c.point.nx ?? 0) * w)},{Math.round((c.point.ny ?? 0) * h)}
                </td>
                <td>
                  <span className="sw" style={swatch(c.target.r, c.target.g, c.target.b)} />
                </td>
                <td>
                  {live ? <span className="sw" style={swatch(live.r, live.g, live.b)} /> : "—"}
                </td>
                <td>
                  <input
                    type="number"
                    min={0}
                    max={64}
                    value={c.tolerance.perChannel}
                    onChange={(e) => setTolerance(c.key, Number(e.target.value))}
                  />
                </td>
                <td>
                  <span className={`badge ${live?.matched ? "ok" : "no"}`}>
                    {live ? (live.matched ? "yes" : "no") : "—"}
                  </span>
                </td>
                <td>
                  <span className="row-actions">
                    <button
                      className="mini"
                      onClick={() => startCountdown(c.key, "point")}
                      disabled={!!pending}
                    >
                      {counting && pending?.mode === "point" ? `${pending?.count}…` : "Calibrate"}
                    </button>
                    <button
                      className="mini"
                      title="Sample the OFF-state pixel to auto-set tolerance"
                      onClick={() => startCountdown(c.key, "tol")}
                      disabled={!!pending}
                    >
                      {counting && pending?.mode === "tol" ? `${pending?.count}…` : "Auto-tol"}
                    </button>
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </section>
  );
}

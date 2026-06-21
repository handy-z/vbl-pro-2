import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

export function WindowControls() {
  return (
    <div className="winctl">
      <button title="Minimize" onClick={() => void appWindow.minimize()}>
        &#x2013;
      </button>
      <button title="Maximize" onClick={() => void appWindow.toggleMaximize()}>
        &#x25A2;
      </button>
      <button className="close" title="Close" onClick={() => void appWindow.close()}>
        &#x2715;
      </button>
    </div>
  );
}

type ResizeDir =
  | "North"
  | "South"
  | "East"
  | "West"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest";

const EDGES: { cls: string; dir: ResizeDir }[] = [
  { cls: "n", dir: "North" },
  { cls: "s", dir: "South" },
  { cls: "e", dir: "East" },
  { cls: "w", dir: "West" },
  { cls: "ne", dir: "NorthEast" },
  { cls: "nw", dir: "NorthWest" },
  { cls: "se", dir: "SouthEast" },
  { cls: "sw", dir: "SouthWest" },
];

export function ResizeHandles() {
  return (
    <>
      {EDGES.map((e) => (
        <div
          key={e.cls}
          className={`rz rz-${e.cls}`}
          onMouseDown={() => void appWindow.startResizeDragging(e.dir)}
        />
      ))}
    </>
  );
}

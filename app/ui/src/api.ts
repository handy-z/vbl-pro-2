import { commands, events } from "./bindings";
import type {
  CaptureSampleDto,
  InjectionDto,
  LogLine,
  Metrics,
  PixelPick,
  Rgb,
  RuntimeStatus,
  Tolerance,
  VblSettings,
} from "./types";

export function getStatus(): Promise<RuntimeStatus> {
  return commands.getStatus();
}

export function getConfig(): Promise<VblSettings> {
  return commands.getConfig();
}

export function updateConfig(config: VblSettings): Promise<void> {
  return commands.updateConfig(config);
}

export function pickPixel(): Promise<PixelPick | null> {
  return commands.pickPixel();
}

export function suggestTolerance(on: Rgb, off: Rgb): Promise<Tolerance> {
  return commands.suggestTolerance(on, off);
}

export function sampleCapture(): Promise<CaptureSampleDto[]> {
  return commands.sampleCapture();
}

export function recentInjections(): Promise<InjectionDto[]> {
  return commands.recentInjections();
}

export function getMetrics(): Promise<Metrics> {
  return commands.getMetrics();
}

export function listProfiles(): Promise<string[]> {
  return commands.listProfiles();
}

export function activeProfile(): Promise<string> {
  return commands.activeProfile();
}

export function switchProfile(name: string): Promise<VblSettings | null> {
  return commands.switchProfile(name);
}

export function saveProfileAs(name: string): Promise<void> {
  return commands.saveProfileAs(name);
}

export function deleteProfile(name: string): Promise<VblSettings | null> {
  return commands.deleteProfile(name);
}

function unwrap<T>(result: { status: "ok"; data: T } | { status: "error"; error: string }): T {
  if (result.status === "error") throw new Error(result.error);
  return result.data;
}

export async function exportProfile(path: string): Promise<void> {
  unwrap(await commands.exportProfile(path));
}

export async function importProfile(path: string): Promise<VblSettings> {
  return unwrap(await commands.importProfile(path));
}

export function arm(): Promise<void> {
  return commands.arm();
}

export function disarm(): Promise<void> {
  return commands.disarm();
}

export function toggleArmed(): Promise<void> {
  return commands.toggleArmed();
}

export function reloadScript(): Promise<void> {
  return commands.reloadScript();
}

export function onStatus(callback: (status: RuntimeStatus) => void) {
  return events.statusEvent.listen((e) => callback(e.payload));
}

export function onLog(callback: (line: LogLine) => void) {
  return events.logEvent.listen((e) => callback(e.payload));
}

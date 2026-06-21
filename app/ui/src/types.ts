import type {
  CaptureSampleDto_Serialize,
  CaptureState_Serialize,
  LogEntry,
  RuntimeStatus,
  StateKey_Serialize,
  VblSettings_Serialize,
} from "./bindings";

export type {
  Aggregate,
  CrosshairConfig,
  CrosshairOffset,
  InjectionDto,
  MacroKeybinds,
  Metrics,
  NormalizedPoint,
  PixelPick,
  Rgb,
  RuntimeStatus,
  SkillMode,
  Tolerance,
} from "./bindings";

export type VblSettings = VblSettings_Serialize;
export type CaptureState = CaptureState_Serialize;
export type CaptureSampleDto = CaptureSampleDto_Serialize;
export type StateKey = StateKey_Serialize;
export type LogLine = LogEntry;

export const DEFAULT_STATUS: RuntimeStatus = {
  armed: false,
  targetFocused: false,
  captureMatched: false,
  gameOnGround: false,
  gameUltimateReady: false,
  x1Held: false,
  x2Held: false,
  skillEnabled: true,
  resolution: null,
};

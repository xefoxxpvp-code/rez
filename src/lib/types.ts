// Mirrors of the Rust IPC structs (serde camelCase).

export interface DisplayMode {
  width: number;
  height: number;
  refreshHz: number;
}

export interface GameEntry {
  id: string;
  name: string;
  exeName: string;
  exePath: string;
  width: number;
  height: number;
  enabled: boolean;
  keepStretched: boolean;
  /** Crosshair shown over this game; null = none. */
  crosshairId: string | null;
}

export interface Settings {
  autostart: boolean;
  crosshairsSeeded: boolean;
  crosshairUndistort: boolean;
  hideCrosshairWithFaceit: boolean;
}

export interface Lines {
  enabled: boolean;
  length: number;
  verticalLength: number | null;
  thickness: number;
  gap: number;
  opacity: number;
}

export interface Crosshair {
  id: string;
  name: string;
  /** #RRGGBB */
  color: string;
  inner: Lines;
  outer: Lines;
  tStyle: boolean;
  outline: { enabled: boolean; thickness: number; opacity: number };
  dot: { enabled: boolean; size: number; opacity: number };
  circle: { enabled: boolean; radius: number; thickness: number; opacity: number };
  image: string | null;
  imageScale: number;
  imageOpacity: number;
  offsetX: number;
  offsetY: number;
}

export interface Imported {
  crosshair: Crosshair;
  notes: string[];
}

export interface Config {
  version: number;
  games: GameEntry[];
  settings: Settings;
  crosshairs: Crosshair[];
}

export interface DisplayInfo {
  device: string;
  label: string;
  primary: boolean;
  native: DisplayMode;
  /** One entry per resolution, with the refresh rate REZ would use there. */
  resolutions: DisplayMode[];
}

/** Data URLs: Steam logo and hero art when the game is a Steam install, plus the exe icon. */
export interface GameArt {
  logo: string | null;
  hero: string | null;
  icon: string | null;
}

export type StatusEvent =
  | { kind: "idle" }
  | { kind: "active"; gameId: string; width: number; height: number; refreshHz: number }
  | { kind: "modeUnavailable"; gameId: string; width: number; height: number }
  | { kind: "error"; message: string };

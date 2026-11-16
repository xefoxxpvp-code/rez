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
}

export interface Settings {
  autostart: boolean;
}

export interface Config {
  version: number;
  games: GameEntry[];
  settings: Settings;
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

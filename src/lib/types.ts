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

export interface Resolution {
  width: number;
  height: number;
}

export interface DisplayInfo {
  device: string;
  label: string;
  primary: boolean;
  native: DisplayMode;
  resolutions: Resolution[];
}

export type StatusEvent =
  | { kind: "idle" }
  | { kind: "active"; gameId: string; width: number; height: number }
  | { kind: "modeUnavailable"; gameId: string; width: number; height: number }
  | { kind: "error"; message: string };

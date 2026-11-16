import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Config, DisplayInfo, GameArt, GameEntry, StatusEvent } from "./types";

export const getConfig = () => invoke<Config>("get_config");
export const listDisplays = () => invoke<DisplayInfo[]>("list_displays");
export const getGameArt = (id: string) => invoke<GameArt>("get_game_art", { id });
export const addGame = (exePath: string) => invoke<GameEntry>("add_game", { exePath });
export const updateGame = (game: GameEntry) => invoke<void>("update_game", { game });
export const removeGame = (id: string) => invoke<void>("remove_game", { id });
export const restoreNative = () => invoke<void>("restore_native");
export const getStatus = () => invoke<StatusEvent>("get_status");
export const setAutostart = (enabled: boolean) => invoke<void>("set_autostart", { enabled });

export const onStatus = (cb: (status: StatusEvent) => void): Promise<UnlistenFn> =>
  listen<StatusEvent>("rez://status", (e) => cb(e.payload));

/** Opens the file picker for a game executable. Returns null when cancelled. */
export async function pickExe(title: string): Promise<string | null> {
  const picked = await open({ title, multiple: false, directory: false, filters: [{ name: "Jeu", extensions: ["exe"] }] });
  return typeof picked === "string" ? picked : null;
}

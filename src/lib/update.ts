import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useState } from "react";

export function updateLabel(version: string): string {
  return `Mise à jour ${version} disponible`;
}

export function progressLabel(done: number, total: number | undefined): string {
  if (!total) return "Téléchargement…";
  return `Téléchargement… ${Math.round((done / total) * 100)} %`;
}

/** The two steps of an update, as provided by the updater plugin's `Update`. */
export interface UpdateSteps {
  download: (onEvent: (event: DownloadEvent) => void) => Promise<void>;
  install: () => Promise<void>;
}

/**
 * Download first, then put the screen back to native (and restore the desktop layout), then
 * install: on Windows the installer closes REZ immediately, so nothing may run after `install`.
 */
export async function runUpdate(update: UpdateSteps, prepare: () => Promise<unknown>, onEvent: (event: DownloadEvent) => void) {
  await update.download(onEvent);
  await prepare();
  await update.install();
}

export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "upToDate" }
  | { kind: "available"; update: Update }
  | { kind: "installing"; label: string }
  | { kind: "error"; message: string };

/** Checks GitHub Releases for a newer signed REZ and installs it in place (no reinstall). */
export function useUpdater(checkOnMount: boolean) {
  const [state, setState] = useState<UpdateState>({ kind: "idle" });

  const checkNow = useCallback(async (silent = false) => {
    if (!silent) setState({ kind: "checking" });
    try {
      const update = await check();
      setState(update ? { kind: "available", update } : silent ? { kind: "idle" } : { kind: "upToDate" });
    } catch (e) {
      setState(silent ? { kind: "idle" } : { kind: "error", message: String(e) });
    }
  }, []);

  useEffect(() => {
    if (checkOnMount) void checkNow(true);
  }, [checkOnMount, checkNow]);

  const install = useCallback(async () => {
    if (state.kind !== "available") return;
    const { update } = state;
    try {
      let done = 0;
      let total: number | undefined;
      setState({ kind: "installing", label: progressLabel(0, undefined) });
      await runUpdate(update, () => invoke("prepare_update"), (event) => {
        if (event.event === "Started") total = event.data.contentLength;
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          setState({ kind: "installing", label: progressLabel(done, total) });
        }
      });
      // On Windows the installer restarts REZ itself; elsewhere, relaunch.
      await relaunch();
    } catch (e) {
      setState({ kind: "error", message: String(e) });
    }
  }, [state]);

  return { state, checkNow, install };
}

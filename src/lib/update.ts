import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useState } from "react";

export function updateLabel(version: string): string {
  return `Mise à jour ${version} disponible`;
}

export function progressLabel(done: number, total: number | undefined): string {
  if (!total) return "Téléchargement…";
  return `Téléchargement… ${Math.round((done / total) * 100)} %`;
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
      // Native resolution back and desktop layout restored before the installer closes REZ.
      await invoke("prepare_update");
      let done = 0;
      let total: number | undefined;
      setState({ kind: "installing", label: progressLabel(0, undefined) });
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength;
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          setState({ kind: "installing", label: progressLabel(done, total) });
        }
      });
      await relaunch();
    } catch (e) {
      setState({ kind: "error", message: String(e) });
    }
  }, [state]);

  return { state, checkNow, install };
}

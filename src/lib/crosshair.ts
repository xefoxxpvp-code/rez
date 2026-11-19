import { useEffect, useState } from "react";
import { previewCrosshair } from "./api";
import type { Crosshair } from "./types";

export type CodeKind = "rez" | "cs2" | "valorant";

/** Which game a pasted crosshair code comes from (label only; Rust does the real parsing). */
export function detectCodeKind(raw: string): CodeKind | null {
  const code = raw.trim();
  if (code.startsWith("REZ1-")) return "rez";
  if (code.startsWith("CSGO-") || (code.startsWith("CS") && code.length === 46)) return "cs2";
  if (code.startsWith("0;")) return "valorant";
  return null;
}

// Previews are deterministic: cache them by crosshair content.
const previews = new Map<string, Promise<string>>();

/** PNG data URL of a crosshair, drawn by the same renderer as the in-game overlay. */
export function useCrosshairPreview(crosshair: Crosshair | null, debounceMs = 0): string | null {
  const [url, setUrl] = useState<string | null>(null);
  const key = crosshair ? JSON.stringify({ ...crosshair, id: "", name: "" }) : null;
  useEffect(() => {
    if (!crosshair || !key) {
      setUrl(null);
      return;
    }
    let alive = true;
    const timer = window.setTimeout(() => {
      let pending = previews.get(key);
      if (!pending) {
        pending = previewCrosshair(crosshair).catch(() => "");
        previews.set(key, pending);
      }
      void pending.then((u) => {
        if (alive) setUrl(u || null);
      });
    }, debounceMs);
    return () => {
      alive = false;
      window.clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, debounceMs]);
  return url;
}

/** Colours offered as one-click swatches (the common pro picks first). */
export const SWATCHES = ["#00FFFF", "#32FA32", "#FFFFFF", "#FAFA32", "#FF00FF", "#FF0000", "#7FFF00", "#FF8A00"];

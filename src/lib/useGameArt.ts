import { useEffect, useState } from "react";
import { getGameArt } from "./api";
import type { GameArt } from "./types";

// Artwork never changes while the app runs: fetch each game's art once.
const cache = new Map<string, Promise<GameArt>>();

export function useGameArt(id: string | null): GameArt | null {
  const [art, setArt] = useState<GameArt | null>(null);
  useEffect(() => {
    if (!id) {
      setArt(null);
      return;
    }
    let alive = true;
    let pending = cache.get(id);
    if (!pending) {
      pending = getGameArt(id).catch(() => ({ logo: null, hero: null, icon: null }));
      cache.set(id, pending);
    }
    void pending.then((a) => {
      if (alive) setArt(a);
    });
    return () => {
      alive = false;
    };
  }, [id]);
  return art;
}

export function forgetGameArt(id: string) {
  cache.delete(id);
}

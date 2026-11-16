import { useTranslation } from "react-i18next";
import { restoreNative } from "../lib/api";
import { formatResolution } from "../lib/format";
import { useGameArt } from "../lib/useGameArt";
import type { GameEntry, StatusEvent } from "../lib/types";
import GameArtTile from "./GameArtTile";

interface Props {
  status: StatusEvent;
  games: GameEntry[];
  screen: { width: number; height: number };
}

/** Live state panel: what REZ is doing to the screen right now. */
export default function StatusBar({ status, games, screen }: Props) {
  const { t } = useTranslation();
  const activeId = status.kind === "active" ? status.gameId : null;
  const art = useGameArt(activeId);
  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;

  if (status.kind === "active") {
    return (
      <div
        role="status"
        aria-live="polite"
        className="relative flex items-center gap-5 overflow-hidden rounded-[24px] bg-white/[0.03] p-3 pr-4 shadow-[0_30px_60px_-15px_rgba(0,0,0,0.55)]"
      >
        {art?.hero ? (
          <>
            <img src={art.hero} alt="" className="absolute inset-0 h-full w-full object-cover object-right opacity-60" />
            <div
              className="absolute inset-0"
              style={{
                background:
                  "linear-gradient(90deg, rgba(0,0,0,.95) 0%, rgba(0,0,0,.85) 30%, rgba(0,0,0,.2) 70%, rgba(0,0,0,.1) 100%)",
              }}
            />
          </>
        ) : null}
        <GameArtTile
          art={art}
          name={nameOf(status.gameId)}
          active
          fallback={{ width: status.width, height: status.height, screen }}
          className="relative h-[90px] w-40"
        />
        <div className="relative min-w-0 flex-1">
          <p className="m-0 text-[11px] font-bold uppercase leading-[16.5px] tracking-[0.2em] text-blue-300/80">
            {t("status.eyebrow")}
          </p>
          <p className="m-0 truncate text-[18px] font-bold leading-7">{nameOf(status.gameId)}</p>
          <p className="m-0 flex items-baseline gap-3">
            <span className="text-[30px] font-black italic leading-none tracking-tight tabular-nums">
              {formatResolution(status.width, status.height)}
            </span>
            <span className="rounded-full bg-blue-600/20 px-2.5 py-0.5 text-xs font-bold tabular-nums text-blue-300">
              {status.refreshHz} Hz
            </span>
          </p>
        </div>
        <button
          type="button"
          onClick={() => void restoreNative()}
          className="relative h-12 rounded-lg border-2 border-primary/50 bg-primary px-6 text-sm font-medium text-white shadow-lg transition-colors hover:border-primary hover:bg-primary-hover"
        >
          {t("status.restore")}
        </button>
      </div>
    );
  }

  let text = t("status.idle");
  let tone = "text-zinc-400";
  if (status.kind === "modeUnavailable") {
    text = t("status.modeUnavailable", { name: nameOf(status.gameId), res: formatResolution(status.width, status.height) });
    tone = "bg-red-500/10 text-red-500";
  } else if (status.kind === "error") {
    text = status.message;
    tone = "bg-red-500/10 text-red-500";
  }

  return (
    <div role="status" aria-live="polite" className={`flex items-center gap-3 rounded-[24px] bg-white/[0.03] px-5 py-4 ${tone}`}>
      {status.kind === "idle" ? <span aria-hidden="true" className="h-2 w-2 rounded-full bg-zinc-600" /> : null}
      <p className="m-0">{text}</p>
    </div>
  );
}

import { useTranslation } from "react-i18next";
import { restoreNative } from "../lib/api";
import { formatResolution } from "../lib/format";
import type { GameEntry, StatusEvent } from "../lib/types";
import AspectFrame from "./AspectFrame";

interface Props {
  status: StatusEvent;
  games: GameEntry[];
  screen: { width: number; height: number };
}

/** Live state panel: what REZ is doing to the screen right now. */
export default function StatusBar({ status, games, screen }: Props) {
  const { t } = useTranslation();
  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;

  if (status.kind === "active") {
    return (
      <div role="status" aria-live="polite" className="flex items-center gap-4 rounded-[24px] bg-white/[0.03] p-2.5 pr-4">
        <div className="flex h-[72px] w-32 items-center justify-center rounded-[14px] bg-primary/30">
          <AspectFrame width={status.width} height={status.height} screen={screen} active size={40} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="m-0 text-[11px] font-bold uppercase leading-[16.5px] tracking-[0.2em] text-blue-300/80">
            {t("status.eyebrow")}
          </p>
          <p className="m-0 truncate text-[18px] font-bold leading-7">{nameOf(status.gameId)}</p>
          <p className="m-0 text-[26px] font-black italic leading-none tracking-tight tabular-nums">
            {formatResolution(status.width, status.height)}
          </p>
        </div>
        <button
          type="button"
          onClick={() => void restoreNative()}
          className="h-12 rounded-lg border-2 border-primary/50 bg-primary px-6 text-sm font-medium text-white shadow-lg transition-colors hover:border-primary hover:bg-primary-hover"
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

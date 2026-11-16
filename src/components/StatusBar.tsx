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

export default function StatusBar({ status, games, screen }: Props) {
  const { t } = useTranslation();
  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;

  let text: string;
  let tone = "border-line text-muted";
  if (status.kind === "active") {
    text = t("status.active", { name: nameOf(status.gameId), res: formatResolution(status.width, status.height) });
    tone = "border-signal text-ink";
  } else if (status.kind === "modeUnavailable") {
    text = t("status.modeUnavailable", { name: nameOf(status.gameId), res: formatResolution(status.width, status.height) });
    tone = "border-danger text-ink";
  } else if (status.kind === "error") {
    text = status.message;
    tone = "border-danger text-ink";
  } else {
    text = t("status.idle");
  }

  return (
    <div role="status" aria-live="polite" className={`flex items-center gap-3 border-l-[3px] bg-panel px-5 py-3 ${tone}`}>
      {status.kind === "active" ? <AspectFrame width={status.width} height={status.height} screen={screen} active /> : null}
      <p className="m-0 flex-1">{text}</p>
      {status.kind === "active" ? (
        <button
          type="button"
          onClick={() => void restoreNative()}
          className="rounded-md bg-signal px-3 py-1.5 font-semibold text-base hover:brightness-110"
        >
          {t("status.restore")}
        </button>
      ) : null}
    </div>
  );
}

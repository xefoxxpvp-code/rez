import { useState } from "react";
import { useTranslation } from "react-i18next";
import GameCard from "../components/GameCard";
import { addGame, pickExe, removeGame, updateGame } from "../lib/api";
import { formatResolution } from "../lib/format";
import type { Config, DisplayInfo, DisplayMode, StatusEvent } from "../lib/types";

interface Props {
  config: Config;
  resolutions: DisplayMode[];
  display: DisplayInfo | null;
  screen: { width: number; height: number };
  status: StatusEvent;
  onChanged: () => void;
}

export default function Games({ config, resolutions, display, screen, status, onChanged }: Props) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);

  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(String(e));
    }
    onChanged();
  };

  const add = () =>
    run(async () => {
      const path = await pickExe(t("games.pickTitle"));
      if (path) await addGame(path);
    });

  const addButton = (
    <button
      type="button"
      onClick={() => void add()}
      className="h-11 shrink-0 rounded-full bg-white px-5 text-xs font-extrabold uppercase tracking-wide text-black transition-colors hover:bg-blue-500 hover:text-white"
    >
      {t("games.add")}
    </button>
  );

  const screenChip = display ? (
    <p className="m-0 flex items-center gap-2 text-xs font-semibold text-zinc-400">
      <span aria-hidden="true" className="h-2 w-2 rounded-full bg-emerald-400" />
      {t("games.screen")}
      <span className="font-black italic tabular-nums text-white">{formatResolution(display.native.width, display.native.height)}</span>
      <span className="rounded-full bg-blue-600/20 px-2 py-0.5 font-bold tabular-nums text-blue-300">{display.native.refreshHz} Hz</span>
    </p>
  ) : null;

  return (
    <section className="flex flex-col gap-5">
      {config.games.length === 0 ? (
        <div className="max-w-[60ch] pt-4">
          <h2 className="display m-0 text-[36px] leading-[40px]">
            {t("games.emptyTitleStart")} <span className="display-accent">{t("games.emptyTitleEnd")}</span>
          </h2>
          <p className="mb-4 mt-3 text-base leading-relaxed text-zinc-400">{t("games.emptyBody")}</p>
          <div className="mb-6">{screenChip}</div>
          {addButton}
        </div>
      ) : (
        <>
          <div className="flex items-end justify-between gap-4">
            <div className="flex flex-col gap-2">
              <h2 className="display m-0 text-[36px] leading-[40px]">
                {t("games.titleStart")} <span className="display-accent">{t("games.titleEnd")}</span>
              </h2>
              {screenChip}
            </div>
            {addButton}
          </div>
          <ul className="m-0 flex list-none flex-col gap-3 p-0">
            {config.games.map((g) => (
              <GameCard
                key={g.id}
                game={g}
                resolutions={resolutions}
                crosshairs={config.crosshairs}
                screen={screen}
                active={status.kind === "active" && status.gameId === g.id}
                onChange={(game) => void run(() => updateGame(game))}
                onRemove={(game) => void run(() => removeGame(game.id))}
              />
            ))}
          </ul>
        </>
      )}
      {error ? (
        <p role="alert" className="m-0 rounded-xl bg-red-500/10 px-4 py-3 text-red-500">
          {error}
        </p>
      ) : null}
    </section>
  );
}

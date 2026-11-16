import { useState } from "react";
import { useTranslation } from "react-i18next";
import GameCard from "../components/GameCard";
import { addGame, pickExe, removeGame, updateGame } from "../lib/api";
import type { Config, Resolution, StatusEvent } from "../lib/types";

interface Props {
  config: Config;
  resolutions: Resolution[];
  screen: { width: number; height: number };
  status: StatusEvent;
  onChanged: () => void;
}

export default function Games({ config, resolutions, screen, status, onChanged }: Props) {
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
      className="rounded-md border border-line px-3 py-1.5 font-semibold hover:border-ink"
    >
      {t("games.add")}
    </button>
  );

  return (
    <section className="flex flex-col gap-4">
      {config.games.length === 0 ? (
        <div className="max-w-[60ch] py-6">
          <h2 className="m-0 font-display text-2xl font-semibold">{t("games.emptyTitle")}</h2>
          <p className="mb-4 mt-2 text-muted">{t("games.emptyBody")}</p>
          {addButton}
        </div>
      ) : (
        <>
          <ul className="m-0 flex list-none flex-col gap-2 p-0">
            {config.games.map((g) => (
              <GameCard
                key={g.id}
                game={g}
                resolutions={resolutions}
                screen={screen}
                active={status.kind === "active" && status.gameId === g.id}
                onChange={(game) => void run(() => updateGame(game))}
                onRemove={(game) => void run(() => removeGame(game.id))}
              />
            ))}
          </ul>
          <div>{addButton}</div>
        </>
      )}
      {error ? (
        <p role="alert" className="m-0 text-danger">
          {error}
        </p>
      ) : null}
    </section>
  );
}

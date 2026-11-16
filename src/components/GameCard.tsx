import { useTranslation } from "react-i18next";
import { aspectLabel, formatResolution } from "../lib/format";
import type { GameEntry, Resolution } from "../lib/types";
import AspectFrame from "./AspectFrame";

interface Props {
  game: GameEntry;
  resolutions: Resolution[];
  screen: { width: number; height: number };
  active: boolean;
  onChange: (game: GameEntry) => void;
  onRemove: (game: GameEntry) => void;
}

export default function GameCard({ game, resolutions, screen, active, onChange, onRemove }: Props) {
  const { t } = useTranslation();
  const value = `${game.width}x${game.height}`;
  const options = resolutions.some((r) => r.width === game.width && r.height === game.height)
    ? resolutions
    : [{ width: game.width, height: game.height }, ...resolutions];

  return (
    <li
      className={`grid grid-cols-[auto_1fr_auto] items-center gap-x-4 gap-y-2 rounded-lg bg-panel px-4 py-3 ${
        game.enabled ? "" : "opacity-60"
      }`}
    >
      <AspectFrame width={game.width} height={game.height} screen={screen} active={active} />
      <div className="min-w-0">
        <p className="m-0 truncate font-semibold">{game.name}</p>
        <p className="m-0 truncate text-xs text-muted" title={game.exePath}>
          {game.exeName}
        </p>
      </div>
      <select
        aria-label={t("games.resolution", { name: game.name })}
        value={value}
        onChange={(e) => {
          const [w, h] = e.target.value.split("x").map(Number);
          onChange({ ...game, width: w, height: h });
        }}
        className="rounded-md border border-line bg-base px-2 py-1.5 font-display text-[15px] font-semibold tabular-nums"
      >
        {options.map((r) => (
          <option key={`${r.width}x${r.height}`} value={`${r.width}x${r.height}`}>
            {formatResolution(r.width, r.height)} ({aspectLabel(r.width, r.height)})
          </option>
        ))}
      </select>
      <div className="col-start-2 col-end-4 flex flex-wrap items-center gap-x-5 gap-y-1 text-[13px]">
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            className="accent-signal"
            checked={game.enabled}
            onChange={(e) => onChange({ ...game, enabled: e.target.checked })}
          />
          {t("games.enabled")}
        </label>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            className="accent-signal"
            checked={game.keepStretched}
            onChange={(e) => onChange({ ...game, keepStretched: e.target.checked })}
          />
          {t("games.keepStretched")}
        </label>
        <button
          type="button"
          onClick={() => {
            if (window.confirm(t("games.confirmRemove", { name: game.name }))) onRemove(game);
          }}
          className="ml-auto text-muted underline-offset-2 hover:text-danger hover:underline"
        >
          {t("games.remove")}
        </button>
      </div>
    </li>
  );
}

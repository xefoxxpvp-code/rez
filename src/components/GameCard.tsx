import { useTranslation } from "react-i18next";
import { useGameArt } from "../lib/useGameArt";
import type { DisplayMode, GameEntry } from "../lib/types";
import GameArtTile from "./GameArtTile";
import ResolutionPicker from "./ResolutionPicker";

interface Props {
  game: GameEntry;
  resolutions: DisplayMode[];
  screen: { width: number; height: number };
  active: boolean;
  onChange: (game: GameEntry) => void;
  onRemove: (game: GameEntry) => void;
}

function TogglePill({ on, label, onToggle }: { on: boolean; label: string; onToggle: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onToggle}
      className={`inline-flex h-8 items-center gap-2 rounded-full px-3.5 text-xs font-bold transition-colors ${
        on ? "bg-blue-600/20 text-blue-300 hover:bg-blue-600/30" : "bg-white/5 text-zinc-400 hover:bg-white/[0.08] hover:text-white"
      }`}
    >
      <span aria-hidden="true" className={`h-1.5 w-1.5 rounded-full ${on ? "bg-blue-400" : "bg-zinc-600"}`} />
      {label}
    </button>
  );
}

/** One game: its own artwork, its switches and its resolution, laid out like the reference's chapter row. */
export default function GameCard({ game, resolutions, screen, active, onChange, onRemove }: Props) {
  const { t } = useTranslation();
  const art = useGameArt(game.id);

  return (
    <li
      className={`flex items-center gap-4 rounded-[22px] bg-white/[0.03] p-2 pr-4 transition-colors duration-300 hover:bg-white/[0.06] ${
        game.enabled ? "" : "opacity-60"
      }`}
    >
      <GameArtTile
        art={art}
        name={game.name}
        active={active}
        fallback={{ width: game.width, height: game.height, screen }}
        className="h-[99px] w-44"
      >
        {active ? (
          <span className="absolute left-1.5 top-1.5 rounded-md bg-blue-600 px-1.5 py-0.5 text-[10px] font-black uppercase leading-[15px] tracking-[0.18em] text-white shadow-lg shadow-blue-600/30">
            {t("games.activeBadge")}
          </span>
        ) : null}
      </GameArtTile>

      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="min-w-0">
          <p className="m-0 truncate text-[15px] font-bold leading-[1.5]">{game.name}</p>
          <p className="m-0 truncate text-xs font-semibold leading-[18px] text-zinc-500" title={game.exePath}>
            {game.exeName}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <TogglePill on={game.enabled} label={t("games.enabled")} onToggle={() => onChange({ ...game, enabled: !game.enabled })} />
          <TogglePill
            on={game.keepStretched}
            label={t("games.keepStretched")}
            onToggle={() => onChange({ ...game, keepStretched: !game.keepStretched })}
          />
        </div>
      </div>

      <div className="flex shrink-0 flex-col items-end gap-2">
        <ResolutionPicker
          label={t("games.resolution", { name: game.name })}
          width={game.width}
          height={game.height}
          options={resolutions}
          screen={screen}
          onChange={(width, height) => onChange({ ...game, width, height })}
        />
        <button
          type="button"
          onClick={() => {
            if (window.confirm(t("games.confirmRemove", { name: game.name }))) onRemove(game);
          }}
          className="px-1 text-xs font-bold uppercase tracking-[0.1em] text-zinc-500 transition-colors hover:text-red-500"
        >
          {t("games.remove")}
        </button>
      </div>
    </li>
  );
}

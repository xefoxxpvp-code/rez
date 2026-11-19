import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import CrosshairEditor from "../components/CrosshairEditor";
import CrosshairPreview from "../components/CrosshairPreview";
import PasteCodeDialog from "../components/PasteCodeDialog";
import { crosshairShareCode, deleteCrosshair, importCrosshairImage, pickPng, saveCrosshair } from "../lib/api";
import type { Config, Crosshair as CrosshairModel } from "../lib/types";

interface Props {
  config: Config;
  onChanged: () => void;
}

const BLANK: CrosshairModel = {
  id: "",
  name: "",
  color: "#00FFFF",
  inner: { enabled: true, length: 4, verticalLength: null, thickness: 2, gap: 2, opacity: 1 },
  outer: { enabled: false, length: 2, verticalLength: null, thickness: 2, gap: 10, opacity: 0.35 },
  tStyle: false,
  outline: { enabled: false, thickness: 1, opacity: 1 },
  dot: { enabled: false, size: 2, opacity: 1 },
  circle: { enabled: false, radius: 12, thickness: 2, opacity: 1 },
  image: null,
  imageScale: 1,
  imageOpacity: 1,
  offsetX: 0,
  offsetY: 0,
};

/** Crosshair library: pick one on the left, edit it on the right. Edits save themselves. */
export default function Crosshair({ config, onChanged }: Props) {
  const { t } = useTranslation();
  const [selectedId, setSelectedId] = useState<string | null>(config.crosshairs[0]?.id ?? null);
  const [draft, setDraft] = useState<CrosshairModel | null>(null);
  const [pasting, setPasting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const saveTimer = useRef<number | undefined>(undefined);

  const selected = config.crosshairs.find((c) => c.id === selectedId) ?? null;
  useEffect(() => {
    setDraft(selected);
    // Only reload the draft when the user picks another crosshair.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedId, config.crosshairs.length]);

  const report = (e: unknown) => setMessage(String(e));

  const create = async (c: CrosshairModel) => {
    try {
      const saved = await saveCrosshair({ ...c, id: "" });
      setSelectedId(saved.id);
      onChanged();
    } catch (e) {
      report(e);
    }
  };

  const edit = (c: CrosshairModel) => {
    setDraft(c);
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveCrosshair(c).then(onChanged).catch(report);
    }, 400);
  };

  const usedBy = (id: string) => config.games.filter((g) => g.crosshairId === id).map((g) => g.name);

  const ghost = "h-11 rounded-full bg-white/5 px-5 text-xs font-extrabold uppercase tracking-wide transition-colors hover:bg-white/[0.08]";

  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h2 className="display m-0 text-[36px] leading-[40px]">
          {t("xh.titleStart")} <span className="display-accent">{t("xh.titleEnd")}</span>
        </h2>
        <div className="flex flex-wrap gap-2">
          <button type="button" className={ghost} onClick={() => setPasting(true)}>
            {t("xh.paste")}
          </button>
          <button
            type="button"
            className={ghost}
            onClick={async () => {
              const path = await pickPng(t("xh.pickImage"));
              if (!path) return;
              try {
                const saved = await importCrosshairImage(path);
                setSelectedId(saved.id);
                onChanged();
              } catch (e) {
                report(e);
              }
            }}
          >
            {t("xh.importImage")}
          </button>
          <button
            type="button"
            onClick={() => void create({ ...BLANK, name: t("xh.newName") })}
            className="h-11 rounded-full bg-white px-5 text-xs font-extrabold uppercase tracking-wide text-black transition-colors hover:bg-blue-500 hover:text-white"
          >
            {t("xh.new")}
          </button>
        </div>
      </div>

      {message ? (
        <p role="status" className="m-0 rounded-xl bg-white/[0.04] px-4 py-3 text-zinc-300">
          {message}
        </p>
      ) : null}

      <div className="grid grid-cols-[220px_1fr] gap-5">
        <ul className="m-0 flex list-none flex-col gap-2 p-0">
          {config.crosshairs.map((c) => (
            <li key={c.id}>
              <button
                type="button"
                aria-pressed={c.id === selectedId}
                onClick={() => setSelectedId(c.id)}
                className={`flex w-full items-center gap-3 rounded-[22px] p-2 pr-3 text-left transition-colors ${
                  c.id === selectedId ? "bg-white/10" : "bg-white/[0.03] hover:bg-white/[0.06]"
                }`}
              >
                <CrosshairPreview crosshair={c} className="h-14 w-14" />
                <span className="min-w-0">
                  <span className="block truncate text-[15px] font-bold">{c.name}</span>
                  <span className="block truncate text-xs font-semibold text-zinc-500">
                    {usedBy(c.id).join(", ") || t("xh.unused")}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>

        {draft ? (
          <div className="flex flex-col gap-4">
            <CrosshairEditor crosshair={draft} onChange={edit} />
            <div className="flex flex-wrap gap-2">
              <button type="button" className={ghost} onClick={() => void create({ ...draft, name: `${draft.name} (copie)` })}>
                {t("xh.duplicate")}
              </button>
              <button
                type="button"
                className={ghost}
                onClick={async () => {
                  const code = await crosshairShareCode(draft);
                  await navigator.clipboard.writeText(code);
                  setMessage(t("xh.copied"));
                }}
              >
                {t("xh.copyCode")}
              </button>
              <button
                type="button"
                className="ml-auto h-10 rounded-xl bg-red-500/10 px-6 text-xs font-bold uppercase tracking-[0.1em] text-red-500 transition-colors hover:bg-red-500/20"
                onClick={async () => {
                  if (!window.confirm(t("xh.confirmDelete", { name: draft.name }))) return;
                  try {
                    await deleteCrosshair(draft.id);
                    setSelectedId(config.crosshairs.find((c) => c.id !== draft.id)?.id ?? null);
                    onChanged();
                  } catch (e) {
                    report(e);
                  }
                }}
              >
                {t("xh.delete")}
              </button>
            </div>
          </div>
        ) : (
          <p className="m-0 text-zinc-400">{t("xh.empty")}</p>
        )}
      </div>

      {pasting ? (
        <PasteCodeDialog
          onClose={() => setPasting(false)}
          onImport={(c) => {
            setPasting(false);
            void create(c);
          }}
        />
      ) : null}
    </section>
  );
}

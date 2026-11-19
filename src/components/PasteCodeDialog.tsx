import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { importCrosshairCode } from "../lib/api";
import { detectCodeKind } from "../lib/crosshair";
import type { Crosshair, Imported } from "../lib/types";
import CrosshairPreview from "./CrosshairPreview";

interface Props {
  onClose: () => void;
  onImport: (c: Crosshair) => void;
}

/** Paste a Valorant, CS2 or REZ code, see the result and what could not be reproduced, then add it. */
export default function PasteCodeDialog({ onClose, onImport }: Props) {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
  const [result, setResult] = useState<Imported | null>(null);
  const [error, setError] = useState<string | null>(null);
  const field = useRef<HTMLTextAreaElement>(null);
  const kind = detectCodeKind(code);

  useEffect(() => {
    field.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    setResult(null);
    setError(null);
    if (!kind) return;
    let alive = true;
    importCrosshairCode(code)
      .then((r) => alive && setResult(r))
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, [code, kind]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("xh.pasteTitle")}
      className="fixed inset-0 z-[60] flex items-start justify-center bg-black/70 px-4 pt-[10vh] backdrop-blur-md"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="relative w-full max-w-xl overflow-hidden rounded-3xl bg-[#121215]/95 p-5 backdrop-blur-2xl">
        <span
          aria-hidden="true"
          className="pointer-events-none absolute -left-24 -top-28 h-72 w-72"
          style={{ background: "radial-gradient(closest-side, rgba(59,130,246,.18), transparent)" }}
        />
        <h2 className="display relative m-0 text-[28px] leading-none">
          {t("xh.pasteTitleStart")} <span className="display-accent">{t("xh.pasteTitleEnd")}</span>
        </h2>
        <p className="relative mb-3 mt-2 text-zinc-400">{t("xh.pasteHint")}</p>
        <textarea
          ref={field}
          value={code}
          onChange={(e) => setCode(e.target.value)}
          rows={3}
          spellCheck={false}
          className="relative w-full resize-none rounded-2xl bg-white/[0.05] px-4 py-3 font-mono text-[13px] outline-none focus:bg-white/[0.08] focus:ring-2 focus:ring-blue-400/50"
        />
        <div className="relative mt-3 flex min-h-[96px] items-center gap-4">
          {result ? (
            <>
              <CrosshairPreview crosshair={result.crosshair} className="h-24 w-24" />
              <div className="min-w-0 flex-1">
                <p className="m-0 text-[11px] font-bold uppercase tracking-[0.2em] text-blue-300/80">
                  {kind ? t(`xh.kind.${kind}`) : ""}
                </p>
                {result.notes.map((n) => (
                  <p key={n} className="m-0 mt-1 text-[13px] text-zinc-400">
                    {n}
                  </p>
                ))}
              </div>
            </>
          ) : (
            <p className={`m-0 text-[13px] ${error ? "text-red-500" : "text-zinc-500"}`}>
              {error ?? (code.trim() && !kind ? t("xh.unknownCode") : t("xh.waitingCode"))}
            </p>
          )}
        </div>
        <div className="relative mt-4 flex justify-end gap-2">
          <button type="button" onClick={onClose} className="h-11 rounded-full bg-white/5 px-5 text-xs font-extrabold uppercase tracking-wide hover:bg-white/[0.08]">
            {t("xh.cancel")}
          </button>
          <button
            type="button"
            disabled={!result}
            onClick={() => result && onImport(result.crosshair)}
            className="h-11 rounded-full bg-white px-5 text-xs font-extrabold uppercase tracking-wide text-black transition-colors enabled:hover:bg-blue-500 enabled:hover:text-white disabled:opacity-40"
          >
            {t("xh.add")}
          </button>
        </div>
      </div>
    </div>
  );
}

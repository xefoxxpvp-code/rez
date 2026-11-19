import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Crosshair } from "../lib/types";
import CrosshairPreview from "./CrosshairPreview";

interface Props {
  label: string;
  crosshairs: Crosshair[];
  value: string | null;
  onChange: (id: string | null) => void;
}

/** Per-game crosshair choice with live thumbnails, including "none". */
export default function CrosshairPicker({ label, crosshairs, value, onChange }: Props) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const current = crosshairs.find((c) => c.id === value) ?? null;

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => root.current && !root.current.contains(e.target as Node) && setOpen(false);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const choose = (id: string | null) => {
    onChange(id);
    setOpen(false);
  };

  const option = (id: string | null, name: string, c: Crosshair | null) => (
    <li key={id ?? "none"} role="option" aria-selected={value === id}>
      <button
        type="button"
        onClick={() => choose(id)}
        className={`flex w-full items-center gap-3 rounded-[14px] px-2 py-1.5 text-left transition-colors hover:bg-white/[0.08] ${
          value === id ? "bg-white/[0.06]" : ""
        }`}
      >
        {c ? <CrosshairPreview crosshair={c} className="h-9 w-9" /> : <span className="flex h-9 w-9 items-center justify-center rounded-[14px] bg-white/5 text-zinc-500">∅</span>}
        <span className="truncate text-[13px] font-bold">{name}</span>
      </button>
    </li>
  );

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="flex h-10 items-center gap-2 rounded-full bg-white/5 py-1 pl-1 pr-3 text-xs font-bold transition-colors hover:bg-white/[0.08]"
      >
        {current ? (
          <CrosshairPreview crosshair={current} className="h-8 w-8 rounded-full" />
        ) : (
          <span className="flex h-8 w-8 items-center justify-center rounded-full bg-white/5 text-zinc-500">∅</span>
        )}
        <span className={current ? "text-white" : "text-zinc-400"}>{current?.name ?? t("xh.none")}</span>
      </button>
      {open ? (
        <ul
          role="listbox"
          aria-label={label}
          className="custom-scrollbar absolute right-0 top-full z-50 m-0 mt-2 max-h-72 w-60 list-none overflow-y-auto rounded-[24px] bg-[rgba(18,18,21,0.97)] p-2 shadow-[0_30px_60px_-15px_rgba(0,0,0,0.8)] backdrop-blur-2xl"
        >
          {option(null, t("xh.none"), null)}
          {crosshairs.map((c) => option(c.id, c.name, c))}
        </ul>
      ) : null}
    </div>
  );
}

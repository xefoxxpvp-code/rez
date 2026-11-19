import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { SWATCHES } from "../lib/crosshair";
import type { Crosshair, Lines } from "../lib/types";
import CrosshairPreview from "./CrosshairPreview";

interface Props {
  crosshair: Crosshair;
  onChange: (c: Crosshair) => void;
}

function Slider(props: { label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void }) {
  const { label, value, min, max, step, onChange } = props;
  return (
    <label className="grid grid-cols-[1fr_auto] items-center gap-x-3 gap-y-1 text-[13px] text-zinc-400">
      <span>{label}</span>
      <span className="font-black italic tabular-nums text-white">{Number.isInteger(step) ? value : value.toFixed(step < 0.1 ? 2 : 1)}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="col-span-2 accent-blue-500"
      />
    </label>
  );
}

function Toggle({ on, label, onToggle }: { on: boolean; label: string; onToggle: () => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
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

function Section({ title, toggle, children }: { title: string; toggle?: ReactNode; children?: ReactNode }) {
  return (
    <section className="flex flex-col gap-3 rounded-[22px] bg-white/[0.03] p-4">
      <div className="flex items-center justify-between gap-3">
        <h3 className="m-0 text-[11px] font-bold uppercase tracking-[0.2em] text-zinc-500">{title}</h3>
        {toggle}
      </div>
      {children ? <div className="grid grid-cols-2 gap-x-5 gap-y-3">{children}</div> : null}
    </section>
  );
}

/** Every crosshair setting, with a live preview drawn by the overlay renderer. */
export default function CrosshairEditor({ crosshair: c, onChange }: Props) {
  const { t } = useTranslation();
  const set = (patch: Partial<Crosshair>) => onChange({ ...c, ...patch });
  const setLines = (key: "inner" | "outer", patch: Partial<Lines>) => onChange({ ...c, [key]: { ...c[key], ...patch } });

  const linesFields = (key: "inner" | "outer") => (
    <>
      <Slider label={t("xh.length")} value={c[key].length} min={0} max={20} step={0.5} onChange={(v) => setLines(key, { length: v })} />
      <Slider label={t("xh.thickness")} value={c[key].thickness} min={0.5} max={10} step={0.5} onChange={(v) => setLines(key, { thickness: v })} />
      <Slider label={t("xh.gap")} value={c[key].gap} min={-5} max={20} step={0.5} onChange={(v) => setLines(key, { gap: v })} />
      <Slider label={t("xh.opacity")} value={c[key].opacity} min={0} max={1} step={0.05} onChange={(v) => setLines(key, { opacity: v })} />
    </>
  );

  return (
    <div className="flex flex-col gap-4">
      <CrosshairPreview crosshair={c} className="h-48 w-full" debounceMs={60} />

      <div className="flex flex-wrap items-center gap-3">
        <input
          aria-label={t("xh.name")}
          value={c.name}
          onChange={(e) => set({ name: e.target.value })}
          className="h-11 min-w-0 flex-1 rounded-full bg-white/5 px-4 text-[15px] font-bold outline-none focus:bg-white/[0.08]"
        />
        <div className="flex items-center gap-1.5">
          {SWATCHES.map((hex) => (
            <button
              key={hex}
              type="button"
              aria-label={hex}
              aria-pressed={c.color.toUpperCase() === hex}
              onClick={() => set({ color: hex })}
              className={`h-7 w-7 rounded-full transition-transform hover:scale-110 ${
                c.color.toUpperCase() === hex ? "ring-2 ring-white ring-offset-2 ring-offset-black" : ""
              }`}
              style={{ background: hex }}
            />
          ))}
          <label className="relative h-7 w-7 overflow-hidden rounded-full bg-[conic-gradient(red,yellow,lime,cyan,blue,magenta,red)]" title={t("xh.customColor")}>
            <input type="color" value={c.color} onChange={(e) => set({ color: e.target.value.toUpperCase() })} className="absolute inset-0 opacity-0" />
          </label>
        </div>
      </div>

      {c.image ? (
        <Section title={t("xh.image")}>
          <Slider label={t("xh.size")} value={c.imageScale} min={0.1} max={3} step={0.05} onChange={(v) => set({ imageScale: v })} />
          <Slider label={t("xh.opacity")} value={c.imageOpacity} min={0} max={1} step={0.05} onChange={(v) => set({ imageOpacity: v })} />
        </Section>
      ) : (
        <>
          <Section
            title={t("xh.lines")}
            toggle={
              <div className="flex gap-2">
                <Toggle on={c.tStyle} label={t("xh.tStyle")} onToggle={() => set({ tStyle: !c.tStyle })} />
                <Toggle on={c.inner.enabled} label={t("xh.shown")} onToggle={() => setLines("inner", { enabled: !c.inner.enabled })} />
              </div>
            }
          >
            {c.inner.enabled ? linesFields("inner") : null}
          </Section>
          <Section
            title={t("xh.outline")}
            toggle={<Toggle on={c.outline.enabled} label={t("xh.shown")} onToggle={() => set({ outline: { ...c.outline, enabled: !c.outline.enabled } })} />}
          >
            {c.outline.enabled ? (
              <>
                <Slider label={t("xh.thickness")} value={c.outline.thickness} min={0.5} max={4} step={0.5} onChange={(v) => set({ outline: { ...c.outline, thickness: v } })} />
                <Slider label={t("xh.opacity")} value={c.outline.opacity} min={0} max={1} step={0.05} onChange={(v) => set({ outline: { ...c.outline, opacity: v } })} />
              </>
            ) : null}
          </Section>
          <Section
            title={t("xh.dot")}
            toggle={<Toggle on={c.dot.enabled} label={t("xh.shown")} onToggle={() => set({ dot: { ...c.dot, enabled: !c.dot.enabled } })} />}
          >
            {c.dot.enabled ? (
              <>
                <Slider label={t("xh.size")} value={c.dot.size} min={1} max={12} step={0.5} onChange={(v) => set({ dot: { ...c.dot, size: v } })} />
                <Slider label={t("xh.opacity")} value={c.dot.opacity} min={0} max={1} step={0.05} onChange={(v) => set({ dot: { ...c.dot, opacity: v } })} />
              </>
            ) : null}
          </Section>
          <Section
            title={t("xh.circle")}
            toggle={<Toggle on={c.circle.enabled} label={t("xh.shown")} onToggle={() => set({ circle: { ...c.circle, enabled: !c.circle.enabled } })} />}
          >
            {c.circle.enabled ? (
              <>
                <Slider label={t("xh.radius")} value={c.circle.radius} min={2} max={40} step={0.5} onChange={(v) => set({ circle: { ...c.circle, radius: v } })} />
                <Slider label={t("xh.thickness")} value={c.circle.thickness} min={1} max={8} step={0.5} onChange={(v) => set({ circle: { ...c.circle, thickness: v } })} />
              </>
            ) : null}
          </Section>
          <Section
            title={t("xh.outer")}
            toggle={<Toggle on={c.outer.enabled} label={t("xh.shown")} onToggle={() => setLines("outer", { enabled: !c.outer.enabled })} />}
          >
            {c.outer.enabled ? linesFields("outer") : null}
          </Section>
        </>
      )}

      <Section title={t("xh.offset")}>
        <Slider label="X" value={c.offsetX} min={-50} max={50} step={1} onChange={(v) => set({ offsetX: v })} />
        <Slider label="Y" value={c.offsetY} min={-50} max={50} step={1} onChange={(v) => set({ offsetY: v })} />
      </Section>
    </div>
  );
}

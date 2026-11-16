import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { aspectLabel, formatResolution } from "../lib/format";
import type { DisplayMode } from "../lib/types";
import AspectFrame from "./AspectFrame";

interface Props {
  label: string;
  width: number;
  height: number;
  options: DisplayMode[];
  screen: { width: number; height: number };
  onChange: (width: number, height: number) => void;
}

function RatioChip({ width, height }: { width: number; height: number }) {
  const ratio = aspectLabel(width, height);
  const isFourThree = ratio === "4:3";
  return (
    <span
      className={`rounded-full px-2 py-0.5 text-[11px] font-bold leading-4 ${
        isFourThree ? "bg-blue-600/20 text-blue-300" : "bg-white/5 text-zinc-400"
      }`}
    >
      {ratio}
    </span>
  );
}

/** Resolution choice showing, for each option, its shape, ratio and the refresh rate REZ will use. */
export default function ResolutionPicker({ label, width, height, options, screen, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const [dropUp, setDropUp] = useState(false);
  const selectedIndex = Math.max(0, options.findIndex((o) => o.width === width && o.height === height));
  const [activeIndex, setActiveIndex] = useState(selectedIndex);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const listId = useId();
  const current = options.find((o) => o.width === width && o.height === height);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (root.current && !root.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    list.current?.focus();
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  useEffect(() => {
    if (open) list.current?.querySelector(`[data-index="${activeIndex}"]`)?.scrollIntoView({ block: "nearest" });
  }, [open, activeIndex]);

  const openList = () => {
    const rect = trigger.current?.getBoundingClientRect();
    setDropUp(rect ? window.innerHeight - rect.bottom < 340 : false);
    setActiveIndex(selectedIndex);
    setOpen(true);
  };

  const choose = (i: number) => {
    const o = options[i];
    if (o) onChange(o.width, o.height);
    setOpen(false);
    trigger.current?.focus();
  };

  const onListKey = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") setActiveIndex((i) => Math.min(options.length - 1, i + 1));
    else if (e.key === "ArrowUp") setActiveIndex((i) => Math.max(0, i - 1));
    else if (e.key === "Home") setActiveIndex(0);
    else if (e.key === "End") setActiveIndex(options.length - 1);
    else if (e.key === "Enter" || e.key === " ") choose(activeIndex);
    else if (e.key === "Escape" || e.key === "Tab") {
      setOpen(false);
      if (e.key === "Escape") trigger.current?.focus();
      return;
    } else return;
    e.preventDefault();
  };

  return (
    <div ref={root} className="relative">
      <button
        ref={trigger}
        type="button"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => (open ? setOpen(false) : openList())}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            openList();
          }
        }}
        className="flex h-14 min-w-[188px] items-center gap-3 rounded-[18px] bg-white/5 py-2 pl-3 pr-4 text-left transition-colors hover:bg-white/[0.08]"
      >
        <AspectFrame width={width} height={height} screen={screen} size={22} />
        <span className="flex flex-col">
          <span className="text-[17px] font-black italic leading-5 tracking-tight tabular-nums">{formatResolution(width, height)}</span>
          <span className="flex items-center gap-1.5 text-[11px] font-bold leading-4 text-zinc-400">
            {aspectLabel(width, height)}
            <span aria-hidden="true" className="h-1 w-1 rounded-full bg-zinc-600" />
            <span className="text-blue-300 tabular-nums">{current ? `${current.refreshHz} Hz` : "— Hz"}</span>
          </span>
        </span>
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true" className={`ml-auto text-zinc-400 transition-transform ${open ? "rotate-180" : ""}`}>
          <path d="M2 4l4 4 4-4" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      {open ? (
        <ul
          ref={list}
          id={listId}
          role="listbox"
          tabIndex={-1}
          aria-label={label}
          aria-activedescendant={`${listId}-${activeIndex}`}
          onKeyDown={onListKey}
          className={`custom-scrollbar absolute right-0 z-50 m-0 max-h-80 w-80 list-none overflow-y-auto rounded-[24px] bg-[rgba(18,18,21,0.97)] p-2 shadow-[0_30px_60px_-15px_rgba(0,0,0,0.8)] backdrop-blur-2xl outline-none ${
            dropUp ? "bottom-full mb-2" : "top-full mt-2"
          }`}
        >
          {options.map((o, i) => {
            const selected = i === selectedIndex;
            return (
              <li
                key={`${o.width}x${o.height}`}
                id={`${listId}-${i}`}
                data-index={i}
                role="option"
                aria-selected={selected}
                onMouseEnter={() => setActiveIndex(i)}
                onClick={() => choose(i)}
                className={`flex items-center gap-3 rounded-[14px] px-3 py-2 transition-colors ${
                  i === activeIndex ? "bg-white/[0.08]" : ""
                } ${selected ? "text-white" : "text-zinc-200"}`}
              >
                <AspectFrame width={o.width} height={o.height} screen={screen} size={18} active={selected} />
                <span className="text-[15px] font-black italic tabular-nums tracking-tight">{formatResolution(o.width, o.height)}</span>
                <RatioChip width={o.width} height={o.height} />
                <span className="ml-auto text-xs font-semibold tabular-nums text-zinc-400">{o.refreshHz} Hz</span>
              </li>
            );
          })}
        </ul>
      ) : null}
    </div>
  );
}

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { setAutostart, setCrosshairSettings } from "../lib/api";
import type { Config } from "../lib/types";

interface Props {
  config: Config;
  onChanged: () => void;
}

function Key({ children }: { children: string }) {
  return (
    <kbd className="rounded-lg bg-white/[0.06] px-2 py-1 font-sans text-[10px] font-black uppercase tracking-wider text-white">
      {children}
    </kbd>
  );
}

function Switch({ on, label, onToggle }: { on: boolean; label: string; onToggle: () => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={onToggle}
      className={`relative h-7 w-12 shrink-0 rounded-full transition-colors ${on ? "bg-primary" : "bg-white/10"}`}
    >
      <span
        aria-hidden="true"
        className={`absolute top-1 h-5 w-5 rounded-full bg-white transition-[left] duration-150 ${on ? "left-6" : "left-1"}`}
      />
    </button>
  );
}

function Row({ title, hint, children }: { title: string; hint: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 px-2.5 py-2.5">
      <div>
        <p className="m-0 font-semibold text-white">{title}</p>
        <p className="m-0 text-zinc-500">{hint}</p>
      </div>
      {children}
    </div>
  );
}

const shortcut = (letter: string) => (
  <span className="flex shrink-0 gap-1">
    <Key>Ctrl</Key>
    <Key>Alt</Key>
    <Key>Shift</Key>
    <Key>{letter}</Key>
  </span>
);

export default function Settings({ config, onChanged }: Props) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);
  const s = config.settings;

  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(String(err));
    }
    onChanged();
  };

  return (
    <section className="flex max-w-[680px] flex-col gap-5">
      <h2 className="display m-0 text-[36px] leading-[40px]">
        {t("settings.titleStart")} <span className="display-accent">{t("settings.titleEnd")}</span>
      </h2>

      <div className="rounded-[24px] bg-white/[0.03] p-2.5">
        <Row title={t("settings.autostart")} hint={t("settings.autostartHint")}>
          <Switch on={s.autostart} label={t("settings.autostart")} onToggle={() => void run(() => setAutostart(!s.autostart))} />
        </Row>
        <Row title={t("settings.hotkeyTitle")} hint={t("settings.hotkey")}>
          {shortcut("R")}
        </Row>
      </div>

      <h3 className="m-0 text-[11px] font-bold uppercase tracking-[0.2em] text-zinc-500">{t("settings.crosshairTitle")}</h3>
      <div className="rounded-[24px] bg-white/[0.03] p-2.5">
        <Row title={t("settings.undistort")} hint={t("settings.undistortHint")}>
          <Switch
            on={s.crosshairUndistort}
            label={t("settings.undistort")}
            onToggle={() => void run(() => setCrosshairSettings(!s.crosshairUndistort, s.hideCrosshairWithFaceit))}
          />
        </Row>
        <Row title={t("settings.faceit")} hint={t("settings.faceitHint")}>
          <Switch
            on={s.hideCrosshairWithFaceit}
            label={t("settings.faceit")}
            onToggle={() => void run(() => setCrosshairSettings(s.crosshairUndistort, !s.hideCrosshairWithFaceit))}
          />
        </Row>
        <Row title={t("settings.crosshairTitle")} hint={t("settings.crosshairHotkey")}>
          {shortcut("X")}
        </Row>
      </div>

      {error ? (
        <p role="alert" className="m-0 rounded-xl bg-red-500/10 px-4 py-3 text-red-500">
          {error}
        </p>
      ) : null}
      <p className="m-0 leading-relaxed text-zinc-400">{t("settings.trayHint")}</p>
    </section>
  );
}

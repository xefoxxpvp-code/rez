import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import RezLogo from "./components/RezLogo";
import StatusBar from "./components/StatusBar";
import { getConfig, getStatus, listDisplays, onStatus } from "./lib/api";
import type { Config, DisplayInfo, StatusEvent } from "./lib/types";
import CrosshairPage from "./pages/Crosshair";
import Games from "./pages/Games";
import Settings from "./pages/Settings";

type Tab = "games" | "crosshair" | "settings";

export default function App() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("games");
  const [config, setConfig] = useState<Config | null>(null);
  const [display, setDisplay] = useState<DisplayInfo | null>(null);
  const [status, setStatus] = useState<StatusEvent>({ kind: "idle" });
  const [loadError, setLoadError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    getConfig()
      .then(setConfig)
      .catch((e) => setLoadError(String(e)));
  }, []);

  useEffect(() => {
    refresh();
    listDisplays()
      .then((ds) => setDisplay(ds.find((d) => d.primary) ?? ds[0] ?? null))
      .catch((e) => setLoadError(t("games.loadError", { error: String(e) })));
    getStatus()
      .then(setStatus)
      .catch(() => {});
    const unlisten = onStatus(setStatus);
    return () => {
      void unlisten.then((f) => f());
    };
  }, [refresh, t]);

  const screen = display ? { width: display.native.width, height: display.native.height } : { width: 16, height: 9 };

  const tabClass = (name: Tab) =>
    `relative inline-flex h-11 items-center rounded-full px-4 text-[13px] font-bold uppercase tracking-wide transition-colors ${
      tab === name ? "bg-white/10 text-white" : "text-zinc-400 hover:text-white"
    }`;

  return (
    <div className="relative flex h-full flex-col overflow-hidden bg-black">
      {/* Soft brand halo behind the header (DESIGN.md "Halos"). */}
      <span
        aria-hidden="true"
        className="pointer-events-none absolute -left-32 -top-24 h-80 w-80"
        style={{ background: "radial-gradient(closest-side, rgba(37,99,235,.2), transparent)" }}
      />
      <header className="relative z-10 flex items-center justify-between gap-4 px-8 pt-6">
        <div className="flex items-center gap-3">
          <RezLogo size={40} />
          <p className="display m-0 text-[28px] leading-none">REZ</p>
        </div>
        <nav className="flex gap-1 rounded-full bg-white/[0.04] p-1" aria-label="REZ">
          <button type="button" className={tabClass("games")} aria-current={tab === "games"} onClick={() => setTab("games")}>
            {t("tabs.games")}
          </button>
          <button
            type="button"
            className={tabClass("crosshair")}
            aria-current={tab === "crosshair"}
            onClick={() => setTab("crosshair")}
          >
            {t("tabs.crosshair")}
          </button>
          <button
            type="button"
            className={tabClass("settings")}
            aria-current={tab === "settings"}
            onClick={() => setTab("settings")}
          >
            {t("tabs.settings")}
          </button>
        </nav>
      </header>
      <main className="custom-scrollbar relative z-10 flex flex-1 flex-col gap-8 overflow-y-auto px-8 pb-8 pt-6">
        <StatusBar status={status} games={config?.games ?? []} screen={screen} />
        {loadError ? (
          <p role="alert" className="m-0 rounded-xl bg-red-500/10 px-4 py-3 text-red-500">
            {loadError}
          </p>
        ) : null}
        {config ? (
          tab === "games" ? (
            <Games
              config={config}
              resolutions={display?.resolutions ?? []}
              display={display}
              screen={screen}
              status={status}
              onChanged={refresh}
            />
          ) : tab === "crosshair" ? (
            <CrosshairPage config={config} onChanged={refresh} />
          ) : (
            <Settings config={config} onChanged={refresh} />
          )
        ) : null}
      </main>
    </div>
  );
}

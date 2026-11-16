import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import StatusBar from "./components/StatusBar";
import { getConfig, getStatus, listDisplays, onStatus } from "./lib/api";
import type { Config, DisplayInfo, StatusEvent } from "./lib/types";
import Games from "./pages/Games";
import Settings from "./pages/Settings";

type Tab = "games" | "settings";

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
    `border-b-2 px-1 pb-2 font-display text-[17px] font-semibold ${
      tab === name ? "border-ink text-ink" : "border-transparent text-muted hover:text-ink"
    }`;

  return (
    <div className="flex h-full flex-col">
      <StatusBar status={status} games={config?.games ?? []} screen={screen} />
      <nav className="flex gap-6 px-6 pt-5" aria-label="REZ">
        <button type="button" className={tabClass("games")} aria-current={tab === "games"} onClick={() => setTab("games")}>
          {t("tabs.games")}
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
      <main className="flex-1 overflow-y-auto px-6 py-5">
        {loadError ? (
          <p role="alert" className="m-0 mb-3 text-danger">
            {loadError}
          </p>
        ) : null}
        {config ? (
          tab === "games" ? (
            <Games
              config={config}
              resolutions={display?.resolutions ?? []}
              screen={screen}
              status={status}
              onChanged={refresh}
            />
          ) : (
            <Settings config={config} onChanged={refresh} />
          )
        ) : null}
      </main>
    </div>
  );
}

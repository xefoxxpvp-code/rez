import { useState } from "react";
import { useTranslation } from "react-i18next";
import { setAutostart } from "../lib/api";
import type { Config } from "../lib/types";

interface Props {
  config: Config;
  onChanged: () => void;
}

export default function Settings({ config, onChanged }: Props) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);

  const toggleAutostart = async (enabled: boolean) => {
    setError(null);
    try {
      await setAutostart(enabled);
    } catch (err) {
      setError(String(err));
    }
    onChanged();
  };

  return (
    <section className="flex max-w-[64ch] flex-col gap-6">
      <div>
        <label className="flex items-center gap-2 font-semibold">
          <input
            type="checkbox"
            className="accent-signal"
            checked={config.settings.autostart}
            onChange={(e) => void toggleAutostart(e.target.checked)}
          />
          {t("settings.autostart")}
        </label>
        <p className="m-0 mt-1 pl-6 text-muted">{t("settings.autostartHint")}</p>
        {error ? (
          <p role="alert" className="m-0 mt-1 pl-6 text-danger">
            {error}
          </p>
        ) : null}
      </div>
      <div>
        <h2 className="m-0 font-display text-lg font-semibold">{t("settings.hotkeyTitle")}</h2>
        <p className="m-0 mt-1 text-muted">{t("settings.hotkey")}</p>
      </div>
      <p className="m-0 text-muted">{t("settings.trayHint")}</p>
    </section>
  );
}

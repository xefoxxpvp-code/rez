import { useTranslation } from "react-i18next";
import { updateLabel, type UpdateState } from "../lib/update";

interface Props {
  state: UpdateState;
  onInstall: () => void;
}

/** Offers the new version in place: one click downloads, installs and restarts REZ. */
export default function UpdateBanner({ state, onInstall }: Props) {
  const { t } = useTranslation();
  if (state.kind !== "available" && state.kind !== "installing") return null;

  return (
    <div role="status" className="flex items-center gap-4 rounded-[24px] bg-blue-600/15 px-5 py-3">
      <span aria-hidden="true" className="h-2 w-2 shrink-0 rounded-full bg-blue-400 shadow-[0_0_12px_rgba(96,165,250,0.8)]" />
      <div className="min-w-0 flex-1">
        <p className="m-0 font-bold text-white">
          {state.kind === "available" ? updateLabel(state.update.version) : state.label}
        </p>
        <p className="m-0 text-[13px] text-blue-300/80">{t("update.hint")}</p>
      </div>
      {state.kind === "available" ? (
        <button
          type="button"
          onClick={onInstall}
          className="h-11 shrink-0 rounded-lg border-2 border-primary/50 bg-primary px-5 text-sm font-medium text-white shadow-lg transition-colors hover:border-primary hover:bg-primary-hover"
        >
          {t("update.install")}
        </button>
      ) : null}
    </div>
  );
}

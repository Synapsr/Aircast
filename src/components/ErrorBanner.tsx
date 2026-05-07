import { AlertTriangle, Settings as SettingsIcon, X } from "lucide-react";
import { useT } from "@/i18n/context";

interface Props {
  message: string;
  onDismiss: () => void;
  onOpenSetup?: () => void;
  showSetup?: boolean;
}

export function ErrorBanner({ message, onDismiss, onOpenSetup, showSetup = true }: Props) {
  const { t } = useT();
  return (
    <div className="flex items-start gap-3 rounded-xl bg-rose-500/15 px-4 py-3 text-rose-100 ring-1 ring-rose-500/30">
      <div className="mt-0.5 shrink-0 rounded-full bg-rose-500 p-1.5 text-white">
        <AlertTriangle className="h-3.5 w-3.5" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium leading-snug">{message}</div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {showSetup && onOpenSetup && (
          <button
            type="button"
            onClick={onOpenSetup}
            className="flex items-center gap-1.5 rounded-full bg-rose-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-rose-400"
          >
            <SettingsIcon className="h-3.5 w-3.5" />
            {t("errors.openSetup")}
          </button>
        )}
        <button
          type="button"
          onClick={onDismiss}
          title={t("common.dismiss")}
          className="rounded-full p-1.5 text-rose-200 hover:bg-rose-500/20 hover:text-white"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}

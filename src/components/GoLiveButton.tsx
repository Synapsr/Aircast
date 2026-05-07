import { Loader2, Radio, Square } from "lucide-react";
import { useT } from "@/i18n/context";
import type { StreamStatus } from "@/types";

interface Props {
  status: StreamStatus;
  canStart: boolean;
  onStart: () => void;
  onStop: () => void;
}

export function GoLiveButton({ status, canStart, onStart, onStop }: Props) {
  const { t } = useT();
  const isStarting = status.kind === "connecting";
  const isLive = status.kind === "live" || status.kind === "reconnecting";
  const isActive = isStarting || isLive;

  const handleClick = () => {
    if (isActive) onStop();
    else onStart();
  };

  return (
    <button
      type="button"
      onClick={handleClick}
      disabled={!isActive && !canStart}
      className={[
        "relative w-full overflow-hidden rounded-xl px-6 py-4 text-base font-semibold transition-all",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950",
        isActive
          ? "bg-rose-500 text-white shadow-lg shadow-rose-500/30 hover:bg-rose-400 focus-visible:ring-rose-400"
          : canStart
            ? "bg-rose-500 text-white shadow-md shadow-rose-500/20 hover:bg-rose-400 hover:shadow-lg hover:shadow-rose-500/30 focus-visible:ring-rose-400"
            : "cursor-not-allowed bg-zinc-800 text-zinc-500",
      ].join(" ")}
    >
      <span className="flex items-center justify-center gap-2.5">
        {isStarting ? (
          <Loader2 className="h-5 w-5 animate-spin" />
        ) : isLive ? (
          <Square className="h-4 w-4 fill-current" />
        ) : (
          <Radio className="h-5 w-5" />
        )}
        <span>
          {isStarting ? t("goLive.connecting") : isLive ? t("goLive.stop") : t("goLive.go")}
        </span>
      </span>
    </button>
  );
}

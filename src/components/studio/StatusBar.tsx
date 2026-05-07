import { Volume2, VolumeX } from "lucide-react";
import { useMonitorMuted } from "@/hooks/useMonitorMuted";
import { useT } from "@/i18n/context";
import type { StreamStatus } from "@/types";

interface Props {
  status: StreamStatus;
  micOpen: boolean;
  deviceName: string | null;
  onAboutClick: () => void;
}

export function StatusBar({ status, micOpen, deviceName, onAboutClick }: Props) {
  const { t } = useT();
  const { muted, toggle } = useMonitorMuted();

  const stream = (() => {
    switch (status.kind) {
      case "idle":
        return { dot: "bg-zinc-600", text: t("status.offAir"), className: "text-zinc-400" };
      case "connecting":
        return { dot: "bg-amber-400 animate-pulse", text: t("status.connecting"), className: "text-amber-300" };
      case "live":
        return { dot: "bg-rose-500 animate-pulse", text: t("status.live"), className: "text-rose-300" };
      case "reconnecting":
        return { dot: "bg-amber-400 animate-pulse", text: t("status.reconnecting"), className: "text-amber-300" };
      case "error":
        return { dot: "bg-red-500", text: status.message, className: "text-red-300" };
    }
  })();

  return (
    <footer className="grid grid-cols-3 items-center gap-3 border-t border-zinc-900 bg-zinc-950 px-5 py-3 text-xs">
      <div className="flex items-center gap-3 justify-self-start">
        <span className="flex items-center gap-2">
          <span className={`h-2 w-2 rounded-full ${stream.dot}`} />
          <span className={`font-medium ${stream.className}`}>{stream.text}</span>
        </span>
        <span className="h-3 w-px bg-zinc-800" />
        <span className="flex items-center gap-2">
          <span className={`h-2 w-2 rounded-full ${micOpen ? "bg-rose-500" : "bg-zinc-600"}`} />
          <span className={micOpen ? "text-rose-300" : "text-zinc-400"}>
            {micOpen ? t("status.micOpen") : t("status.micMuted")}
          </span>
        </span>
      </div>

      <button
        type="button"
        onClick={onAboutClick}
        className="justify-self-center text-[11px] text-zinc-500 transition-colors hover:text-zinc-300"
      >
        {t("about.proudly")}{" "}
        <span className="font-semibold text-zinc-300 hover:text-rose-300">Synapsr</span>
      </button>

      <div className="flex items-center gap-3 justify-self-end">
        <span className="truncate text-zinc-500">{deviceName ?? t("status.noInput")}</span>
        <button
          type="button"
          onClick={toggle}
          title={muted ? t("status.monitorUnmuteHint") : t("status.monitorMuteHint")}
          className={[
            "flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-medium transition-colors",
            muted
              ? "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
              : "bg-emerald-500 text-white hover:bg-emerald-400",
          ].join(" ")}
        >
          {muted ? <VolumeX className="h-3.5 w-3.5" /> : <Volume2 className="h-3.5 w-3.5" />}
          <span>{muted ? t("status.monitorOff") : t("status.monitorOn")}</span>
        </button>
      </div>
    </footer>
  );
}

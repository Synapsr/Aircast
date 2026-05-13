import { Cable, Volume2, VolumeX } from "lucide-react";
import { useMonitorMuted } from "@/hooks/useMonitorMuted";
import { useUpstreamStatus } from "@/hooks/useUpstreamStatus";
import { useT } from "@/i18n/context";
import type { AppMode, StreamStatus } from "@/types";

interface Props {
  status: StreamStatus;
  micOpen: boolean;
  deviceName: string | null;
  onAboutClick: () => void;
  /** The active app mode drives what input chip we render: a "mic open/muted"
   *  badge in Simple/Studio, or a relay-upstream pill in Relay. */
  mode: AppMode;
}

export function StatusBar({ status, micOpen, deviceName, onAboutClick, mode }: Props) {
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
        {mode === "relay" ? <RelayInputBadge /> : <MicBadge open={micOpen} />}
      </div>

      <button
        type="button"
        onClick={onAboutClick}
        title="Aircast"
        className="justify-self-center font-mono text-[11px] tracking-tight text-zinc-500 transition-colors hover:text-zinc-300"
      >
        v{__APP_VERSION__}
      </button>

      <div className="flex items-center gap-3 justify-self-end">
        {/* The device name used to live here, but it duplicated the header
            DevicePill (Simple) and the MicPanel (Studio). The status bar is
            for *state* (stream, mic gate, monitor) — selection lives in the
            picker above. */}
        {!deviceName && (
          <span className="truncate text-zinc-500">{t("status.noInput")}</span>
        )}
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

function MicBadge({ open }: { open: boolean }) {
  const { t } = useT();
  return (
    <span className="flex items-center gap-2">
      <span className={`h-2 w-2 rounded-full ${open ? "bg-rose-500" : "bg-zinc-600"}`} />
      <span className={open ? "text-rose-300" : "text-zinc-400"}>
        {open ? t("status.micOpen") : t("status.micMuted")}
      </span>
    </span>
  );
}

/// Mirrors the relay upstream state in the status bar so the user has the
/// same at-a-glance feedback in Relay mode as Simple/Studio gives via the
/// mic indicator.
function RelayInputBadge() {
  const { t } = useT();
  const upstream = useUpstreamStatus();
  const { dot, label, pulse } = (() => {
    switch (upstream) {
      case "streaming":
        return { dot: "bg-emerald-500", label: t("relay.status.streaming"), pulse: true };
      case "connecting":
        return { dot: "bg-amber-400", label: t("relay.status.connecting"), pulse: true };
      case "reconnecting":
        return { dot: "bg-amber-400", label: t("relay.status.reconnecting"), pulse: true };
      case "stopped":
      case "idle":
      default:
        return { dot: "bg-zinc-600", label: t("relay.status.idle"), pulse: false };
    }
  })();
  return (
    <span className="flex items-center gap-2">
      <span className="relative flex h-2 w-2">
        {pulse && (
          <span
            className={`absolute inline-flex h-full w-full animate-ping rounded-full ${dot} opacity-60`}
          />
        )}
        <span className={`relative inline-flex h-2 w-2 rounded-full ${dot}`} />
      </span>
      <Cable className="h-3 w-3 text-zinc-500" />
      <span className={upstream === "streaming" ? "text-emerald-300" : "text-zinc-400"}>
        {label}
      </span>
    </span>
  );
}

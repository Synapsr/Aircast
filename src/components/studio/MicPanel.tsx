import { Mic, MicOff } from "lucide-react";
import { DevicePill } from "@/components/DevicePill";
import { useT } from "@/i18n/context";

interface Props {
  /** Currently selected input device id (drives the selector). */
  deviceId: string | null;
  onDeviceChange: (id: string) => void;
  /** Mic gate state (Studio mode). */
  open: boolean;
  onToggle: () => void;
  /** Live mic level 0..1, gated by `open`. */
  level: number;
}

/**
 * Studio-mode microphone control. Combines what was previously three places
 * (header pill, mic toggle, status bar device label) into a single block:
 *
 *   ┌──────────────────────────────────────┐
 *   │ MICRO                                │
 *   │ ┌────────────────────────────────┐   │
 *   │ │ 🎤  Micro MacBook Air        ▼ │   │  ← selector
 *   │ └────────────────────────────────┘   │
 *   │ ┌────────────────────────────────┐   │
 *   │ │  ▣  Tap to open mic            │   │  ← toggle
 *   │ │ [─── live VU ──]               │   │
 *   │ └────────────────────────────────┘   │
 *   └──────────────────────────────────────┘
 */
export function MicPanel({ deviceId, onDeviceChange, open, onToggle, level }: Props) {
  const { t } = useT();
  const pct = Math.round(Math.min(100, Math.max(0, level * 100)));
  return (
    <div className="flex flex-col gap-3">
      <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
        {t("mic.panelLabel")}
      </span>

      <DevicePill value={deviceId} onChange={onDeviceChange} variant="row" />

      <button
        type="button"
        onClick={onToggle}
        className={[
          "group flex w-full items-center gap-3 rounded-xl px-4 py-3 transition-all",
          open
            ? "bg-rose-500 text-white shadow-md shadow-rose-500/30"
            : "bg-zinc-800 text-zinc-300 hover:bg-zinc-700",
        ].join(" ")}
      >
        <div
          className={[
            "flex h-10 w-10 shrink-0 items-center justify-center rounded-full transition-colors",
            open
              ? "bg-white/20"
              : "bg-zinc-900 text-zinc-400 group-hover:bg-zinc-800",
          ].join(" ")}
        >
          {open ? <Mic className="h-5 w-5" /> : <MicOff className="h-5 w-5" />}
        </div>
        <div className="flex flex-1 flex-col items-start gap-1.5">
          <span className="text-sm font-semibold">
            {open ? t("mic.open") : t("mic.tapToOpen")}
          </span>
          <div
            className={`relative h-1.5 w-full overflow-hidden rounded-full ${
              open ? "bg-white/20" : "bg-zinc-900"
            }`}
          >
            <div
              className="absolute inset-y-0 left-0 bg-gradient-to-r from-emerald-300 via-amber-200 to-white transition-[width] duration-75"
              style={{ width: `${open ? pct : 0}%` }}
            />
          </div>
        </div>
      </button>
    </div>
  );
}

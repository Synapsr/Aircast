import { useEffect, useMemo, useState } from "react";
import { Cable, ChevronDown, Plus, Settings as SettingsIcon } from "lucide-react";
import { GoLiveButton } from "@/components/GoLiveButton";
import { StatusBadge } from "@/components/StatusBadge";
import { VuMeter } from "@/components/VuMeter";
import { useUpstreamStatus } from "@/hooks/useUpstreamStatus";
import { useT } from "@/i18n/context";
import type { RelaySource, StreamConfig, StreamStatus, UpstreamStatus } from "@/types";

interface Props {
  level: number;
  vuActive: boolean;
  config: StreamConfig | null;
  status: StreamStatus;
  onStart: () => void;
  onStop: () => void;
  /** Open Setup at the relay-sources section so the user can add/edit URLs. */
  onOpenRelaySources: () => void;
  /** Active relay source persisted in settings, or null when none picked yet. */
  activeSourceName: string | null;
  /** Whole list of saved relay sources (rendered in the picker dropdown). */
  sources: RelaySource[];
  /** Called when the user picks a different source from the dropdown — the
   *  parent persists `activeSourceName` and triggers `startRelayInput`. */
  onPickSource: (name: string) => void;
}

/**
 * Relay mode UI — a stream-URL counterpart of Simple mode. Same vertical
 * rhythm (source pill at top, VU bar, big primary button, status badge at
 * the bottom) so users moving between modes feel oriented.
 *
 * The active relay source is picked from a dropdown of saved URLs; adding /
 * editing URLs lives in Setup → Relay sources. Hides the strip when no
 * source is configured and points the user to Setup with a clear CTA.
 */
export function RelayMode({
  level,
  vuActive,
  config,
  status,
  onStart,
  onStop,
  onOpenRelaySources,
  activeSourceName,
  sources,
  onPickSource,
}: Props) {
  const { t } = useT();
  const upstream = useUpstreamStatus();
  const hasSource = !!activeSourceName && sources.some((s) => s.name === activeSourceName);
  const canStart = hasSource && !!config && status.kind !== "connecting";

  const mountPath = config
    ? config.mount.startsWith("/")
      ? config.mount
      : `/${config.mount}`
    : null;

  return (
    <section className="flex flex-col items-center gap-7">
      {/* Source picker */}
      {sources.length === 0 ? (
        <EmptyState onOpenRelaySources={onOpenRelaySources} />
      ) : (
        <SourcePicker
          sources={sources}
          activeName={activeSourceName}
          onPick={onPickSource}
          onManage={onOpenRelaySources}
        />
      )}

      {/* Server destination pill (mirrors SimpleMode) */}
      {config ? (
        <div className="flex max-w-full items-center gap-2 rounded-full bg-zinc-900/80 px-4 py-2 text-xs ring-1 ring-zinc-800">
          <span className="text-zinc-500">{t("simple.serverPrefix")}</span>
          <span className="truncate text-zinc-200">
            {config.host}:{config.port}
            {mountPath}
          </span>
          <span className="hidden h-3 w-px bg-zinc-800 sm:block" />
          <span className="hidden shrink-0 text-zinc-500 sm:block">
            {config.format.toUpperCase()} · {config.bitrate} kbps
          </span>
        </div>
      ) : (
        <div className="rounded-full bg-zinc-900/60 px-4 py-2 text-xs text-zinc-500 ring-1 ring-zinc-800/80">
          {t("simple.noServer")}
        </div>
      )}

      {/* Upstream + VU */}
      <div className="flex w-full flex-col gap-2">
        <div className="flex items-center justify-between text-[11px] font-medium uppercase tracking-wider text-zinc-500">
          <span>{t("relay.upstreamLevel")}</span>
          <UpstreamPill status={upstream} />
        </div>
        <VuMeter level={level} active={vuActive && hasSource} />
      </div>

      {/* Primary action */}
      <div className="w-full">
        <GoLiveButton
          status={status}
          canStart={canStart}
          onStart={onStart}
          onStop={onStop}
        />
      </div>

      <StatusBadge status={status} />
    </section>
  );
}

function EmptyState({ onOpenRelaySources }: { onOpenRelaySources: () => void }) {
  const { t } = useT();
  return (
    <button
      type="button"
      onClick={onOpenRelaySources}
      className="flex w-full items-center justify-center gap-2 rounded-full bg-rose-500 px-4 py-3 text-sm font-semibold text-white shadow-md shadow-rose-500/20 transition-colors hover:bg-rose-400"
    >
      <Plus className="h-4 w-4" />
      <span>{t("relay.addFirstSource")}</span>
    </button>
  );
}

function SourcePicker({
  sources,
  activeName,
  onPick,
  onManage,
}: {
  sources: RelaySource[];
  activeName: string | null;
  onPick: (name: string) => void;
  onManage: () => void;
}) {
  const { t } = useT();
  const [open, setOpen] = useState(false);
  const containerRef = useMemo<{ current: HTMLDivElement | null }>(() => ({ current: null }), []);
  const active = sources.find((s) => s.name === activeName) ?? null;

  useEffect(() => {
    if (!open) return;
    function handleMouseDown(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", handleMouseDown);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleMouseDown);
      document.removeEventListener("keydown", handleKey);
    };
  }, [open, containerRef]);

  return (
    <div
      ref={(el) => {
        containerRef.current = el;
      }}
      className="relative w-full"
    >
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full items-center gap-3 rounded-xl bg-zinc-900 px-4 py-3 text-sm text-zinc-100 ring-1 ring-zinc-800 transition-colors hover:bg-zinc-800/80"
      >
        <Cable className="h-4 w-4 shrink-0 text-rose-400" />
        <div className="flex min-w-0 flex-1 flex-col items-start text-left">
          <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
            {t("relay.source")}
          </span>
          <span className="truncate">
            {active?.name ?? t("relay.pickSource")}
          </span>
        </div>
        <ChevronDown
          className={`h-4 w-4 shrink-0 text-zinc-500 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open && (
        <div className="absolute left-0 right-0 top-full z-50 mt-2 overflow-hidden rounded-xl bg-zinc-900 shadow-2xl ring-1 ring-zinc-800">
          <div className="flex items-center justify-between border-b border-zinc-800 px-3 py-2">
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
              {t("relay.sources")}
            </span>
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                onManage();
              }}
              className="flex items-center gap-1 rounded-full bg-zinc-800 px-2 py-1 text-[11px] font-medium text-zinc-200 hover:bg-zinc-700"
            >
              <SettingsIcon className="h-3 w-3" />
              {t("relay.manage")}
            </button>
          </div>
          <div className="max-h-72 overflow-y-auto p-1">
            {sources.map((s) => (
              <button
                key={s.name}
                type="button"
                onClick={() => {
                  onPick(s.name);
                  setOpen(false);
                }}
                className={[
                  "flex w-full flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left transition-colors",
                  s.name === activeName
                    ? "bg-zinc-800 text-zinc-100"
                    : "text-zinc-300 hover:bg-zinc-800/60",
                ].join(" ")}
              >
                <span className="flex w-full items-center gap-2">
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                      s.name === activeName ? "bg-rose-500" : "bg-zinc-700"
                    }`}
                  />
                  <span className="truncate text-sm">{s.name}</span>
                </span>
                <span className="ml-3.5 truncate text-[11px] text-zinc-500">
                  {s.url}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function UpstreamPill({ status }: { status: UpstreamStatus }) {
  const { t } = useT();
  const { dot, label, pulse } = (() => {
    switch (status) {
      case "streaming":
        return { dot: "bg-emerald-500", label: t("relay.status.streaming"), pulse: true };
      case "connecting":
        return { dot: "bg-amber-400", label: t("relay.status.connecting"), pulse: true };
      case "reconnecting":
        return { dot: "bg-amber-400", label: t("relay.status.reconnecting"), pulse: true };
      case "stopped":
        return { dot: "bg-zinc-600", label: t("relay.status.stopped"), pulse: false };
      case "idle":
      default:
        return { dot: "bg-zinc-700", label: t("relay.status.idle"), pulse: false };
    }
  })();
  return (
    <span className="flex items-center gap-1.5 normal-case tracking-normal">
      <span className="relative flex h-2 w-2">
        {pulse && (
          <span
            className={`absolute inline-flex h-full w-full animate-ping rounded-full ${dot} opacity-60`}
          />
        )}
        <span className={`relative inline-flex h-2 w-2 rounded-full ${dot}`} />
      </span>
      <span className="text-zinc-400">{label}</span>
    </span>
  );
}

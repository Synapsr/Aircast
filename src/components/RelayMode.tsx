import { Cable } from "lucide-react";
import { FlowArrow } from "@/components/FlowArrow";
import { GoLiveButton } from "@/components/GoLiveButton";
import { RelaySourceCard } from "@/components/RelaySourceCard";
import { ServerDestinationCard } from "@/components/ServerDestinationCard";
import { StatusBadge } from "@/components/StatusBadge";
import { VuMeter } from "@/components/VuMeter";
import { useUpstreamStatus } from "@/hooks/useUpstreamStatus";
import { useT } from "@/i18n/context";
import type { Preset, RelaySource, StreamConfig, StreamStatus, UpstreamStatus } from "@/types";

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
  // Server destination — same wiring as Simple mode.
  presets: Preset[];
  activePresetName: string | null;
  onSelectPreset: (name: string) => void;
  onManageServers: () => void;
}

/**
 * Relay mode displays the signal flow at a glance: upstream URL → server.
 * Identical layout to Simple mode (source card → arrow → destination
 * card → VU → Go Live → status) so muscle memory carries over between the
 * two source-driven modes.
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
  presets,
  activePresetName,
  onSelectPreset,
  onManageServers,
}: Props) {
  const { t } = useT();
  const upstream = useUpstreamStatus();
  const hasSource = !!activeSourceName && sources.some((s) => s.name === activeSourceName);
  const canStart = hasSource && !!config && status.kind !== "connecting";

  return (
    <section className="flex flex-col items-center gap-6">
      {/* Source → destination flow */}
      <div className="flex w-full flex-col gap-0">
        <RelaySourceCard
          sources={sources}
          activeName={activeSourceName}
          onPick={onPickSource}
          onManage={onOpenRelaySources}
        />
        <FlowArrow />
        <ServerDestinationCard
          presets={presets}
          activeName={activePresetName}
          onSelect={onSelectPreset}
          onManage={onManageServers}
        />
      </div>

      {/* Live meter + upstream status */}
      <div className="flex w-full flex-col gap-2 pt-2">
        <div className="flex items-center justify-between text-[11px] font-medium uppercase tracking-wider text-zinc-500">
          <span>{t("relay.upstreamLevel")}</span>
          <UpstreamPill status={upstream} />
        </div>
        <VuMeter level={level} active={vuActive && hasSource} />
      </div>

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
      <Cable className="h-3 w-3 text-zinc-500" />
      <span className="text-zinc-400">{label}</span>
    </span>
  );
}

import { FlowArrow } from "@/components/FlowArrow";
import { GoLiveButton } from "@/components/GoLiveButton";
import { MicSourceCard } from "@/components/MicSourceCard";
import { ServerDestinationCard } from "@/components/ServerDestinationCard";
import { StatusBadge } from "@/components/StatusBadge";
import { VuMeter } from "@/components/VuMeter";
import { useT } from "@/i18n/context";
import type { Preset, StreamConfig, StreamStatus } from "@/types";

interface Props {
  level: number;
  vuActive: boolean;
  config: StreamConfig | null;
  status: StreamStatus;
  onStart: () => void;
  onStop: () => void;
  /** Currently selected mic device id. */
  deviceId: string | null;
  onDeviceChange: (id: string) => void;
  /** Server presets list (drives the destination card dropdown). */
  presets: Preset[];
  /** Name of the currently active server preset. */
  activePresetName: string | null;
  /** Switch to another saved server preset by name. */
  onSelectPreset: (name: string) => void;
  /** Open Setup on the Servers tab — used for manage + empty-state CTA. */
  onManageServers: () => void;
}

/**
 * Simple mode displays the signal flow at a glance: mic → server.
 * Both cards are interactive — click the source to change device, click
 * the destination to switch server preset. VU + Go Live + status sit below.
 */
export function SimpleMode({
  level,
  vuActive,
  config,
  status,
  onStart,
  onStop,
  deviceId,
  onDeviceChange,
  presets,
  activePresetName,
  onSelectPreset,
  onManageServers,
}: Props) {
  const { t } = useT();
  const canStart = !!deviceId && !!config && status.kind !== "connecting";

  return (
    <section className="flex flex-col items-center gap-6">
      {/* Source → destination flow */}
      <div className="flex w-full flex-col gap-0">
        <MicSourceCard value={deviceId} onChange={onDeviceChange} />
        <FlowArrow />
        <ServerDestinationCard
          presets={presets}
          activeName={activePresetName}
          onSelect={onSelectPreset}
          onManage={onManageServers}
        />
      </div>

      {/* Live meter */}
      <div className="flex w-full flex-col gap-2 pt-2">
        <div className="flex items-center justify-between text-[11px] font-medium uppercase tracking-wider text-zinc-500">
          <span>{t("vu.label")}</span>
        </div>
        <VuMeter level={level} active={vuActive} />
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

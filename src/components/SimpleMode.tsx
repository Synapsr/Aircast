import { Server } from "lucide-react";
import { GoLiveButton } from "@/components/GoLiveButton";
import { StatusBadge } from "@/components/StatusBadge";
import { VuMeter } from "@/components/VuMeter";
import { useT } from "@/i18n/context";
import type { StreamConfig, StreamStatus } from "@/types";

interface Props {
  level: number;
  vuActive: boolean;
  config: StreamConfig | null;
  status: StreamStatus;
  onStart: () => void;
  onStop: () => void;
  deviceReady: boolean;
}

export function SimpleMode({
  level,
  vuActive,
  config,
  status,
  onStart,
  onStop,
  deviceReady,
}: Props) {
  const { t } = useT();
  const canStart = deviceReady && !!config && status.kind !== "connecting";
  const mountPath = config
    ? config.mount.startsWith("/")
      ? config.mount
      : `/${config.mount}`
    : null;

  return (
    <section className="flex flex-col items-center gap-7">
      {/* Server pill — minimal, subtle, but informative */}
      {config ? (
        <div className="flex max-w-full items-center gap-2 rounded-full bg-zinc-900/80 px-4 py-2 text-xs ring-1 ring-zinc-800">
          <Server className="h-3.5 w-3.5 shrink-0 text-zinc-500" />
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

      {/* VU meter — full width with a label above */}
      <div className="flex w-full flex-col gap-2">
        <div className="flex items-center justify-between text-[11px] font-medium uppercase tracking-wider text-zinc-500">
          <span>{t("vu.label")}</span>
        </div>
        <VuMeter level={level} active={vuActive} />
      </div>

      {/* Big primary action */}
      <div className="w-full">
        <GoLiveButton
          status={status}
          canStart={canStart}
          onStart={onStart}
          onStop={onStop}
        />
      </div>

      {/* Status under the button */}
      <StatusBadge status={status} />
    </section>
  );
}

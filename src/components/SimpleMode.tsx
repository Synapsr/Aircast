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

  return (
    <section className="flex flex-col gap-5 rounded-2xl bg-zinc-900 p-6">
      <div className="flex flex-col gap-2">
        <span className="text-sm font-medium text-zinc-300">{t("vu.label")}</span>
        <VuMeter level={level} active={vuActive} />
      </div>

      {config ? (
        <div className="flex items-center justify-between text-sm">
          <span className="truncate text-zinc-300">
            <span className="text-zinc-500">{t("simple.serverPrefix")} </span>
            {config.host}:{config.port}
            {config.mount.startsWith("/") ? config.mount : `/${config.mount}`}
          </span>
          <span className="shrink-0 pl-3 text-zinc-500">
            {config.format.toUpperCase()} · {config.bitrate} kbps
          </span>
        </div>
      ) : (
        <div className="text-sm text-zinc-500">{t("simple.noServer")}</div>
      )}

      <GoLiveButton
        status={status}
        canStart={canStart}
        onStart={onStart}
        onStop={onStop}
      />

      <div className="flex items-center justify-center pt-1">
        <StatusBadge status={status} />
      </div>
    </section>
  );
}

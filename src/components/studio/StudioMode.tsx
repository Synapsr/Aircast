import { GoLiveButton } from "@/components/GoLiveButton";
import { Cartoucheur } from "@/components/studio/Cartoucheur";
import { MicPanel } from "@/components/studio/MicPanel";
import { MusicQueue } from "@/components/studio/MusicQueue";
import { NowPlaying } from "@/components/studio/NowPlaying";
import { useCarts } from "@/hooks/useCarts";
import { useMicOpen } from "@/hooks/useMicOpen";
import { useMusic } from "@/hooks/useMusic";
import { useT } from "@/i18n/context";
import { api } from "@/lib/api";
import type { StreamConfig, StreamStatus } from "@/types";

interface Props {
  level: number;
  config: StreamConfig | null;
  status: StreamStatus;
  onStart: () => void;
  onStop: () => void;
  deviceReady: boolean;
  /** Currently selected input device id; the studio MicPanel exposes a
   *  selector so the user can change mic without going up to the header. */
  deviceId: string | null;
  onDeviceChange: (id: string) => void;
  /** Title currently broadcast to Icecast (drives the on-air chip in NowPlaying). */
  broadcastTitle: string;
  /** Open Setup directly at the broadcast metadata section. */
  onEditBroadcast: () => void;
}

export function StudioMode({
  level,
  config,
  status,
  onStart,
  onStop,
  deviceReady,
  deviceId,
  onDeviceChange,
  broadcastTitle,
  onEditBroadcast,
}: Props) {
  const { t } = useT();
  const { snapshot: music, refresh: refreshMusic } = useMusic();
  const { carts, refresh: refreshCarts } = useCarts();
  const { open: micOpen, toggle: toggleMic } = useMicOpen();

  const canStart = deviceReady && !!config && status.kind !== "connecting";

  return (
    <div className="grid min-h-0 flex-1 grid-cols-[1fr_340px] gap-4">
      {/* Left: now playing + queue */}
      <div className="flex min-h-0 flex-col gap-4">
        <NowPlaying
          snapshot={music}
          onPlay={() => api.musicPlay().then(refreshMusic)}
          onPause={() => api.musicPause().then(refreshMusic)}
          onStop={() => api.musicStop().then(refreshMusic)}
          onNext={() => api.musicNext().then(refreshMusic)}
          broadcastTitle={broadcastTitle}
          live={status.kind === "live"}
          onEditBroadcast={onEditBroadcast}
        />
        <MusicQueue snapshot={music} onChange={refreshMusic} />
      </div>

      {/* Right: carts + mic + broadcast control. Mic and Go Live live in
          separate cards because they're different concerns: the mic block
          owns the device + on-air gate, the broadcast block owns the
          stream lifecycle (which mixes mic + music + carts together). */}
      <div className="flex min-h-0 flex-col gap-4">
        <Cartoucheur carts={carts} onChange={refreshCarts} />

        <section className="rounded-2xl bg-zinc-900 p-5">
          <MicPanel
            deviceId={deviceId}
            onDeviceChange={onDeviceChange}
            open={micOpen}
            onToggle={toggleMic}
            level={micOpen ? level : 0}
          />
        </section>

        <section className="flex flex-col gap-3 rounded-2xl bg-zinc-900 p-5">
          <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
            {t("broadcast.sectionLabel")}
          </span>
          <GoLiveButton
            status={status}
            canStart={canStart}
            onStart={onStart}
            onStop={onStop}
          />
        </section>
      </div>
    </div>
  );
}

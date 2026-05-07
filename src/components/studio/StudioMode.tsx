import { GoLiveButton } from "@/components/GoLiveButton";
import { Cartoucheur } from "@/components/studio/Cartoucheur";
import { MicToggle } from "@/components/studio/MicToggle";
import { MusicQueue } from "@/components/studio/MusicQueue";
import { NowPlaying } from "@/components/studio/NowPlaying";
import { useCarts } from "@/hooks/useCarts";
import { useMicOpen } from "@/hooks/useMicOpen";
import { useMusic } from "@/hooks/useMusic";
import { api } from "@/lib/api";
import type { StreamConfig, StreamStatus } from "@/types";

interface Props {
  level: number;
  config: StreamConfig | null;
  status: StreamStatus;
  onStart: () => void;
  onStop: () => void;
  deviceReady: boolean;
}

export function StudioMode({
  level,
  config,
  status,
  onStart,
  onStop,
  deviceReady,
}: Props) {
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
        />
        <MusicQueue snapshot={music} onChange={refreshMusic} />
      </div>

      {/* Right: carts + controls */}
      <div className="flex min-h-0 flex-col gap-4">
        <Cartoucheur carts={carts} onChange={refreshCarts} />

        <section className="flex flex-col gap-3 rounded-2xl bg-zinc-900 p-5">
          <MicToggle open={micOpen} onToggle={toggleMic} level={micOpen ? level : 0} />
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

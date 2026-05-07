import { Pause, Play, SkipForward, Square } from "lucide-react";
import { useT } from "@/i18n/context";
import type { MusicSnapshot } from "@/types";

interface Props {
  snapshot: MusicSnapshot;
  onPlay: () => void;
  onPause: () => void;
  onStop: () => void;
  onNext: () => void;
}

export function NowPlaying({ snapshot, onPlay, onPause, onStop, onNext }: Props) {
  const { t } = useT();
  const current = snapshot.current;
  const isPlaying = snapshot.state === "playing";
  const nextTrack = snapshot.queue[0] ?? null;

  const elapsed = current?.elapsedSecs ?? 0;
  const duration = current?.durationSecs ?? null;
  const knowsDuration = duration !== null && duration > 0.1;
  const progressPct = current && knowsDuration
    ? Math.min(100, (elapsed / duration!) * 100)
    : 0;

  const hasContent = !!current || snapshot.queue.length > 0;
  const queueCountKey =
    snapshot.queue.length === 1
      ? "nowPlaying.tracksInQueue_one"
      : "nowPlaying.tracksInQueue_other";

  // Subtitle: what comes next
  let subtitle: React.ReactNode;
  if (current && nextTrack) {
    subtitle = (
      <>
        {t("nowPlaying.upNext")} <span className="text-zinc-300">{nextTrack.title}</span>
      </>
    );
  } else if (current && !nextTrack) {
    subtitle = t("nowPlaying.lastTrack");
  } else if (!current && nextTrack) {
    subtitle = t("nowPlaying.upFirst", { title: nextTrack.title });
  } else {
    subtitle = t("nowPlaying.addBelow");
  }

  return (
    <section className="rounded-2xl bg-zinc-900 p-6">
      <div className="flex flex-col gap-1.5">
        <span className="text-xs font-medium uppercase tracking-wider text-rose-400">
          {t("nowPlaying.label")}
        </span>
        <h2
          className={`truncate text-2xl font-semibold tracking-tight ${
            current ? "text-zinc-100" : "text-zinc-500"
          }`}
        >
          {current
            ? current.info.title
            : hasContent
              ? t("nowPlaying.pressPlay")
              : t("nowPlaying.emptyQueue")}
        </h2>
        <p className="text-sm text-zinc-500">{subtitle}</p>
      </div>

      <div className="mt-6 flex flex-col gap-2">
        <div className="relative h-2 w-full overflow-hidden rounded-full bg-zinc-800">
          {current && knowsDuration && (
            <div
              className="absolute inset-y-0 left-0 bg-gradient-to-r from-rose-500 to-rose-400 transition-[width] duration-300"
              style={{ width: `${progressPct}%` }}
            />
          )}
          {current && !knowsDuration && isPlaying && (
            <div className="aircast-indeterminate absolute inset-0" />
          )}
        </div>
        <div className="flex items-baseline justify-between font-mono text-sm tabular-nums">
          <span className={current ? "text-zinc-100" : "text-zinc-700"}>
            {current ? formatTime(elapsed) : "—:—"}
          </span>
          <span className="text-zinc-500">
            {current
              ? knowsDuration
                ? formatTime(duration!)
                : t("nowPlaying.unknownDuration")
              : "—:—"}
          </span>
        </div>
      </div>

      <div className="mt-5 flex items-center justify-between">
        <div className="text-sm text-zinc-500">
          {t(queueCountKey, { count: snapshot.queue.length })}
        </div>
        <div className="flex items-center gap-2">
          <SecondaryBtn
            onClick={onStop}
            disabled={!current}
            icon={<Square className="h-3.5 w-3.5" />}
          >
            {t("transport.stop")}
          </SecondaryBtn>
          <SecondaryBtn
            onClick={onNext}
            disabled={snapshot.queue.length === 0 && !current}
            icon={<SkipForward className="h-4 w-4" />}
          >
            {t("transport.next")}
          </SecondaryBtn>
          <PrimaryBtn
            onClick={isPlaying ? onPause : onPlay}
            disabled={!current && snapshot.queue.length === 0}
            icon={
              isPlaying ? (
                <Pause className="h-5 w-5 fill-current" />
              ) : (
                <Play className="h-5 w-5 fill-current" />
              )
            }
          >
            {isPlaying ? t("transport.pause") : t("transport.play")}
          </PrimaryBtn>
        </div>
      </div>
    </section>
  );
}

function PrimaryBtn({
  children,
  onClick,
  disabled,
  icon,
}: {
  children: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
  icon: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="flex items-center gap-2 rounded-full bg-rose-500 px-5 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-rose-400 disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-500"
    >
      {icon}
      <span>{children}</span>
    </button>
  );
}

function SecondaryBtn({
  children,
  onClick,
  disabled,
  icon,
}: {
  children: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
  icon: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="flex items-center gap-1.5 rounded-full bg-zinc-800 px-4 py-2.5 text-sm font-medium text-zinc-200 transition-colors hover:bg-zinc-700 disabled:cursor-not-allowed disabled:opacity-40"
    >
      {icon}
      <span>{children}</span>
    </button>
  );
}

function formatTime(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return "00:00";
  const total = Math.floor(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ChevronDown, ChevronUp, ListMusic, Plus, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { useT } from "@/i18n/context";
import type { MusicSnapshot } from "@/types";

interface Props {
  snapshot: MusicSnapshot;
  onChange: () => void;
}

export function MusicQueue({ snapshot, onChange }: Props) {
  const { t } = useT();
  async function handleAdd() {
    const selected = await openDialog({
      multiple: true,
      filters: [{ name: "Audio", extensions: ["mp3", "wav", "flac", "ogg", "m4a"] }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    if (paths.length === 0) return;
    await api.musicEnqueue(paths);
    onChange();
  }

  async function handleRemove(id: string) {
    await api.musicRemove(id);
    onChange();
  }

  async function handleMove(id: string, delta: number) {
    await api.musicMove(id, delta);
    onChange();
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col rounded-2xl bg-zinc-900">
      <header className="flex items-center justify-between px-6 py-4">
        <h3 className="flex items-center gap-2 text-sm font-medium text-zinc-200">
          <ListMusic className="h-4 w-4 text-zinc-500" />
          {t("queue.title")}
        </h3>
        <button
          type="button"
          onClick={handleAdd}
          className="flex items-center gap-1.5 rounded-full bg-rose-500 px-4 py-1.5 text-xs font-semibold text-white transition-colors hover:bg-rose-400"
        >
          <Plus className="h-3.5 w-3.5" />
          {t("queue.add")}
        </button>
      </header>

      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-3 pb-3">
        {snapshot.queue.length === 0 && !snapshot.current && <EmptyState onAdd={handleAdd} />}

        {snapshot.current && (
          <div className="mb-2 flex items-center gap-3 rounded-xl bg-rose-500/15 px-4 py-3">
            <span className="h-2 w-2 shrink-0 animate-pulse rounded-full bg-rose-400" />
            <span className="min-w-0 flex-1 truncate text-sm font-medium text-zinc-100">
              {snapshot.current.info.title}
            </span>
            <span className="shrink-0 text-xs text-rose-300">{t("queue.nowPlaying")}</span>
          </div>
        )}

        {snapshot.queue.map((track, idx) => (
          <div
            key={track.id}
            className="group flex items-center gap-3 rounded-xl px-4 py-2.5 hover:bg-zinc-800/60"
          >
            <span className="w-5 shrink-0 text-right text-xs font-medium text-zinc-600">
              {idx + 1}
            </span>
            <span className="min-w-0 flex-1 truncate text-sm text-zinc-200">{track.title}</span>
            {track.durationSecs !== null && (
              <span className="shrink-0 text-xs tabular-nums text-zinc-500">
                {formatDuration(track.durationSecs)}
              </span>
            )}
            <div className="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <IconBtn onClick={() => handleMove(track.id, -1)} title={t("queue.moveUp")} disabled={idx === 0}>
                <ChevronUp className="h-4 w-4" />
              </IconBtn>
              <IconBtn
                onClick={() => handleMove(track.id, 1)}
                title={t("queue.moveDown")}
                disabled={idx === snapshot.queue.length - 1}
              >
                <ChevronDown className="h-4 w-4" />
              </IconBtn>
              <IconBtn onClick={() => handleRemove(track.id)} title={t("queue.remove")} danger>
                <Trash2 className="h-4 w-4" />
              </IconBtn>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  const { t } = useT();
  return (
    <button
      type="button"
      onClick={onAdd}
      className="m-3 flex flex-1 flex-col items-center justify-center gap-3 rounded-2xl bg-zinc-800/40 px-6 py-12 text-center text-zinc-400 transition-colors hover:bg-zinc-800/80 hover:text-zinc-200"
    >
      <div className="rounded-full bg-zinc-800 p-3">
        <ListMusic className="h-6 w-6" />
      </div>
      <div>
        <div className="text-sm font-semibold text-zinc-200">{t("queue.empty")}</div>
        <div className="mt-1 text-xs text-zinc-500">{t("queue.emptyHint")}</div>
      </div>
    </button>
  );
}

function IconBtn({
  children,
  onClick,
  title,
  disabled,
  danger,
}: {
  children: React.ReactNode;
  onClick: () => void;
  title: string;
  disabled?: boolean;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      disabled={disabled}
      className={[
        "rounded-full p-1.5 transition-colors disabled:opacity-30",
        danger
          ? "text-zinc-500 hover:bg-rose-500 hover:text-white"
          : "text-zinc-400 hover:bg-zinc-700 hover:text-zinc-100",
      ].join(" ")}
    >
      {children}
    </button>
  );
}

function formatDuration(seconds: number): string {
  const total = Math.floor(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

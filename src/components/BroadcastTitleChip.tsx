import { Settings } from "lucide-react";
import { useT } from "@/i18n/context";

interface Props {
  /** Title currently broadcast. Empty = chip hidden. */
  title: string;
  /** Whether the stream is in the live state. Chip hidden otherwise. */
  live: boolean;
  onEdit: () => void;
}

/**
 * Compact inline variant of the broadcast title indicator. Designed to live
 * in a card header (Now Playing top-right) — single-line, truncating, with
 * a pulse dot to make it obvious that listeners are seeing this *right now*.
 *
 * Hidden when the stream isn't live or when no title is being pushed.
 */
export function BroadcastTitleChip({ title, live, onEdit }: Props) {
  const { t } = useT();
  if (!live || !title.trim()) return null;

  // The native `title` attribute on the button surfaces the full text on
  // hover, even when truncated — no separate tooltip widget needed.
  return (
    <button
      type="button"
      onClick={onEdit}
      title={`${title}\n\n${t("broadcast.editHint")}`}
      className="group flex min-w-0 max-w-md items-center gap-2.5 rounded-full bg-zinc-800/80 px-3 py-1.5 ring-1 ring-zinc-700/50 transition-all hover:bg-zinc-700 hover:ring-zinc-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-500/50"
    >
      <span className="relative flex h-2 w-2 shrink-0">
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-rose-500 opacity-60" />
        <span className="relative inline-flex h-2 w-2 rounded-full bg-rose-500" />
      </span>
      <span className="shrink-0 text-[10px] font-semibold uppercase tracking-wider text-zinc-400">
        {t("broadcast.labelShort")}
      </span>
      <span className="min-w-0 flex-1 truncate text-left text-xs font-medium text-zinc-100">
        {title}
      </span>
      <Settings className="h-3 w-3 shrink-0 text-zinc-500 transition-colors group-hover:text-zinc-300" />
    </button>
  );
}

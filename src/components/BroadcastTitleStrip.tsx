import { Radio, Settings } from "lucide-react";
import { useT } from "@/i18n/context";

interface Props {
  /** Title currently broadcast to Icecast. Empty = broadcaster dormant. */
  title: string;
  /** Whether the stream is in the live state. We only show the strip when both
   *  conditions hold (live AND non-empty title) so an idle app stays clean. */
  live: boolean;
  /** Click handler that opens Setup directly at the metadata section. */
  onEdit: () => void;
}

/**
 * Slim row pinned just above the status bar, visible only while a title is
 * actively broadcasting. Single line, truncates long titles, click-to-edit
 * on the trailing icon (or anywhere on the strip — the whole row is the
 * affordance).
 */
export function BroadcastTitleStrip({ title, live, onEdit }: Props) {
  const { t } = useT();
  if (!live || !title.trim()) return null;

  return (
    <button
      type="button"
      onClick={onEdit}
      title={`${title}\n\n${t("broadcast.editHint")}`}
      className="group flex w-full shrink-0 items-center gap-3 border-t border-zinc-900 bg-zinc-900/40 px-5 py-2.5 text-left text-xs transition-colors hover:bg-zinc-900/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-500/40"
    >
      {/* Live pulse + label */}
      <span className="flex shrink-0 items-center gap-2">
        <span className="relative flex h-2 w-2">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-rose-500 opacity-60" />
          <span className="relative inline-flex h-2 w-2 rounded-full bg-rose-500" />
        </span>
        <Radio className="h-3.5 w-3.5 text-zinc-500" />
        <span className="font-medium uppercase tracking-wider text-zinc-500">
          {t("broadcast.label")}
        </span>
      </span>

      {/* Title — flex-1 so it absorbs available space and truncates cleanly */}
      <span className="flex-1 truncate text-zinc-100">{title}</span>

      {/* Edit affordance — visible always, brightens on hover */}
      <span className="flex shrink-0 items-center gap-1.5 rounded-full bg-zinc-800 px-2.5 py-1 text-zinc-400 transition-colors group-hover:bg-zinc-700 group-hover:text-zinc-200">
        <Settings className="h-3 w-3" />
        <span className="text-[11px] font-medium">{t("broadcast.edit")}</span>
      </span>
    </button>
  );
}

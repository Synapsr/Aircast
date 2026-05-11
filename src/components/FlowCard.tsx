import { ChevronDown } from "lucide-react";

interface Props {
  /** Tiny uppercase caption above the value (e.g. "Source", "Server"). */
  label: string;
  /** Lucide icon shown at the left of the value row. */
  icon: React.ReactNode;
  /** Bold line — typically the name of the active selection. */
  primary: React.ReactNode;
  /** Optional sub-line — e.g. "MP3 · 128 kbps" or full URL. Truncated. */
  secondary?: React.ReactNode;
  /** Pass a click handler to make the whole card open a picker. The chevron
   *  appears automatically. Omit for a passive display. */
  onClick?: () => void;
  /** When true the trailing chevron is drawn rotated (picker open). */
  isOpen?: boolean;
  /** Accent: rose when calling to action (e.g. "Pick a server"), zinc otherwise. */
  intent?: "default" | "accent";
}

/**
 * One row in the source→destination flow. Used identically in Simple and
 * Relay modes so the user learns the affordance once. Click the card to
 * open the matching picker (mic devices, relay URLs, server presets).
 */
export function FlowCard({
  label,
  icon,
  primary,
  secondary,
  onClick,
  isOpen,
  intent = "default",
}: Props) {
  const Tag: "button" | "div" = onClick ? "button" : "div";
  return (
    <Tag
      type={onClick ? "button" : undefined}
      onClick={onClick}
      className={[
        "flex w-full items-center gap-3 rounded-xl px-4 py-3 text-left transition-colors",
        intent === "accent"
          ? "bg-rose-500/15 ring-1 ring-rose-500/30 text-rose-100 hover:bg-rose-500/20"
          : "bg-zinc-900 ring-1 ring-zinc-800 hover:bg-zinc-800/80",
        onClick ? "cursor-pointer" : "cursor-default",
      ].join(" ")}
    >
      <span
        className={
          intent === "accent"
            ? "shrink-0 text-rose-300"
            : "shrink-0 text-rose-400"
        }
      >
        {icon}
      </span>
      <div className="flex min-w-0 flex-1 flex-col items-start">
        <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
          {label}
        </span>
        <span className="w-full truncate text-sm text-zinc-100">
          {primary}
        </span>
        {secondary && (
          <span className="w-full truncate text-[11px] text-zinc-500">
            {secondary}
          </span>
        )}
      </div>
      {onClick && (
        <ChevronDown
          className={`h-4 w-4 shrink-0 text-zinc-500 transition-transform ${
            isOpen ? "rotate-180" : ""
          }`}
        />
      )}
    </Tag>
  );
}

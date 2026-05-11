import { ArrowDown } from "lucide-react";

/**
 * Visual link between the source FlowCard and the destination FlowCard.
 * A small rose-tinted arrow vertically centred between them, with thin
 * lines extending above/below to subtly hint at a wire/cable between cards.
 */
export function FlowArrow() {
  return (
    <div className="flex flex-col items-center py-1.5">
      <span aria-hidden className="h-4 w-px bg-zinc-800" />
      <span
        aria-hidden
        className="my-1 flex h-6 w-6 items-center justify-center rounded-full bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30"
      >
        <ArrowDown className="h-3.5 w-3.5" />
      </span>
      <span aria-hidden className="h-4 w-px bg-zinc-800" />
    </div>
  );
}

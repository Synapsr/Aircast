import { useEffect, useRef } from "react";
import { AlertTriangle, X } from "lucide-react";
import { useT } from "@/i18n/context";
import type { AppMode } from "@/types";

interface Props {
  /** Current mode (the one we're leaving). Drives which side effects we list. */
  source: AppMode;
  /** Target mode the user is trying to switch to. `null` = modal closed. */
  target: AppMode | null;
  /** Music currently playing in the studio queue. Drives one of the warnings. */
  musicPlaying: boolean;
  /** Whether the mic is open right now (in studio mode it can be closed). */
  micOpen: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

/**
 * Confirmation dialog shown when the user tries to switch mode while a stream
 * is live. Mode transitions have side effects on the on-air content
 * (music stops, mic opens/closes), so we make these explicit before the
 * change goes through.
 */
export function ConfirmModeSwitchModal({
  source,
  target,
  musicPlaying,
  micOpen,
  onCancel,
  onConfirm,
}: Props) {
  const { t } = useT();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const d = dialogRef.current;
    if (!d) return;
    if (target && !d.open) {
      d.showModal();
      // Cancel is the safe default — focus it so Enter doesn't accidentally
      // confirm a destructive action.
      requestAnimationFrame(() => cancelButtonRef.current?.focus());
    } else if (!target && d.open) {
      d.close();
    }
  }, [target]);

  function handleBackdropClick(e: React.MouseEvent<HTMLDialogElement>) {
    if (e.target === dialogRef.current) onCancel();
  }

  // Compute the consequences list per direction. Each bullet is something the
  // user will actually hear (or stop hearing) on the antenna after confirm.
  const consequences: string[] = (() => {
    if (!target) return [];
    const items: string[] = [];

    // Source-side side effects: what stops broadcasting.
    if (source === "studio" && musicPlaying) {
      items.push(t("modeSwitch.effect.musicStops"));
    }
    if (source === "relay") {
      items.push(t("modeSwitch.effect.relayStops"));
    }

    // Target-side side effects: what becomes the audio source.
    if (target === "simple") {
      // Simple = mic-passthrough, always on-air.
      if (!micOpen || source === "relay") {
        items.push(t("modeSwitch.effect.micOpens"));
      }
    } else if (target === "studio") {
      // Studio always opens with the mic gated closed.
      items.push(t("modeSwitch.effect.micCloses"));
      // If no music is queued, the antenna goes silent until the user does
      // something (presses play, opens mic).
      if (!musicPlaying) {
        items.push(t("modeSwitch.effect.silence"));
      }
    } else if (target === "relay") {
      // Relay replaces whatever was being broadcast with the upstream URL.
      if (source !== "relay") {
        items.push(t("modeSwitch.effect.upstreamStarts"));
      }
    }

    // Failsafe: always show at least one bullet so the user knows something
    // will change after they confirm.
    if (items.length === 0) {
      items.push(t("modeSwitch.effect.generic"));
    }
    return items;
  })();

  // Lead-in line — which mode we're heading to.
  const leadKey = target ? `modeSwitch.lead.${target}` : "modeSwitch.lead.simple";

  return (
    <dialog
      ref={dialogRef}
      onClose={onCancel}
      onClick={handleBackdropClick}
      className="fixed inset-0 m-auto w-full max-w-md rounded-2xl bg-zinc-900 p-0 text-zinc-100 shadow-2xl backdrop:bg-black/70 backdrop:backdrop-blur-sm open:flex open:flex-col"
    >
      <header className="flex items-start justify-between gap-3 px-6 pb-3 pt-6">
        <div className="flex items-start gap-3">
          <div className="rounded-xl bg-amber-500/15 p-2 text-amber-300 ring-1 ring-amber-500/30">
            <AlertTriangle className="h-5 w-5" />
          </div>
          <div>
            <h2 className="text-base font-semibold tracking-tight">
              {t("modeSwitch.title")}
            </h2>
            <p className="mt-0.5 text-xs text-zinc-500">{t("modeSwitch.subtitle")}</p>
          </div>
        </div>
        <button
          type="button"
          onClick={onCancel}
          aria-label={t("common.dismiss")}
          className="rounded-full p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
        >
          <X className="h-4 w-4" />
        </button>
      </header>

      <div className="flex flex-col gap-3 px-6 pb-5">
        <p className="text-sm text-zinc-300">{t(leadKey)}</p>
        <ul className="flex flex-col gap-1.5 rounded-lg bg-zinc-800/60 p-3.5 text-sm">
          {consequences.map((c, i) => (
            <li key={i} className="flex items-start gap-2 text-zinc-200">
              <span aria-hidden className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-amber-400" />
              <span>{c}</span>
            </li>
          ))}
        </ul>
      </div>

      <footer className="flex items-center justify-end gap-2 border-t border-zinc-800 px-6 py-4">
        <button
          ref={cancelButtonRef}
          type="button"
          onClick={onCancel}
          className="rounded-lg bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 transition-colors hover:bg-zinc-700"
        >
          {t("modeSwitch.cancel")}
        </button>
        <button
          type="button"
          onClick={onConfirm}
          className="rounded-lg bg-amber-500 px-4 py-2 text-sm font-semibold text-zinc-950 shadow-md shadow-amber-500/20 transition-colors hover:bg-amber-400"
        >
          {t("modeSwitch.confirm")}
        </button>
      </footer>
    </dialog>
  );
}

import { useEffect, useRef } from "react";
import { ExternalLink, X } from "lucide-react";
import { api } from "@/lib/api";
import { useT } from "@/i18n/context";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function AboutModal({ open, onClose }: Props) {
  const { t } = useT();
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const d = dialogRef.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    else if (!open && d.open) d.close();
  }, [open]);

  function handleBackdropClick(e: React.MouseEvent<HTMLDialogElement>) {
    if (e.target === dialogRef.current) onClose();
  }

  function openSynapsr() {
    api.openExternal("https://synapsr.io").catch(() => {});
  }

  return (
    <dialog
      ref={dialogRef}
      onClose={onClose}
      onClick={handleBackdropClick}
      className="fixed inset-0 m-auto w-full max-w-md rounded-2xl bg-zinc-900 p-0 text-zinc-100 shadow-2xl backdrop:bg-black/70 backdrop:backdrop-blur-sm open:flex open:flex-col"
    >
      <div className="flex flex-col items-center gap-5 px-7 pb-7 pt-8 text-center">
        <button
          type="button"
          onClick={onClose}
          aria-label={t("common.dismiss")}
          className="absolute right-4 top-4 rounded-full p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
        >
          <X className="h-4 w-4" />
        </button>

        <img
          src="/icon.png"
          alt="Aircast"
          className="h-20 w-20 rounded-2xl shadow-md shadow-rose-500/20"
        />

        <div className="flex flex-col items-center gap-1">
          <h2 className="text-xl font-semibold tracking-tight">Aircast</h2>
          <p className="text-xs text-zinc-500">
            {t("about.tagline")}
          </p>
        </div>

        <p className="max-w-sm text-sm leading-relaxed text-zinc-400">
          {t("about.body")}
        </p>

        <div className="my-1 h-px w-full bg-zinc-800" />

        <button
          type="button"
          onClick={openSynapsr}
          className="group flex items-center gap-2 rounded-full bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 transition-colors hover:bg-zinc-700"
        >
          <span>{t("about.visitSynapsr")}</span>
          <ExternalLink className="h-3.5 w-3.5 text-zinc-400 transition-transform group-hover:translate-x-0.5" />
        </button>

        <p className="text-[11px] text-zinc-600">
          {t("about.license")}
        </p>
      </div>
    </dialog>
  );
}

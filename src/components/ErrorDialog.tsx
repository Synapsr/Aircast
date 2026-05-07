import { useEffect, useRef, useState } from "react";
import { AlertTriangle, ChevronDown, Copy, Settings as SettingsIcon, X } from "lucide-react";
import { useT } from "@/i18n/context";

interface Props {
  open: boolean;
  message: string;
  details?: string | null;
  onClose: () => void;
  onOpenSetup?: () => void;
}

export function ErrorDialog({ open, message, details, onClose, onOpenSetup }: Props) {
  const { t } = useT();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [showDetails, setShowDetails] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const d = dialogRef.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    else if (!open && d.open) d.close();
    if (open) setShowDetails(false);
  }, [open]);

  function handleBackdropClick(e: React.MouseEvent<HTMLDialogElement>) {
    if (e.target === dialogRef.current) onClose();
  }

  async function copyDetails() {
    if (!details) return;
    try {
      await navigator.clipboard.writeText(`${message}\n\n${details}`);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore — some environments restrict clipboard
    }
  }

  return (
    <dialog
      ref={dialogRef}
      onClose={onClose}
      onClick={handleBackdropClick}
      className="fixed inset-0 m-auto w-full max-w-lg rounded-2xl bg-zinc-900 p-0 text-zinc-100 shadow-2xl backdrop:bg-black/70 backdrop:backdrop-blur-sm open:flex open:flex-col"
    >
      <header className="flex items-start justify-between gap-3 px-6 py-5">
        <div className="flex items-start gap-3">
          <div className="mt-0.5 shrink-0 rounded-xl bg-rose-500/15 p-2 text-rose-300 ring-1 ring-rose-500/30">
            <AlertTriangle className="h-5 w-5" />
          </div>
          <div>
            <h2 className="text-base font-semibold tracking-tight">
              {t("errors.dialogTitle")}
            </h2>
            <p className="mt-1 text-sm leading-relaxed text-zinc-300">{message}</p>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="shrink-0 rounded-full p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
        >
          <X className="h-4 w-4" />
        </button>
      </header>

      {details && (
        <div className="px-6 pb-2">
          <button
            type="button"
            onClick={() => setShowDetails((v) => !v)}
            className="flex w-full items-center justify-between rounded-lg bg-zinc-800/50 px-3 py-2 text-xs font-medium text-zinc-300 hover:bg-zinc-800"
          >
            <span className="flex items-center gap-2">
              {showDetails ? t("errors.hideDetails") : t("errors.showDetails")}
            </span>
            <ChevronDown
              className={`h-4 w-4 transition-transform ${showDetails ? "rotate-180" : ""}`}
            />
          </button>
          {showDetails && (
            <div className="mt-2 flex flex-col gap-2">
              <div className="max-h-48 overflow-y-auto rounded-lg bg-zinc-950 p-3">
                <pre className="whitespace-pre-wrap break-all font-mono text-[11px] leading-relaxed text-zinc-400">
                  {details}
                </pre>
              </div>
              <button
                type="button"
                onClick={copyDetails}
                className="flex items-center gap-1.5 self-start rounded-md bg-zinc-800 px-3 py-1.5 text-xs font-medium text-zinc-200 hover:bg-zinc-700"
              >
                <Copy className="h-3.5 w-3.5" />
                {copied ? t("errors.copied") : t("errors.copy")}
              </button>
            </div>
          )}
        </div>
      )}

      <footer className="mt-2 flex items-center justify-end gap-2 px-6 py-4">
        {onOpenSetup && (
          <button
            type="button"
            onClick={onOpenSetup}
            className="flex items-center gap-1.5 rounded-lg bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-700"
          >
            <SettingsIcon className="h-3.5 w-3.5" />
            {t("errors.openSetup")}
          </button>
        )}
        <button
          type="button"
          onClick={onClose}
          className="rounded-lg bg-rose-500 px-5 py-2 text-sm font-semibold text-white hover:bg-rose-400"
        >
          {t("errors.ok")}
        </button>
      </footer>
    </dialog>
  );
}

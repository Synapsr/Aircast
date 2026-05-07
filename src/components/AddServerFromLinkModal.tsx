import { useEffect, useRef } from "react";
import { Link2, X } from "lucide-react";
import { useT } from "@/i18n/context";
import type { ParsedServerLink } from "@/lib/deeplink";

interface Props {
  parsed: ParsedServerLink | null;
  finalName: string | null;
  onCancel: () => void;
  onConfirm: () => Promise<void> | void;
}

export function AddServerFromLinkModal({ parsed, finalName, onCancel, onConfirm }: Props) {
  const { t } = useT();
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const d = dialogRef.current;
    if (!d) return;
    if (parsed && !d.open) d.showModal();
    else if (!parsed && d.open) d.close();
  }, [parsed]);

  function handleBackdropClick(e: React.MouseEvent<HTMLDialogElement>) {
    if (e.target === dialogRef.current) onCancel();
  }

  return (
    <dialog
      ref={dialogRef}
      onClose={onCancel}
      onClick={handleBackdropClick}
      className="fixed inset-0 m-auto w-full max-w-md rounded-2xl bg-zinc-900 p-0 text-zinc-100 shadow-2xl backdrop:bg-black/70 backdrop:backdrop-blur-sm open:flex open:flex-col"
    >
      {parsed && (
        <>
          <header className="flex items-center justify-between px-6 py-5">
            <div className="flex items-center gap-3">
              <div className="rounded-xl bg-rose-500/15 p-2 text-rose-300 ring-1 ring-rose-500/30">
                <Link2 className="h-5 w-5" />
              </div>
              <div>
                <h2 className="text-base font-semibold tracking-tight">
                  {t("link.title")}
                </h2>
                <p className="text-xs text-zinc-500">{t("link.subtitle")}</p>
              </div>
            </div>
            <button
              type="button"
              onClick={onCancel}
              className="rounded-full p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100"
            >
              <X className="h-4 w-4" />
            </button>
          </header>

          <div className="flex flex-col gap-4 px-6 pb-2 text-sm">
            <p className="text-zinc-300">{t("link.questionMark")}</p>

            <dl className="flex flex-col gap-2.5 rounded-xl bg-zinc-800/50 px-4 py-3.5">
              <Row label={t("link.name")} value={parsed.name} />
              <Row
                label={t("link.server")}
                value={`${parsed.config.host}:${parsed.config.port}${parsed.config.mount}`}
                mono
              />
              <Row
                label={t("link.credentials")}
                value={
                  parsed.config.password
                    ? parsed.config.username
                    : `${parsed.config.username} ${t("link.anonymous")}`
                }
              />
              <Row
                label={t("link.encoding")}
                value={`${parsed.config.format.toUpperCase()} · ${parsed.config.bitrate} kbps`}
              />
            </dl>

            {finalName && finalName !== parsed.name && (
              <p className="text-xs text-amber-300">
                {t("link.renamedHint", { name: finalName })}
              </p>
            )}
          </div>

          <footer className="mt-2 flex items-center justify-end gap-2 px-6 py-4">
            <button
              type="button"
              onClick={onCancel}
              className="rounded-lg bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-700"
            >
              {t("common.cancel")}
            </button>
            <button
              type="button"
              onClick={() => void onConfirm()}
              className="rounded-lg bg-rose-500 px-5 py-2 text-sm font-semibold text-white hover:bg-rose-400"
            >
              {t("link.add")}
            </button>
          </footer>
        </>
      )}
    </dialog>
  );
}

function Row({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-baseline gap-3">
      <dt className="w-28 shrink-0 text-xs uppercase tracking-wider text-zinc-500">{label}</dt>
      <dd className={`min-w-0 flex-1 truncate ${mono ? "font-mono text-zinc-100" : "text-zinc-100"}`}>
        {value}
      </dd>
    </div>
  );
}

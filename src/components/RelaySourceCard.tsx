import { useEffect, useRef, useState } from "react";
import { Cable, Settings as SettingsIcon } from "lucide-react";
import { FlowCard } from "@/components/FlowCard";
import { useT } from "@/i18n/context";
import type { RelaySource } from "@/types";

interface Props {
  sources: RelaySource[];
  activeName: string | null;
  onPick: (name: string) => void;
  onManage: () => void;
}

/**
 * Source card for Relay mode: shows the current upstream URL and lets the
 * user switch between saved relay sources. Mirrors `MicSourceCard` and
 * `ServerDestinationCard` for affordance consistency.
 */
export function RelaySourceCard({ sources, activeName, onPick, onManage }: Props) {
  const { t } = useT();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    function handleMouseDown(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", handleMouseDown);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleMouseDown);
      document.removeEventListener("keydown", handleKey);
    };
  }, [open]);

  const active = sources.find((s) => s.name === activeName) ?? null;

  if (sources.length === 0) {
    return (
      <FlowCard
        label={t("flow.relayLabel")}
        icon={<Cable className="h-4 w-4" />}
        primary={t("relay.addFirstSource")}
        onClick={onManage}
        intent="accent"
      />
    );
  }

  return (
    <div ref={containerRef} className="relative w-full">
      <FlowCard
        label={t("flow.relayLabel")}
        icon={<Cable className="h-4 w-4" />}
        primary={active?.name ?? t("relay.pickSource")}
        secondary={active?.url}
        onClick={() => setOpen((o) => !o)}
        isOpen={open}
        intent={active ? "default" : "accent"}
      />

      {open && (
        <div className="absolute left-0 right-0 top-full z-50 mt-2 overflow-hidden rounded-xl bg-zinc-900 shadow-2xl ring-1 ring-zinc-800">
          <div className="flex items-center justify-between border-b border-zinc-800 px-3 py-2">
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
              {t("relay.sources")}
            </span>
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                onManage();
              }}
              className="flex items-center gap-1 rounded-full bg-zinc-800 px-2 py-1 text-[11px] font-medium text-zinc-200 hover:bg-zinc-700"
            >
              <SettingsIcon className="h-3 w-3" />
              {t("flow.manage")}
            </button>
          </div>
          <div className="max-h-72 overflow-y-auto p-1">
            {sources.map((s) => (
              <button
                key={s.name}
                type="button"
                onClick={() => {
                  onPick(s.name);
                  setOpen(false);
                }}
                className={[
                  "flex w-full flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left transition-colors",
                  s.name === activeName
                    ? "bg-zinc-800 text-zinc-100"
                    : "text-zinc-300 hover:bg-zinc-800/60",
                ].join(" ")}
              >
                <span className="flex w-full items-center gap-2">
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                      s.name === activeName ? "bg-rose-500" : "bg-zinc-700"
                    }`}
                  />
                  <span className="truncate text-sm">{s.name}</span>
                </span>
                <span className="ml-3.5 truncate text-[11px] text-zinc-500">{s.url}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

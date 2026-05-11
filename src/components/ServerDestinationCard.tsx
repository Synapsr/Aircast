import { useEffect, useRef, useState } from "react";
import { Globe, Settings as SettingsIcon } from "lucide-react";
import { FlowCard } from "@/components/FlowCard";
import { useT } from "@/i18n/context";
import type { Preset } from "@/types";

interface Props {
  presets: Preset[];
  activeName: string | null;
  /** Switch to another saved preset. Receives the preset name. */
  onSelect: (name: string) => void;
  /** Open the Settings modal on the Servers tab — used both for the
   *  "Manage" button in the dropdown and for the CTA shown when no
   *  server has been configured yet. */
  onManage: () => void;
}

/**
 * Destination card used by Simple and Relay modes: shows the active server
 * preset (host:port + codec) and lets the user switch between saved presets
 * via a dropdown. Mirrors the relay-source picker UX so the user learns one
 * affordance and reuses it for the other end of the flow.
 */
export function ServerDestinationCard({
  presets,
  activeName,
  onSelect,
  onManage,
}: Props) {
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

  const active = presets.find((p) => p.name === activeName) ?? null;

  // Empty state: no server configured → CTA card that opens Setup directly.
  if (presets.length === 0) {
    return (
      <FlowCard
        label={t("flow.serverLabel")}
        icon={<Globe className="h-4 w-4" />}
        primary={t("flow.serverPickCTA")}
        onClick={onManage}
        intent="accent"
      />
    );
  }

  return (
    <div ref={containerRef} className="relative w-full">
      <FlowCard
        label={t("flow.serverLabel")}
        icon={<Globe className="h-4 w-4" />}
        primary={active?.name ?? t("flow.serverPickPrompt")}
        secondary={
          active
            ? formatServer(active)
            : undefined
        }
        onClick={() => setOpen((o) => !o)}
        isOpen={open}
      />

      {open && (
        <div className="absolute left-0 right-0 top-full z-50 mt-2 overflow-hidden rounded-xl bg-zinc-900 shadow-2xl ring-1 ring-zinc-800">
          <div className="flex items-center justify-between border-b border-zinc-800 px-3 py-2">
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
              {t("flow.servers")}
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
            {presets.map((p) => (
              <button
                key={p.name}
                type="button"
                onClick={() => {
                  onSelect(p.name);
                  setOpen(false);
                }}
                className={[
                  "flex w-full flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left transition-colors",
                  p.name === activeName
                    ? "bg-zinc-800 text-zinc-100"
                    : "text-zinc-300 hover:bg-zinc-800/60",
                ].join(" ")}
              >
                <span className="flex w-full items-center gap-2">
                  <span
                    className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                      p.name === activeName ? "bg-rose-500" : "bg-zinc-700"
                    }`}
                  />
                  <span className="truncate text-sm">{p.name}</span>
                </span>
                <span className="ml-3.5 truncate text-[11px] text-zinc-500">
                  {formatServer(p)}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function formatServer(p: Preset): string {
  const mount = p.config.mount.startsWith("/")
    ? p.config.mount
    : `/${p.config.mount}`;
  return `${p.config.host}:${p.config.port}${mount} · ${p.config.format.toUpperCase()} ${p.config.bitrate} kbps`;
}

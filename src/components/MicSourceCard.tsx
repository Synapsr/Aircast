import { useEffect, useRef, useState } from "react";
import { Mic, RefreshCw } from "lucide-react";
import { FlowCard } from "@/components/FlowCard";
import { useT } from "@/i18n/context";
import { api } from "@/lib/api";
import type { AudioDevice } from "@/types";

interface Props {
  value: string | null;
  onChange: (id: string) => void;
}

/**
 * Source card for Simple mode: shows the current microphone and lets the
 * user switch between input devices. Same dropdown UX as
 * `ServerDestinationCard` so the source→destination flow feels consistent.
 */
export function MicSourceCard({ value, onChange }: Props) {
  const { t } = useT();
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);

  async function refresh() {
    setLoading(true);
    try {
      const list = await api.listAudioDevices();
      setDevices(list);
      if (!value && list.length > 0) {
        const def = list.find((d) => d.isDefault) ?? list[0];
        onChange(def.id);
      }
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  const current = devices.find((d) => d.id === value) ?? null;
  const hasDevice = !!current;

  if (devices.length === 0 && !loading) {
    // No mic detected — refresh CTA so the user knows the panel is reactive.
    return (
      <FlowCard
        label={t("flow.micLabel")}
        icon={<Mic className="h-4 w-4" />}
        primary={t("device.empty")}
        onClick={() => void refresh()}
        intent="accent"
      />
    );
  }

  return (
    <div ref={containerRef} className="relative w-full">
      <FlowCard
        label={t("flow.micLabel")}
        icon={<Mic className="h-4 w-4" />}
        primary={current?.name ?? t("device.pick")}
        onClick={() => setOpen((o) => !o)}
        isOpen={open}
        intent={hasDevice ? "default" : "accent"}
      />

      {open && (
        <div className="absolute left-0 right-0 top-full z-50 mt-2 overflow-hidden rounded-xl bg-zinc-900 shadow-2xl ring-1 ring-zinc-800">
          <div className="flex items-center justify-between border-b border-zinc-800 px-3 py-2">
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
              {t("device.label")}
            </span>
            <button
              type="button"
              onClick={() => void refresh()}
              disabled={loading}
              className="rounded-full p-1 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 disabled:opacity-50"
              title={t("device.refresh")}
            >
              <RefreshCw className={`h-3 w-3 ${loading ? "animate-spin" : ""}`} />
            </button>
          </div>
          <div className="max-h-72 overflow-y-auto p-1">
            {devices.map((d) => (
              <button
                key={d.id}
                type="button"
                onClick={() => {
                  onChange(d.id);
                  setOpen(false);
                }}
                className={[
                  "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors",
                  value === d.id
                    ? "bg-zinc-800 text-zinc-100"
                    : "text-zinc-300 hover:bg-zinc-800/60",
                ].join(" ")}
              >
                <span
                  className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                    value === d.id ? "bg-rose-500" : "bg-zinc-700"
                  }`}
                />
                <span className="truncate">
                  {d.name}
                  {d.isDefault && (
                    <span className="text-zinc-500">{t("device.default")}</span>
                  )}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

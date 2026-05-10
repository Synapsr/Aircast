import { useEffect, useRef, useState } from "react";
import { ChevronDown, Mic, RefreshCw } from "lucide-react";
import { api } from "@/lib/api";
import { useT } from "@/i18n/context";
import type { AudioDevice } from "@/types";

interface Props {
  value: string | null;
  onChange: (id: string) => void;
  /**
   * Visual style:
   * - `"pill"` (default): compact rounded pill, designed for the header bar.
   * - `"row"`: full-width row with larger padding and a chevron pinned to the
   *   right. Designed to live inside a card alongside the mic toggle.
   */
  variant?: "pill" | "row";
}

export function DevicePill({ value, onChange, variant = "pill" }: Props) {
  const { t } = useT();
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

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

  const current = devices.find((d) => d.id === value);
  const hasDevice = !!current;

  const triggerClass =
    variant === "row"
      ? [
          "flex w-full items-center gap-3 rounded-xl px-3.5 py-2.5 text-sm font-medium transition-colors",
          hasDevice
            ? "bg-zinc-800/80 text-zinc-100 ring-1 ring-zinc-700/60 hover:bg-zinc-800 hover:ring-zinc-600"
            : "bg-rose-500 text-white hover:bg-rose-400",
        ].join(" ")
      : [
          "flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-semibold transition-colors",
          hasDevice
            ? "bg-zinc-800 text-zinc-200 hover:bg-zinc-700"
            : "bg-rose-500 text-white hover:bg-rose-400",
        ].join(" ");

  const containerClass = variant === "row" ? "relative w-full" : "relative";
  const dropdownAlignment =
    variant === "row" ? "left-0 right-0 w-full" : "right-0 w-72";

  return (
    <div ref={containerRef} className={containerClass}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        title={hasDevice ? t("device.change") : t("device.pick")}
        className={triggerClass}
      >
        <Mic className={variant === "row" ? "h-4 w-4 shrink-0" : "h-3.5 w-3.5"} />
        <span
          className={
            variant === "row"
              ? "min-w-0 flex-1 truncate text-left"
              : "max-w-[180px] truncate"
          }
        >
          {current?.name ?? t("device.pick")}
        </span>
        <ChevronDown
          className={[
            "shrink-0 transition-transform",
            variant === "row" ? "h-4 w-4 text-zinc-500" : "h-3 w-3",
            open ? "rotate-180" : "",
          ].join(" ")}
        />
      </button>

      {open && (
        <div
          className={`absolute top-full z-50 mt-2 overflow-hidden rounded-xl bg-zinc-900 shadow-2xl ring-1 ring-zinc-800 ${dropdownAlignment}`}
        >
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
            {devices.length === 0 ? (
              <div className="px-3 py-6 text-center text-xs text-zinc-500">
                {t("device.empty")}
              </div>
            ) : (
              devices.map((d) => (
                <button
                  key={d.id}
                  type="button"
                  onClick={() => {
                    onChange(d.id);
                    setOpen(false);
                  }}
                  className={[
                    "flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left text-sm transition-colors",
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
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

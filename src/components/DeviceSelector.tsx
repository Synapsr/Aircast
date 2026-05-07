import { useEffect, useState } from "react";
import { Mic, RefreshCw } from "lucide-react";
import { api } from "@/lib/api";
import { useT } from "@/i18n/context";
import type { AudioDevice } from "@/types";

interface Props {
  value: string | null;
  onChange: (id: string) => void;
}

export function DeviceSelector({ value, onChange }: Props) {
  const { t } = useT();
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const list = await api.listAudioDevices();
      setDevices(list);
      if (!value && list.length > 0) {
        const def = list.find((d) => d.isDefault) ?? list[0];
        onChange(def.id);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex flex-col gap-2">
      <label className="flex items-center gap-2 text-sm font-medium text-zinc-300">
        <Mic className="h-4 w-4 text-zinc-500" />
        {t("device.label")}
      </label>
      <div className="flex items-center gap-2">
        <select
          value={value ?? ""}
          onChange={(e) => onChange(e.target.value)}
          disabled={devices.length === 0}
          className="flex-1 cursor-pointer rounded-lg bg-zinc-800 px-3.5 py-2.5 text-sm text-zinc-100 outline-none transition-colors hover:bg-zinc-700 focus:bg-zinc-700 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {devices.length === 0 && <option value="">{t("device.empty")}</option>}
          {devices.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name}
              {d.isDefault ? t("device.default") : ""}
            </option>
          ))}
        </select>
        <button
          type="button"
          onClick={refresh}
          disabled={loading}
          title={t("device.refresh")}
          className="rounded-lg bg-zinc-800 p-2.5 text-zinc-400 transition-colors hover:bg-zinc-700 hover:text-zinc-100 disabled:opacity-50"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </button>
      </div>
      {error && <p className="text-xs text-rose-400">{error}</p>}
    </div>
  );
}

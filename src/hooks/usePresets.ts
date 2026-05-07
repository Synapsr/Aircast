import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { Preset, StreamConfig } from "@/types";

export function usePresets() {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.loadPresets();
      setPresets(list);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const save = useCallback(
    async (name: string, config: StreamConfig) => {
      await api.savePreset(name, config);
      await refresh();
    },
    [refresh],
  );

  const remove = useCallback(
    async (name: string) => {
      await api.deletePreset(name);
      await refresh();
    },
    [refresh],
  );

  return { presets, loading, refresh, save, remove };
}

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { DEFAULT_CONFIG, type StreamConfig } from "@/types";

export function useCurrentConfig(deviceId: string | null) {
  const [config, setConfigState] = useState<StreamConfig | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    api.loadCurrentConfig().then((c) => {
      if (cancelled) return;
      setConfigState(c);
      setLoaded(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // Once a device is known, ensure the config has a deviceId.
  useEffect(() => {
    if (!loaded || !deviceId) return;
    if (!config) {
      setConfigState({ ...DEFAULT_CONFIG, deviceId });
    } else if (config.deviceId !== deviceId) {
      setConfigState({ ...config, deviceId });
    }
  }, [loaded, deviceId, config]);

  const update = useCallback(async (next: StreamConfig) => {
    setConfigState(next);
    await api.saveCurrentConfig(next);
  }, []);

  return { config, update, loaded };
}

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { Settings } from "@/types";

const DEFAULT_SETTINGS: Settings = {
  reconnectIntervalSeconds: 5,
  language: "auto",
  activePreset: null,
  musicVolumeWhenMicOpen: 0.3,
  crossfadeSeconds: 3,
  metadata: {
    enabled: true,
    mode: "auto",
    template: "{artist} — {title}",
    staticText: "",
    filePath: null,
    filePollSecs: 5,
    micOverride: "",
    stationName: "",
    showName: "",
  },
  relaySources: [],
  activeRelaySource: null,
  enabledModes: { simple: true, studio: true, relay: true },
};

export function useSettings() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    api.loadSettings().then((s) => {
      if (!cancelled) {
        setSettings(s);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const update = useCallback(async (next: Settings) => {
    setSettings(next);
    await api.saveSettings(next);
  }, []);

  return { settings, update, loaded };
}

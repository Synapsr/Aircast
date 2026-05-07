import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { AppMode } from "@/types";

export function useMode() {
  const [mode, setModeState] = useState<AppMode>("simple");
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    api.getMode().then((m) => {
      if (!cancelled) {
        setModeState(m);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const setMode = useCallback(async (next: AppMode) => {
    await api.setMode(next);
    setModeState(next);
  }, []);

  return { mode, setMode, loaded };
}

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/lib/api";

export function useMonitorMuted() {
  const [muted, setMutedState] = useState(false);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let cancelled = false;
    api.getMonitorMuted().then((m) => {
      if (!cancelled) setMutedState(m);
    });
    listen<boolean>("monitor-state-changed", (event) => {
      setMutedState(event.payload);
    }).then((u) => {
      if (cancelled) u();
      else unlistenRef.current = u;
    });
    return () => {
      cancelled = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, []);

  const setMuted = useCallback(async (next: boolean) => {
    await api.setMonitorMuted(next);
    setMutedState(next);
  }, []);

  const toggle = useCallback(() => setMuted(!muted), [muted, setMuted]);

  return { muted, setMuted, toggle };
}

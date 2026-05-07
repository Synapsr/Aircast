import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/lib/api";

export function useMicOpen() {
  const [open, setOpenState] = useState(false);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let cancelled = false;
    api.getMicOpen().then((o) => {
      if (!cancelled) setOpenState(o);
    });
    listen<boolean>("mic-state-changed", (event) => {
      setOpenState(event.payload);
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

  const setOpen = useCallback(async (next: boolean) => {
    await api.setMicOpen(next);
    setOpenState(next);
  }, []);

  const toggle = useCallback(() => setOpen(!open), [open, setOpen]);

  return { open, setOpen, toggle };
}

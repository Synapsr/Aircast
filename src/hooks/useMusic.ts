import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/lib/api";
import type { MusicSnapshot } from "@/types";

const EMPTY: MusicSnapshot = { state: "stopped", queue: [], current: null };

export function useMusic() {
  const [snapshot, setSnapshot] = useState<MusicSnapshot>(EMPTY);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await api.musicSnapshot();
      setSnapshot(s);
    } catch {
      // ignore — backend may not be ready
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void refresh();
    listen<MusicSnapshot>("music-state-changed", (event) => {
      // Backend emits with no payload sometimes (mode changes); refetch then.
      if (event.payload && typeof event.payload === "object" && "queue" in event.payload) {
        setSnapshot(event.payload);
      } else {
        void refresh();
      }
    }).then((u) => {
      if (cancelled) u();
      else unlistenRef.current = u;
    });

    // Tick to update elapsedSecs while a track is playing
    const tickId = setInterval(() => {
      void refresh();
    }, 1000);

    return () => {
      cancelled = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
      clearInterval(tickId);
    };
  }, [refresh]);

  return { snapshot, refresh };
}

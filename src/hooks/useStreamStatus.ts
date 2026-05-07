import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { StreamStatus } from "@/types";

export function useStreamStatus(): StreamStatus {
  const [status, setStatus] = useState<StreamStatus>({ kind: "idle" });
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let cancelled = false;

    listen<StreamStatus>("stream-status", (event) => {
      setStatus(event.payload);
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        unlistenRef.current = unlisten;
      }
    });

    return () => {
      cancelled = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, []);

  return status;
}

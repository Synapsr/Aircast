import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { UpstreamStatus } from "@/types";

/**
 * Subscribes to `relay-upstream-changed` — emitted by the Rust URL input
 * thread as it transitions between connecting / streaming / reconnecting.
 * Drives the relay-mode status indicator in the UI.
 */
export function useUpstreamStatus(): UpstreamStatus {
  const [status, setStatus] = useState<UpstreamStatus>("idle");
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let cancelled = false;
    listen<UpstreamStatus>("relay-upstream-changed", (event) => {
      setStatus(event.payload);
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

  return status;
}

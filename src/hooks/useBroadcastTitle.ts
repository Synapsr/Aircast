import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Subscribes to the `metadata-broadcast-changed` event emitted by the Rust
 * backend whenever a title is successfully pushed to Icecast (or cleared
 * when the stream stops). Empty string = broadcaster dormant.
 */
export function useBroadcastTitle(): string {
  const [title, setTitle] = useState("");
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let cancelled = false;
    listen<string>("metadata-broadcast-changed", (event) => {
      setTitle(event.payload ?? "");
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

  return title;
}

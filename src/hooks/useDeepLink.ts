import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export function useDeepLink() {
  const [url, setUrl] = useState<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let cancelled = false;
    listen<string>("deep-link-url", (event) => {
      setUrl(event.payload);
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

  const clear = useCallback(() => setUrl(null), []);

  return { url, clear };
}

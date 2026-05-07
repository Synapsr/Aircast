import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "@/lib/api";
import type { CartSnapshot } from "@/types";

export function useCarts() {
  const [carts, setCarts] = useState<CartSnapshot[]>([]);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await api.cartSnapshot();
      setCarts(s);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void refresh();
    listen<CartSnapshot[]>("cart-state-changed", (event) => {
      if (Array.isArray(event.payload)) {
        setCarts(event.payload);
      } else {
        void refresh();
      }
    }).then((u) => {
      if (cancelled) u();
      else unlistenRef.current = u;
    });

    const tickId = setInterval(() => {
      // refresh while any cart is playing to update elapsed
      setCarts((cur) => {
        if (cur.some((c) => c.playing)) {
          void refresh();
        }
        return cur;
      });
    }, 250);

    return () => {
      cancelled = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
      clearInterval(tickId);
    };
  }, [refresh]);

  return { carts, refresh };
}

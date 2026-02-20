import { useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

/**
 * Subscribe to a Tauri event with automatic cleanup.
 * Re-subscribes when event name or callback identity changes.
 */
export function useTauriEvent<T = unknown>(
  event: string,
  callback: (payload: T) => void,
) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen<T>(event, (e) => callback(e.payload)).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [event, callback]);
}

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Subscribe to a Tauri event with automatic cleanup.
 * Re-subscribes when event name or callback identity changes.
 *
 * Cleanup is guarded against double-invocation. `@tauri-apps/api/event`'s
 * internal `unregisterListener` reads `listeners[eventId].handlerId` and
 * throws TypeError if the listener was already removed (StrictMode double-
 * mount or race between `listen()` resolving and the effect's cleanup
 * firing both produce this state). Wrapping the unlisten call in try/catch
 * and clearing the local ref after use prevents the throw from breaking
 * the surrounding effect-cleanup chain, which otherwise leaves component
 * state half-torn-down and surfaces as a permanently-loading UI.
 */
export function useTauriEvent<T = unknown>(
  event: string,
  callback: (payload: T) => void,
) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    const safeUnlisten = (fn: UnlistenFn) => {
      try {
        fn();
      } catch {
        // listener was already removed (StrictMode race or double-cleanup)
      }
    };

    listen<T>(event, (e) => callback(e.payload)).then((fn) => {
      if (cancelled) {
        safeUnlisten(fn);
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      if (unlisten) {
        const fn = unlisten;
        unlisten = undefined;
        safeUnlisten(fn);
      }
    };
  }, [event, callback]);
}

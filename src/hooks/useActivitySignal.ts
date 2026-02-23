import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Signals user activity to the backend for background task throttling.
 *
 * - Reports window focus/blur immediately
 * - Reports interaction (click/keypress) debounced to 1 call per 5 seconds
 *
 * Mount once in RootLayout.
 */
export function useActivitySignal() {
  const lastSignal = useRef(0);

  useEffect(() => {
    function onFocus() {
      invoke("signal_window_focus", { focused: true }).catch(() => {});
    }

    function onBlur() {
      invoke("signal_window_focus", { focused: false }).catch(() => {});
    }

    function onInteraction() {
      const now = Date.now();
      if (now - lastSignal.current < 5000) return; // debounce 5s
      lastSignal.current = now;
      invoke("signal_user_activity").catch(() => {});
    }

    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    window.addEventListener("click", onInteraction, { passive: true });
    window.addEventListener("keydown", onInteraction, { passive: true });

    // Signal initial focus state
    if (document.hasFocus()) {
      onFocus();
    }

    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("click", onInteraction);
      window.removeEventListener("keydown", onInteraction);
    };
  }, []);
}

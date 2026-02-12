import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ClaudeStatus {
  installed: boolean;
  authenticated: boolean;
}

interface UseClaudeStatusReturn {
  status: ClaudeStatus | null;
  checking: boolean;
  aiUnavailable: boolean;
  refresh: () => Promise<void>;
}

const MIN_FOCUS_REFRESH_INTERVAL_MS = 60_000;

/**
 * Lightweight Claude availability check for subtle UI affordances.
 *
 * We intentionally keep this status coarse-grained:
 * - available: installed + authenticated
 * - unavailable: otherwise (includes auth/subscription failures)
 */
export function useClaudeStatus(): UseClaudeStatusReturn {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const lastCheckRef = useRef(0);
  const inFlightRef = useRef(false);

  const refreshStatus = useCallback(async (force: boolean) => {
    const now = Date.now();
    if (!force && now - lastCheckRef.current < MIN_FOCUS_REFRESH_INTERVAL_MS) {
      return;
    }
    if (inFlightRef.current) {
      return;
    }

    inFlightRef.current = true;
    lastCheckRef.current = now;
    setChecking(true);
    try {
      const result = await invoke<ClaudeStatus>("check_claude_status");
      setStatus(result);
    } catch {
      setStatus({ installed: false, authenticated: false });
    } finally {
      inFlightRef.current = false;
      setChecking(false);
    }
  }, []);

  const refresh = useCallback(async () => {
    await refreshStatus(true);
  }, [refreshStatus]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    const onFocus = () => {
      void refreshStatus(false);
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refreshStatus]);

  const aiUnavailable = Boolean(status && (!status.installed || !status.authenticated));

  return { status, checking, aiUnavailable, refresh };
}

/**
 * useClaudeStatus.ts
 *
 * Shared Claude Code availability state. All consumers read from a single
 * context so refreshing in one place updates every status indicator (Settings
 * section, Settings banner, dashboard Header).
 *
 * ClaudeCode.tsx (onboarding) manages its own state and is not a consumer.
 */

import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface ClaudeStatus {
  installed: boolean;
  authenticated: boolean;
  nodeInstalled?: boolean;
}

interface ClaudeStatusContext {
  status: ClaudeStatus | null;
  checking: boolean;
  aiUnavailable: boolean;
  /** Normal refresh — respects 5-minute TTL cache. */
  refresh: () => Promise<void>;
  /** Force refresh — clears backend cache first, then re-checks. */
  forceRefresh: () => Promise<void>;
}

const MIN_FOCUS_REFRESH_INTERVAL_MS = 300_000;

const ClaudeStatusCtx = createContext<ClaudeStatusContext>({
  status: null,
  checking: true,
  aiUnavailable: true,
  refresh: async () => {},
  forceRefresh: async () => {},
});

export function useClaudeStatus(): ClaudeStatusContext {
  return useContext(ClaudeStatusCtx);
}

export { ClaudeStatusCtx };

/**
 * Provider hook — call once in router.tsx, pass to ClaudeStatusCtx.Provider.
 */
export function useClaudeStatusProvider(): ClaudeStatusContext {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const lastCheckRef = useRef(0);
  const inFlightRef = useRef(false);

  const doCheck = useCallback(async (clearCache: boolean) => {
    if (inFlightRef.current) return;

    // Respect TTL for non-forced checks
    if (!clearCache) {
      const now = Date.now();
      if (now - lastCheckRef.current < MIN_FOCUS_REFRESH_INTERVAL_MS) return;
    }

    inFlightRef.current = true;
    lastCheckRef.current = Date.now();
    setChecking(true);
    try {
      if (clearCache) {
        await invoke("clear_claude_status_cache");
      }
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
    await doCheck(false);
  }, [doCheck]);

  const forceRefresh = useCallback(async () => {
    await doCheck(true);
  }, [doCheck]);

  // Initial check
  useEffect(() => {
    void doCheck(false);
  }, [doCheck]);

  // Re-check on window focus (respects TTL)
  useEffect(() => {
    const onFocus = () => { void doCheck(false); };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [doCheck]);

  const aiUnavailable = Boolean(status && (!status.installed || !status.authenticated));

  return { status, checking, aiUnavailable, refresh, forceRefresh };
}

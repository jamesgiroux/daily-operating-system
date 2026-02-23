import { useState, useEffect, useCallback, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ExecutiveIntelligence } from "@/types";

export function useExecutiveIntelligence() {
  const [data, setData] = useState<ExecutiveIntelligence | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [, startTransition] = useTransition();

  const load = useCallback(async (silent = false) => {
    try {
      const result = await invoke<ExecutiveIntelligence>(
        "get_executive_intelligence"
      );
      const apply = () => {
        setData(result);
        setError(null);
      };
      if (silent) {
        startTransition(apply);
      } else {
        apply();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  // Load on mount
  useEffect(() => {
    load();
  }, [load]);

  // Refresh after workflow completion — silent to avoid content blink
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen("workflow-completed", () => {
      load(true);
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [load]);

  // Refresh on calendar updates — silent
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen("calendar-updated", () => {
      load(true);
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [load]);

  const totalSignals = data
    ? data.signalCounts.decisions +
      data.signalCounts.delegations +
      data.signalCounts.portfolioAlerts +
      // cancelable excluded — now shown as badge on MeetingCard (ADR-0055)
      data.signalCounts.skipToday
    : 0;

  return { data, error, refresh: load, totalSignals };
}

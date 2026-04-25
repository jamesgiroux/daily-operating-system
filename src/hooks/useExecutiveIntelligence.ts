import { useState, useEffect, useCallback, useTransition } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ExecutiveIntelligence } from "@/types";
import { useTauriEvent } from "./useTauriEvent";

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

  const refreshSilently = useCallback(() => {
    load(true);
  }, [load]);

  // Refresh after workflow completion — silent to avoid content blink
  useTauriEvent("workflow-completed", refreshSilently);

  // Refresh on calendar updates — silent
  useTauriEvent("calendar-updated", refreshSilently);

  const totalSignals = data
    ? data.signalCounts.decisions +
      data.signalCounts.delegations +
      data.signalCounts.portfolioAlerts +
      // cancelable excluded — now shown as badge on MeetingCard (ADR-0055)
      data.signalCounts.skipToday
    : 0;

  return { data, error, refresh: load, totalSignals };
}

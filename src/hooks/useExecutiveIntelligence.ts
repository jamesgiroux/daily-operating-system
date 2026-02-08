import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ExecutiveIntelligence } from "@/types";

export function useExecutiveIntelligence() {
  const [data, setData] = useState<ExecutiveIntelligence | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const result = await invoke<ExecutiveIntelligence>(
        "get_executive_intelligence"
      );
      setData(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  // Load on mount
  useEffect(() => {
    load();
  }, [load]);

  // Refresh after workflow completion (briefing may flag new decisions)
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen("workflow-completed", () => {
      load();
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [load]);

  // Refresh on calendar updates (new meetings may be cancelable)
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    listen("calendar-updated", () => {
      load();
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

import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SyncFreshness {
  source: string;
  status: "green" | "amber" | "red" | "unknown";
  lastSuccessAt: string | null;
  lastAttemptAt: string | null;
  lastError: string | null;
  consecutiveFailures: number;
  ageDescription: string;
}

export function useConnectivity() {
  const [freshness, setFreshness] = useState<SyncFreshness[]>([]);

  const load = useCallback(async () => {
    try {
      const result = await invoke<SyncFreshness[]>("get_sync_freshness");
      setFreshness(result);
    } catch {
      // Silently fail — connectivity check shouldn't break the app
    }
  }, []);

  useEffect(() => {
    load();
    const interval = setInterval(load, 60_000); // Poll every 60s
    return () => clearInterval(interval);
  }, [load]);

  const isFullyFresh = useMemo(
    () => freshness.length > 0 && freshness.every((f) => f.status === "green"),
    [freshness],
  );

  const staleServices = useMemo(
    () => freshness.filter((f) => f.status === "amber" || f.status === "red"),
    [freshness],
  );

  const oldestUpdate = useMemo(() => {
    if (staleServices.length === 0) return null;
    const sorted = [...staleServices].sort((a, b) => {
      if (!a.lastSuccessAt) return -1;
      if (!b.lastSuccessAt) return 1;
      return a.lastSuccessAt < b.lastSuccessAt ? -1 : 1;
    });
    return sorted[0]?.ageDescription ?? null;
  }, [staleServices]);

  return { freshness, isFullyFresh, staleServices, oldestUpdate, refresh: load };
}

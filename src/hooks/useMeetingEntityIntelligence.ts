import { useCallback, useEffect, useRef, useState } from "react";

import { useTauriEvent } from "./useTauriEvent";
import {
  invokeEntityIntelligenceAbility,
  type EntityIntelligenceAbilityResponse,
} from "@/services/entity-intelligence/invoke";

export interface UseMeetingEntityIntelligenceResult {
  response: EntityIntelligenceAbilityResponse | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useMeetingEntityIntelligence(
  meetingId: string | null | undefined,
): UseMeetingEntityIntelligenceResult {
  const [response, setResponse] = useState<EntityIntelligenceAbilityResponse | null>(null);
  const [loading, setLoading] = useState(Boolean(meetingId));
  const [error, setError] = useState<string | null>(null);
  const inFlightKeyRef = useRef<string | null>(null);
  const generationRef = useRef(0);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const load = useCallback(async (showLoading = true) => {
    if (!meetingId) {
      generationRef.current += 1;
      inFlightKeyRef.current = null;
      setResponse(null);
      setLoading(false);
      setError(null);
      return;
    }
    const requestKey = meetingId;
    if (inFlightKeyRef.current === requestKey) {
      return;
    }
    inFlightKeyRef.current = requestKey;
    const generation = ++generationRef.current;
    if (showLoading) {
      setLoading(true);
      setResponse(null);
    }
    setError(null);
    try {
      const next = await invokeEntityIntelligenceAbility({
        entityType: "meeting",
        entityId: meetingId,
        depth: "standard",
        sections: ["facts", "health", "open_loops", "record"],
        renderSurface: "tauri_meeting_detail",
      });
      if (generationRef.current !== generation) return;
      setResponse(next);
    } catch (err) {
      if (generationRef.current !== generation) return;
      setResponse(null);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (generationRef.current === generation) {
        setLoading(false);
      }
      if (inFlightKeyRef.current === requestKey) {
        inFlightKeyRef.current = null;
      }
    }
  }, [meetingId]);

  const scheduleRefresh = useCallback((payload?: unknown) => {
    if (
      payload
      && typeof payload === "object"
      && "meetingId" in payload
      && (payload as { meetingId?: unknown }).meetingId !== meetingId
    ) {
      return;
    }
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      void load(false);
    }, 300);
  }, [load, meetingId]);

  useEffect(() => {
    void load(true);
  }, [load]);

  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, []);

  useTauriEvent("prep-ready", scheduleRefresh);
  useTauriEvent("entity-updated", scheduleRefresh);
  useTauriEvent("meeting-briefing-refresh-complete", scheduleRefresh);

  return {
    response,
    loading,
    error,
    refresh: () => {
      void load(false);
    },
  };
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useTauriEvent } from "./useTauriEvent";
import {
  invokeDailyBriefingAbility,
  type DailyBriefingAbilityResponse,
} from "@/services/daily-briefing/invoke";
import type { BriefingSection } from "@/services/daily-briefing/contracts";

interface ConfigResponse {
  workspacePath?: string;
  workspace_path?: string;
}

export interface UseDailyBriefingAbilityOptions {
  date?: string;
  sections?: BriefingSection[];
  workspaceId?: string | null;
}

export interface UseDailyBriefingAbilityResult {
  response: DailyBriefingAbilityResponse | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

const DEFAULT_SECTIONS: BriefingSection[] = [
  "state",
  "current_meeting",
  "next_meeting",
  "upcoming_meetings",
  "trust_summary",
];

function localDateString(date = new Date()): string {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function workspaceIdFromConfig(config: ConfigResponse): string | null {
  const workspaceId = config.workspacePath ?? config.workspace_path ?? "";
  return workspaceId.trim().length > 0 ? workspaceId : null;
}

export function useDailyBriefingAbility({
  date = localDateString(),
  sections = DEFAULT_SECTIONS,
  workspaceId: providedWorkspaceId = null,
}: UseDailyBriefingAbilityOptions = {}): UseDailyBriefingAbilityResult {
  const [response, setResponse] = useState<DailyBriefingAbilityResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const inFlightKeyRef = useRef<string | null>(null);
  const generationRef = useRef(0);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const sectionsKey = sections.join("|");
  const requestedSections = useMemo(
    () => sectionsKey.split("|").filter(Boolean) as BriefingSection[],
    [sectionsKey],
  );

  const load = useCallback(async (showLoading = true) => {
    const requestKey = `${date}|${providedWorkspaceId ?? "config"}|${sectionsKey}`;
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
      const workspaceId = providedWorkspaceId ?? workspaceIdFromConfig(
        await invoke<ConfigResponse>("get_config"),
      );
      if (!workspaceId) {
        throw new Error("Workspace is not configured");
      }
      const next = await invokeDailyBriefingAbility({
        date,
        workspaceId,
        sections: requestedSections,
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
  }, [date, providedWorkspaceId, requestedSections, sectionsKey]);

  const scheduleRefresh = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      void load(false);
    }, 300);
  }, [load]);

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

  useTauriEvent("workflow-completed", scheduleRefresh);
  useTauriEvent("calendar-updated", scheduleRefresh);
  useTauriEvent("prep-ready", scheduleRefresh);
  useTauriEvent("entity-updated", scheduleRefresh);

  return {
    response,
    loading,
    error,
    refresh: () => {
      void load(false);
    },
  };
}

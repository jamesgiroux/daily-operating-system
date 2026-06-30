import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectedCompositionCommandResponse,
  RenderedProvenance,
} from "@/services/composition/contracts";

interface UseProjectedCompositionState {
  data: ProjectedCompositionCommandResponse | null;
  loading: boolean;
  error: string | null;
  renderedProvenance: RenderedProvenance | null;
  refetch: (options?: UseProjectedCompositionRefetchOptions) => Promise<void>;
}

export type CompositionSubjectKind = "account" | "project" | "person" | "action" | "briefing" | "meeting";

export interface ProjectedCompositionSubject {
  entityType: CompositionSubjectKind;
  entityId: string | undefined;
}

type UseProjectedCompositionInput = string | ProjectedCompositionSubject | undefined;

interface UseProjectedCompositionRefetchOptions {
  forceRefresh?: boolean;
}

interface ProjectedCompositionCacheEntry {
  response: ProjectedCompositionCommandResponse;
  cacheHintToken: string | null;
  compositionVersion: number;
}

const PRODUCER_BY_SUBJECT: Record<CompositionSubjectKind, string> = {
  account: "dailyos/account-overview",
  project: "dailyos/project-overview",
  person: "dailyos/person-overview",
  action: "dailyos/action-detail",
  briefing: "dailyos/daily-briefing",
  meeting: "dailyos/meeting-detail",
};

const projectedCompositionCache = new Map<string, ProjectedCompositionCacheEntry>();

function normalizeSubject(input: UseProjectedCompositionInput): ProjectedCompositionSubject | null {
  if (typeof input === "string") {
    return { entityType: "account", entityId: input };
  }
  return input ?? null;
}

export function compositionIdForSubject(subject: ProjectedCompositionSubject | null): string | null {
  if (!subject?.entityId?.trim()) return null;
  return `${PRODUCER_BY_SUBJECT[subject.entityType]}:${subject.entityType}:${subject.entityId}`;
}

export function __clearProjectedCompositionCacheForTests() {
  projectedCompositionCache.clear();
}

export function useProjectedComposition(accountId: string | undefined): UseProjectedCompositionState;
export function useProjectedComposition(subject: ProjectedCompositionSubject | undefined): UseProjectedCompositionState;
export function useProjectedComposition(input: UseProjectedCompositionInput): UseProjectedCompositionState {
  const subject = normalizeSubject(input);
  const entityLabel = subject?.entityType ? subject.entityType[0].toUpperCase() + subject.entityType.slice(1) : "Subject";
  const compositionId = useMemo(() => compositionIdForSubject(subject), [subject?.entityId, subject?.entityType]);
  const initialCacheEntry = compositionId ? projectedCompositionCache.get(compositionId) : undefined;
  const [data, setData] = useState<ProjectedCompositionCommandResponse | null>(
    () => initialCacheEntry?.response ?? null,
  );
  const [loading, setLoading] = useState(Boolean(compositionId));
  const [error, setError] = useState<string | null>(null);
  const cacheHintTokenRef = useRef<string | null>(initialCacheEntry?.cacheHintToken ?? null);
  const compositionVersionRef = useRef<number>(initialCacheEntry?.compositionVersion ?? 0);
  const requestSequenceRef = useRef(0);
  const loadedCompositionIdRef = useRef<string | null>(
    initialCacheEntry && compositionId ? compositionId : null,
  );

  const load = useCallback(async (options?: UseProjectedCompositionRefetchOptions) => {
    const requestSequence = requestSequenceRef.current + 1;
    requestSequenceRef.current = requestSequence;
    const isActiveRequest = () => requestSequenceRef.current === requestSequence;

    if (!compositionId) {
      setData(null);
      setLoading(false);
      setError(`${entityLabel} not found`);
      loadedCompositionIdRef.current = null;
      return;
    }

    if (loadedCompositionIdRef.current !== compositionId) {
      const cached = projectedCompositionCache.get(compositionId);
      if (cached) {
        cacheHintTokenRef.current = cached.cacheHintToken;
        compositionVersionRef.current = cached.compositionVersion;
        loadedCompositionIdRef.current = compositionId;
        setData(cached.response);
      } else {
        setData(null);
      }
    }
    setLoading(true);
    setError(null);
    try {
      const request: {
        compositionId: string;
        compositionVersion: number;
        cacheHintToken: string | null;
        forceRefresh?: boolean;
      } = {
        compositionId,
        compositionVersion: compositionVersionRef.current,
        cacheHintToken: cacheHintTokenRef.current,
      };
      if (options?.forceRefresh) {
        request.forceRefresh = true;
      }
      const response = await invoke<ProjectedCompositionCommandResponse>(
        "get_projected_composition",
        request,
      );
      if (!isActiveRequest()) return;
      cacheHintTokenRef.current = response.cache_hint_token;
      compositionVersionRef.current = response.projection.composition_version ?? 0;
      loadedCompositionIdRef.current = compositionId;
      projectedCompositionCache.set(compositionId, {
        response,
        cacheHintToken: response.cache_hint_token,
        compositionVersion: response.projection.composition_version ?? 0,
      });
      setData(response);
    } catch (err) {
      if (!isActiveRequest()) return;
      loadedCompositionIdRef.current = compositionId;
      // Tauri commands that return a structured error (e.g. BridgeSurfaceError)
      // reject with an object, not a string — String(object) is "[object Object]".
      // Surface a readable message instead.
      let message: string;
      if (err instanceof Error) {
        message = err.message;
      } else if (typeof err === "string") {
        message = err;
      } else if (err && typeof err === "object" && typeof (err as { message?: unknown }).message === "string") {
        message = (err as { message: string }).message;
      } else {
        try {
          message = JSON.stringify(err);
        } catch {
          message = String(err);
        }
      }
      setError(message);
    } finally {
      if (isActiveRequest()) setLoading(false);
    }
  }, [compositionId, entityLabel]);

  useEffect(() => {
    const cached = compositionId ? projectedCompositionCache.get(compositionId) : undefined;
    cacheHintTokenRef.current = cached?.cacheHintToken ?? null;
    compositionVersionRef.current = cached?.compositionVersion ?? 0;
    if (cached && compositionId) {
      loadedCompositionIdRef.current = compositionId;
      setData(cached.response);
    }
    void load();
  }, [compositionId, load]);

  const subjectMatchesLoadedData = Boolean(compositionId) && loadedCompositionIdRef.current === compositionId;
  const visibleData = subjectMatchesLoadedData ? data : null;
  const visibleError = subjectMatchesLoadedData || !compositionId ? error : null;
  const visibleLoading = Boolean(compositionId) && !visibleError && (!subjectMatchesLoadedData || loading);

  const renderedProvenance = useMemo(
    () => visibleData?.rendered_provenance ?? null,
    [visibleData?.rendered_provenance],
  );

  return {
    data: visibleData,
    loading: visibleLoading,
    error: visibleError,
    renderedProvenance,
    refetch: load,
  };
}

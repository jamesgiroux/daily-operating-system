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

export type CompositionSubjectKind = "account" | "project" | "person" | "action";

export interface ProjectedCompositionSubject {
  entityType: CompositionSubjectKind;
  entityId: string | undefined;
}

type UseProjectedCompositionInput = string | ProjectedCompositionSubject | undefined;

interface UseProjectedCompositionRefetchOptions {
  forceRefresh?: boolean;
}

const PRODUCER_BY_SUBJECT: Record<CompositionSubjectKind, string> = {
  account: "dailyos/account-overview",
  project: "dailyos/project-overview",
  person: "dailyos/person-overview",
  action: "dailyos/action-detail",
};

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

export function useProjectedComposition(accountId: string | undefined): UseProjectedCompositionState;
export function useProjectedComposition(subject: ProjectedCompositionSubject | undefined): UseProjectedCompositionState;
export function useProjectedComposition(input: UseProjectedCompositionInput): UseProjectedCompositionState {
  const subject = normalizeSubject(input);
  const entityLabel = subject?.entityType ? subject.entityType[0].toUpperCase() + subject.entityType.slice(1) : "Subject";
  const compositionId = useMemo(() => compositionIdForSubject(subject), [subject?.entityId, subject?.entityType]);
  const [data, setData] = useState<ProjectedCompositionCommandResponse | null>(null);
  const [loading, setLoading] = useState(Boolean(compositionId));
  const [error, setError] = useState<string | null>(null);
  const cacheHintTokenRef = useRef<string | null>(null);
  const compositionVersionRef = useRef<number>(0);
  const requestSequenceRef = useRef(0);
  const loadedCompositionIdRef = useRef<string | null>(null);

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
      setData(null);
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
      setData(response);
    } catch (err) {
      if (!isActiveRequest()) return;
      loadedCompositionIdRef.current = compositionId;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (isActiveRequest()) setLoading(false);
    }
  }, [compositionId, entityLabel]);

  useEffect(() => {
    cacheHintTokenRef.current = null;
    compositionVersionRef.current = 0;
    void load();
  }, [load]);

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

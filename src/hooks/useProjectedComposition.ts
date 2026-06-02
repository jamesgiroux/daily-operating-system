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
  refetch: () => Promise<void>;
}

function compositionIdForAccount(accountId: string): string {
  return `dailyos/account-overview:account:${accountId}`;
}

export function useProjectedComposition(accountId: string | undefined): UseProjectedCompositionState {
  const [data, setData] = useState<ProjectedCompositionCommandResponse | null>(null);
  const [loading, setLoading] = useState(Boolean(accountId));
  const [error, setError] = useState<string | null>(null);
  const cacheHintTokenRef = useRef<string | null>(null);
  const compositionVersionRef = useRef<number>(0);
  const requestSequenceRef = useRef(0);
  const loadedAccountIdRef = useRef<string | null>(null);

  const load = useCallback(async () => {
    const requestSequence = requestSequenceRef.current + 1;
    requestSequenceRef.current = requestSequence;
    const isActiveRequest = () => requestSequenceRef.current === requestSequence;

    if (!accountId) {
      setData(null);
      setLoading(false);
      setError("Account not found");
      loadedAccountIdRef.current = null;
      return;
    }

    if (loadedAccountIdRef.current !== accountId) {
      setData(null);
    }
    setLoading(true);
    setError(null);
    try {
      const response = await invoke<ProjectedCompositionCommandResponse>("get_projected_composition", {
        compositionId: compositionIdForAccount(accountId),
        compositionVersion: compositionVersionRef.current,
        cacheHintToken: cacheHintTokenRef.current,
      });
      if (!isActiveRequest()) return;
      cacheHintTokenRef.current = response.cache_hint_token;
      compositionVersionRef.current = response.projection.composition_version ?? 0;
      loadedAccountIdRef.current = accountId;
      setData(response);
    } catch (err) {
      if (!isActiveRequest()) return;
      loadedAccountIdRef.current = accountId;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (isActiveRequest()) setLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    cacheHintTokenRef.current = null;
    compositionVersionRef.current = 0;
    void load();
  }, [load]);

  const accountMatchesLoadedData = Boolean(accountId) && loadedAccountIdRef.current === accountId;
  const visibleData = accountMatchesLoadedData ? data : null;
  const visibleError = accountMatchesLoadedData || !accountId ? error : null;
  const visibleLoading = Boolean(accountId) && !visibleError && (!accountMatchesLoadedData || loading);

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

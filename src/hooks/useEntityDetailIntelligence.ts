import { useCallback, useEffect, useRef, useState } from "react";

import { useTauriEvent } from "./useTauriEvent";
import {
  invokeEntityIntelligenceAbility,
  type EntityIntelligenceAbilityResponse,
} from "@/services/entity-intelligence/invoke";
import type { EntityKind } from "@/services/entity-intelligence/contracts";

interface EntityRefreshPayload {
  claimId?: string;
  claim_id?: string;
  entityId?: string;
  entity_id?: string;
  entityType?: string;
  entity_type?: string;
}

export interface UseEntityDetailIntelligenceResult {
  response: EntityIntelligenceAbilityResponse | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

export function useEntityDetailIntelligence(
  entityType: Exclude<EntityKind, "meeting">,
  entityId: string | null | undefined,
): UseEntityDetailIntelligenceResult {
  const [response, setResponse] = useState<EntityIntelligenceAbilityResponse | null>(null);
  const [loading, setLoading] = useState(Boolean(entityId));
  const [error, setError] = useState<string | null>(null);
  const inFlightKeyRef = useRef<string | null>(null);
  const generationRef = useRef(0);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRefreshRef = useRef(false);
  const currentClaimIdsRef = useRef<Set<string>>(new Set());
  const currentRequestKey = entityId ? `${entityType}:${entityId}` : null;
  const latestRequestKeyRef = useRef<string | null>(currentRequestKey);
  latestRequestKeyRef.current = currentRequestKey;

  const load = useCallback(async (showLoading = true) => {
    if (!entityId) {
      generationRef.current += 1;
      inFlightKeyRef.current = null;
      pendingRefreshRef.current = false;
      currentClaimIdsRef.current = new Set();
      setResponse(null);
      setLoading(false);
      setError(null);
      return;
    }

    const requestKey = `${entityType}:${entityId}`;
    if (latestRequestKeyRef.current !== requestKey) {
      return;
    }
    if (inFlightKeyRef.current === requestKey) {
      if (!showLoading) {
        pendingRefreshRef.current = true;
      }
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
        entityType,
        entityId,
        depth: "shallow",
        sections: ["facts", "open_loops"],
        renderSurface: "tauri_entity_detail",
      });
      if (generationRef.current !== generation || latestRequestKeyRef.current !== requestKey) {
        return;
      }
      if (!responseMatchesRequest(next, entityType, entityId)) {
        setResponse(null);
        currentClaimIdsRef.current = new Set();
        setError("Entity intelligence response did not match the requested entity.");
        return;
      }
      setResponse(next);
      currentClaimIdsRef.current = claimIdsForResponse(next);
    } catch (err) {
      if (generationRef.current !== generation || latestRequestKeyRef.current !== requestKey) {
        return;
      }
      setResponse(null);
      currentClaimIdsRef.current = new Set();
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (generationRef.current === generation && latestRequestKeyRef.current === requestKey) {
        setLoading(false);
      }
      if (inFlightKeyRef.current === requestKey) {
        inFlightKeyRef.current = null;
        if (pendingRefreshRef.current && latestRequestKeyRef.current === requestKey) {
          pendingRefreshRef.current = false;
          window.setTimeout(() => {
            void load(false);
          }, 0);
        }
      }
    }
  }, [entityId, entityType]);

  const scheduleRefresh = useCallback((payload?: EntityRefreshPayload | string | null) => {
    if (payload && typeof payload === "object") {
      const payloadId = payload.entityId ?? payload.entity_id;
      const payloadType = payload.entityType ?? payload.entity_type;
      if (payloadId && payloadId !== entityId) return;
      if (payloadType && payloadType !== entityType) return;
    }
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      void load(false);
    }, 300);
  }, [entityId, entityType, load]);

  useEffect(() => {
    void load(true);
  }, [load]);

  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    pendingRefreshRef.current = false;
  }, [entityId, entityType]);

  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, []);

  useTauriEvent<EntityRefreshPayload | string | null>("entity-updated", scheduleRefresh);
  useTauriEvent<EntityRefreshPayload | string | null>("intelligence-updated", scheduleRefresh);
  useTauriEvent<EntityRefreshPayload | string | null>(
    "claim_receipt:invalidated",
    (payload) => {
      if (payload && typeof payload === "object") {
        const claimId = payload.claimId ?? payload.claim_id;
        if (claimId && !currentClaimIdsRef.current.has(claimId)) return;
      }
      scheduleRefresh(payload);
    },
  );

  return {
    response,
    loading,
    error,
    refresh: () => {
      void load(false);
    },
  };
}

function claimIdsForResponse(
  response: EntityIntelligenceAbilityResponse | null,
): Set<string> {
  const claimIds = new Set<string>();
  for (const fact of response?.data.facts.items ?? []) {
    claimIds.add(fact.claimId);
  }
  for (const item of response?.data.openLoops.items ?? []) {
    claimIds.add(item.receiptTarget.claimId);
  }
  return claimIds;
}

function responseMatchesRequest(
  response: EntityIntelligenceAbilityResponse,
  entityType: Exclude<EntityKind, "meeting">,
  entityId: string,
): boolean {
  return response.data.subject.kind === entityType && response.data.subject.id === entityId;
}

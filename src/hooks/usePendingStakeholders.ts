/**
 * usePendingStakeholders — DOS-258 Lane F
 *
 * Fetches account_stakeholders rows with status='pending_review' and exposes
 * confirm/dismiss mutations with optimistic removal. The queue refetches after
 * each mutation so the item disappears immediately from the UI.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PendingStakeholderSuggestion } from "@/types";

export interface UsePendingStakeholdersResult {
  suggestions: PendingStakeholderSuggestion[];
  loading: boolean;
  confirm: (personId: string) => Promise<void>;
  dismiss: (personId: string) => Promise<void>;
  /** IDs currently being acted on — drives disabled state on buttons. */
  inFlight: Set<string>;
}

export function usePendingStakeholders(
  accountId: string | undefined,
): UsePendingStakeholdersResult {
  const [suggestions, setSuggestions] = useState<PendingStakeholderSuggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [inFlight, setInFlight] = useState<Set<string>>(new Set());

  const fetch = useCallback(async () => {
    if (!accountId) {
      setSuggestions([]);
      return;
    }
    setLoading(true);
    try {
      const rows = await invoke<PendingStakeholderSuggestion[]>(
        "get_pending_stakeholder_suggestions",
        { accountId },
      );
      setSuggestions(rows);
    } catch {
      setSuggestions([]);
    } finally {
      setLoading(false);
    }
  }, [accountId]);

  // Initial load + re-load when account changes.
  useEffect(() => {
    void fetch();
  }, [fetch]);

  const confirm = useCallback(
    async (personId: string) => {
      if (!accountId) return;
      // Capture item for rollback before optimistic removal.
      const removed = suggestions.find((s) => s.personId === personId);
      setSuggestions((prev) => prev.filter((s) => s.personId !== personId));
      setInFlight((prev) => new Set([...prev, personId]));
      try {
        await invoke("confirm_pending_stakeholder", { accountId, personId });
        void fetch();
      } catch {
        // Server rejected — restore the item so the user can retry.
        if (removed) {
          setSuggestions((prev) =>
            prev.some((s) => s.personId === personId)
              ? prev
              : [...prev, removed],
          );
        }
      } finally {
        setInFlight((prev) => {
          const next = new Set(prev);
          next.delete(personId);
          return next;
        });
      }
    },
    [accountId, fetch, suggestions],
  );

  const dismiss = useCallback(
    async (personId: string) => {
      if (!accountId) return;
      const removed = suggestions.find((s) => s.personId === personId);
      setSuggestions((prev) => prev.filter((s) => s.personId !== personId));
      setInFlight((prev) => new Set([...prev, personId]));
      try {
        await invoke("dismiss_pending_stakeholder", { accountId, personId });
        void fetch();
      } catch {
        if (removed) {
          setSuggestions((prev) =>
            prev.some((s) => s.personId === personId)
              ? prev
              : [...prev, removed],
          );
        }
      } finally {
        setInFlight((prev) => {
          const next = new Set(prev);
          next.delete(personId);
          return next;
        });
      }
    },
    [accountId, fetch, suggestions],
  );

  return { suggestions, loading, confirm, dismiss, inFlight };
}

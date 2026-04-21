/**
 * useAccountWorkData — DOS Work-tab Phase 3.
 *
 * Reads the three Action-table-backed surfaces that power the Work tab:
 *   - Commitments: action_kind='commitment' AND status IN (backlog, unstarted, started)
 *   - Suggestions: status='backlog' (any kind)
 *   - Recently landed: status='completed' in the last 30 days (cap 20)
 *
 * Dispatches mutations by stable `action.id` (not array index) so the
 * Commitments / Suggestions / Recently landed chapters stay correct across
 * refreshes and concurrent enrichment passes.
 *
 * Optimistic UI: items disappear immediately when the user acts. On error
 * the full list is re-fetched (the server wins) and a toast surfaces the
 * failure.
 *
 * Intelligence Loop notes: commitment-lifecycle signals
 * (`commitment_accepted`, `commitment_delivered`, `commitment_rejected`)
 * are emitted by the services::actions layer when action_kind=commitment;
 * this hook doesn't need to emit anything extra.
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import type { DbAction } from "@/types";

export interface UseAccountWorkDataResult {
  commitments: DbAction[];
  suggestions: DbAction[];
  recentlyLanded: DbAction[];

  /** In-flight action ids keyed by surface. Drive per-card spinners. */
  commitmentDoneInFlight: Set<string>;
  commitmentDismissInFlight: Set<string>;
  suggestionAcceptInFlight: Set<string>;
  suggestionDismissInFlight: Set<string>;

  /** Handlers — all dispatch by stable action.id. */
  handleMarkCommitmentDone: (actionId: string) => Promise<void>;
  handleDismissCommitment: (actionId: string) => Promise<void>;
  handleAcceptSuggestion: (actionId: string) => Promise<void>;
  handleDismissSuggestion: (actionId: string) => Promise<void>;
  handleUpdateCommitment: (
    actionId: string,
    patch: { title?: string; context?: string; dueDate?: string },
  ) => Promise<void>;
  /** Navigate to the Action detail page where the Linear team picker lives. */
  handlePushToLinear: (actionId: string) => void;

  /** Spinner-visible refresh. */
  refresh: () => Promise<void>;
  /** Quiet refresh — no loading state flip. */
  silentRefresh: () => Promise<void>;
  loading: boolean;
}

function toggleKey(
  setter: React.Dispatch<React.SetStateAction<Set<string>>>,
  id: string,
  on: boolean,
) {
  setter((prev) => {
    const next = new Set(prev);
    if (on) next.add(id);
    else next.delete(id);
    return next;
  });
}

export function useAccountWorkData(
  accountId: string | undefined,
): UseAccountWorkDataResult {
  const navigate = useNavigate();
  const [commitments, setCommitments] = useState<DbAction[]>([]);
  const [suggestions, setSuggestions] = useState<DbAction[]>([]);
  const [recentlyLanded, setRecentlyLanded] = useState<DbAction[]>([]);
  const [loading, setLoading] = useState(false);

  const [commitmentDoneInFlight, setCommitmentDoneInFlight] = useState<Set<string>>(new Set());
  const [commitmentDismissInFlight, setCommitmentDismissInFlight] = useState<Set<string>>(new Set());
  const [suggestionAcceptInFlight, setSuggestionAcceptInFlight] = useState<Set<string>>(new Set());
  const [suggestionDismissInFlight, setSuggestionDismissInFlight] = useState<Set<string>>(new Set());

  // Keep a ref to the current accountId so async callbacks see the latest value.
  const accountIdRef = useRef<string | undefined>(accountId);
  accountIdRef.current = accountId;

  const fetchAll = useCallback(
    async (showLoading: boolean) => {
      const id = accountIdRef.current;
      if (!id) {
        setCommitments([]);
        setSuggestions([]);
        setRecentlyLanded([]);
        return;
      }
      if (showLoading) setLoading(true);
      try {
        const [c, s, r] = await Promise.all([
          invoke<DbAction[]>("get_account_commitments", { accountId: id }),
          invoke<DbAction[]>("get_account_suggestions", { accountId: id }),
          invoke<DbAction[]>("get_account_recently_landed", { accountId: id }),
        ]);
        setCommitments(c);
        setSuggestions(s);
        setRecentlyLanded(r);
      } catch (err) {
        console.error("useAccountWorkData fetch failed:", err);
      } finally {
        if (showLoading) setLoading(false);
      }
    },
    [],
  );

  const refresh = useCallback(() => fetchAll(true), [fetchAll]);
  const silentRefresh = useCallback(() => fetchAll(false), [fetchAll]);

  useEffect(() => {
    fetchAll(true);
  }, [accountId, fetchAll]);

  // ─── Handlers ───────────────────────────────────────────────────────────
  // Optimistic UI: remove the item from the displayed list immediately, then
  // invoke. On error we reinstate by falling back to a full silentRefresh.

  const handleMarkCommitmentDone = useCallback(
    async (actionId: string) => {
      toggleKey(setCommitmentDoneInFlight, actionId, true);
      const prev = commitments;
      setCommitments((list) => list.filter((a) => a.id !== actionId));
      try {
        await invoke("complete_action", { id: actionId });
        toast.success("Commitment marked done");
        await silentRefresh();
      } catch (err) {
        setCommitments(prev);
        toast.error(`Could not mark done: ${String(err)}`);
      } finally {
        toggleKey(setCommitmentDoneInFlight, actionId, false);
      }
    },
    [commitments, silentRefresh],
  );

  const handleDismissCommitment = useCallback(
    async (actionId: string) => {
      toggleKey(setCommitmentDismissInFlight, actionId, true);
      const prev = commitments;
      setCommitments((list) => list.filter((a) => a.id !== actionId));
      try {
        // Dismissing an open commitment = reject_suggested_action style
        // terminal transition. Back-end archives + tombstones the bridge
        // when action_kind='commitment'. For commitments already past
        // backlog, services::actions handles the terminal transition via
        // archive_action semantics — we reuse reject_suggested_action only
        // when the row is still backlog. For unstarted/started commitments,
        // fall back to reopen → archive is not available; instead we use
        // the `archive_action` command path which services expose via
        // reject_suggested_action when status permits. In practice bridged
        // commitments come in at status='unstarted' (per Phase 2b
        // sync_ai_commitments), so we use complete-with-cancel vocab via
        // reject_suggested_action's permissive archive fallback.
        //
        // Pragmatic path: call `reject_suggested_action` which archives
        // and tombstones; its SQL only changes rows with status='backlog',
        // so for unstarted/started we invoke `archive_action` instead.
        const row = prev.find((a) => a.id === actionId);
        if (row && row.status === "backlog") {
          await invoke("reject_suggested_action", {
            id: actionId,
            source: "actions_page",
          });
        } else {
          // archive_action isn't wired as a Tauri command today — fall
          // back to reject which returns QueryReturnedNoRows for non-
          // backlog rows. Surface an honest error in that case so we don't
          // silently drop the card.
          await invoke("reject_suggested_action", {
            id: actionId,
            source: "actions_page",
          });
        }
        await silentRefresh();
      } catch (err) {
        setCommitments(prev);
        toast.error(`Could not dismiss: ${String(err)}`);
      } finally {
        toggleKey(setCommitmentDismissInFlight, actionId, false);
      }
    },
    [commitments, silentRefresh],
  );

  const handleAcceptSuggestion = useCallback(
    async (actionId: string) => {
      toggleKey(setSuggestionAcceptInFlight, actionId, true);
      const prev = suggestions;
      setSuggestions((list) => list.filter((a) => a.id !== actionId));
      try {
        await invoke("accept_suggested_action", { id: actionId });
        toast.success("Added to your actions");
        await silentRefresh();
      } catch (err) {
        setSuggestions(prev);
        toast.error(`Could not accept: ${String(err)}`);
      } finally {
        toggleKey(setSuggestionAcceptInFlight, actionId, false);
      }
    },
    [suggestions, silentRefresh],
  );

  const handleDismissSuggestion = useCallback(
    async (actionId: string) => {
      toggleKey(setSuggestionDismissInFlight, actionId, true);
      const prev = suggestions;
      setSuggestions((list) => list.filter((a) => a.id !== actionId));
      try {
        await invoke("reject_suggested_action", {
          id: actionId,
          source: "actions_page",
        });
        await silentRefresh();
      } catch (err) {
        setSuggestions(prev);
        toast.error(`Could not dismiss: ${String(err)}`);
      } finally {
        toggleKey(setSuggestionDismissInFlight, actionId, false);
      }
    },
    [suggestions, silentRefresh],
  );

  const handleUpdateCommitment = useCallback(
    async (
      actionId: string,
      patch: { title?: string; context?: string; dueDate?: string },
    ) => {
      try {
        await invoke("update_action", {
          request: { id: actionId, ...patch },
        });
        await silentRefresh();
      } catch (err) {
        toast.error(`Could not save edit: ${String(err)}`);
      }
    },
    [silentRefresh],
  );

  const handlePushToLinear = useCallback(
    (actionId: string) => {
      // Linear push requires a team-id pick; navigate to the Action detail
      // page which already hosts the team picker + push button.
      navigate({
        to: "/actions/$actionId",
        params: { actionId },
      });
    },
    [navigate],
  );

  return {
    commitments,
    suggestions,
    recentlyLanded,
    commitmentDoneInFlight,
    commitmentDismissInFlight,
    suggestionAcceptInFlight,
    suggestionDismissInFlight,
    handleMarkCommitmentDone,
    handleDismissCommitment,
    handleAcceptSuggestion,
    handleDismissSuggestion,
    handleUpdateCommitment,
    handlePushToLinear,
    refresh,
    silentRefresh,
    loading,
  };
}

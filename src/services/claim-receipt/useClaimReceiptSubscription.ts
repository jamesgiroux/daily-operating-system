/**
 * `useClaimReceiptSubscription` — AC-339.1 + AC-339.6
 *
 * Subscribes to the existing `claim_verification_state_changed` signal (and
 * adjacent claim-lifecycle signals) and re-fetches the claim receipt via the
 * `render_claim_receipt` Tauri command whenever the underlying claim mutates.
 *
 * Coalescing policy (AC-339.6): 250 ms trailing-edge debounce per
 * `(target.claim_id, surface)` pair. Bursts of state changes (e.g. user
 * triages five stale claims in rapid succession from Actions/Work) collapse
 * into at most one re-fetch per pair per 250 ms window. The pair key ensures
 * fan-out: a feedback action on claim C from ActionsWork re-emits to every
 * subscribed surface (EntityDetail, DailyBriefing, …) within one invalidation
 * cycle.
 *
 * Signal substrate (verified at L1 per R3 risk):
 *   - Registry entry: `signals::policy_registry::SignalType::ClaimVerificationStateChanged`
 *     (durable-claim policy, sync propagation, ClaimSubject resolver).
 *   - Emission site: `services::claims::emit_claim_feedback_signals`
 *     (claims.rs:8713) fires on every verification-state transition driven
 *     by `record_claim_feedback`.
 *
 * The hook does NOT introduce a new signal type — it reuses the existing
 * substrate per memory `feedback_check_substrate_before_authoring_primitives`.
 *
 * Proposal-receipt deferral (per L0-W1 §5.6 + cycle-1 codex-consult F3):
 * `target.kind === "proposal" | "workItem"` will receive a `target_not_found`
 * error from the backend in v1.4.4 W1. W4 extension is planned but NOT
 * budgeted in this packet.
 */

import { invoke as defaultInvoke } from "@tauri-apps/api/core";
import { listen as defaultListen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";

import type { ClaimReceipt, ReceiptTarget, SurfaceContext } from "./contracts";

export const CLAIM_RECEIPT_INVALIDATION_EVENT = "claim_receipt:invalidated";

/**
 * Default trailing-edge debounce window per `(claim_id, surface)` pair.
 *
 * Locked at 250 ms per AC-339.6. Exposed as a constant so test code can
 * reference the same value rather than duplicating a magic number.
 */
export const RECEIPT_COALESCE_WINDOW_MS = 250;

export interface ClaimReceiptInvalidationPayload {
  /** Always `claim_verification_state_changed` in v1.4.4 W1 — kept generic for future signal types. */
  signalType: string;
  /** Claim that mutated. Hooks for OTHER claim_ids ignore the event. */
  claimId: string;
  /** Optional payload from the upstream signal — surfaces are free to inspect, but the hook ignores. */
  from?: string | null;
  to?: string | null;
}

type InvokeFn = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type ListenFn = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<UnlistenFn>;

export interface UseClaimReceiptSubscriptionOptions {
  /** Override the trailing-edge debounce window. Defaults to 250 ms (AC-339.6). */
  coalesceWindowMs?: number;
  /** Disable the subscription (e.g. while the target is `null`). */
  enabled?: boolean;
  /** Test seam: override the Tauri `invoke` binding. */
  invoke?: InvokeFn;
  /** Test seam: override the Tauri `listen` binding. */
  listen?: ListenFn;
}

export interface UseClaimReceiptSubscriptionResult {
  receipt: ClaimReceipt | null;
  error: string | null;
  /** `true` while a debounce window is open with a pending re-fetch. */
  pending: boolean;
  /** `true` during the initial fetch (before the first receipt resolves). */
  loading: boolean;
}

function targetClaimId(target: ReceiptTarget): string | null {
  switch (target.kind) {
    case "claim":
      return target.claimId;
    case "proposal":
      // Proposal-receipt deferral — backend returns target_not_found, but the
      // hook still keys debounce by proposal id so a future W4 extension is
      // a drop-in.
      return target.proposalId;
    case "workItem":
      return target.backingClaimId ?? target.actionId;
    default:
      return null;
  }
}

/**
 * Subscribe to the claim receipt for `target` rendered against `surface`.
 *
 * Returns the latest receipt + a `pending` flag while a debounce window is
 * open. The hook keys its debounce by `(claimId, surface)` so multi-surface
 * fan-out re-emits exactly once per surface per coalesce window.
 */
export function useClaimReceiptSubscription(
  target: ReceiptTarget | null,
  surface: SurfaceContext,
  options: UseClaimReceiptSubscriptionOptions = {},
): UseClaimReceiptSubscriptionResult {
  const {
    coalesceWindowMs = RECEIPT_COALESCE_WINDOW_MS,
    enabled = true,
    invoke = defaultInvoke as InvokeFn,
    listen = defaultListen as ListenFn,
  } = options;

  // Stable serialization key — the effect resubscribes only when the
  // *structural identity* of the target changes, not when a parent re-renders
  // and constructs a new object literal. AC-339.6 fan-out relies on this:
  // every parent render must not retrigger an invoke.
  const targetKey = useMemo(() => (target ? JSON.stringify(target) : null), [target]);
  const claimId = useMemo(
    () => (target ? targetClaimId(target) : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- targetKey collapses target identity
    [targetKey],
  );
  const targetRef = useRef<ReceiptTarget | null>(target);
  targetRef.current = target;

  const [receipt, setReceipt] = useState<ClaimReceipt | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [loading, setLoading] = useState(false);

  // Track in-flight debounce timer per `(claimId, surface)` pair. Map is keyed
  // by `${claimId}\\x00${surface}` to keep the pair structurally distinct.
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastFetchKey = useRef<string | null>(null);

  useEffect(() => {
    if (!enabled || !target || !claimId) {
      setReceipt(null);
      setError(null);
      setPending(false);
      setLoading(false);
      return undefined;
    }

    let cancelled = false;
    const subscriptionKey = `${claimId}\x00${surface}`;
    lastFetchKey.current = subscriptionKey;

    const fetchReceipt = async () => {
      const snapshot = targetRef.current;
      if (!snapshot) return;
      try {
        const next = await invoke<ClaimReceipt>("render_claim_receipt", {
          target: snapshot,
          surface,
        });
        if (cancelled || lastFetchKey.current !== subscriptionKey) return;
        setReceipt(next);
        setError(null);
      } catch (err) {
        if (cancelled || lastFetchKey.current !== subscriptionKey) return;
        setReceipt(null);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        if (!cancelled && lastFetchKey.current === subscriptionKey) {
          setLoading(false);
          setPending(false);
        }
      }
    };

    setLoading(true);
    setPending(false);
    void fetchReceipt();

    const scheduleRefetch = () => {
      if (debounceTimer.current !== null) {
        clearTimeout(debounceTimer.current);
      }
      setPending(true);
      debounceTimer.current = setTimeout(() => {
        debounceTimer.current = null;
        void fetchReceipt();
      }, coalesceWindowMs);
    };

    let unlisten: UnlistenFn | null = null;
    const unlistenPromise = listen<ClaimReceiptInvalidationPayload>(
      CLAIM_RECEIPT_INVALIDATION_EVENT,
      (event) => {
        if (cancelled) return;
        if (!event.payload || event.payload.claimId !== claimId) return;
        scheduleRefetch();
      },
    )
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });

    return () => {
      cancelled = true;
      if (debounceTimer.current !== null) {
        clearTimeout(debounceTimer.current);
        debounceTimer.current = null;
      }
      void unlistenPromise;
      if (unlisten) unlisten();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- targetKey collapses target identity; targetRef carries the live value
  }, [claimId, surface, enabled, coalesceWindowMs, invoke, listen, targetKey]);

  return { receipt, error, pending, loading };
}

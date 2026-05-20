/** @vitest-environment jsdom */

/**
 * AC-339.6 multi-surface fan-out + coalesce window integration test.
 *
 * Two subscribers on the same `claimId` from two different surfaces
 * (`actions_work` + `entity_detail`) both observe a single
 * `claim_receipt:invalidated` emission and re-fetch within one invalidation
 * cycle. The 250 ms trailing-edge debounce per `(claim_id, surface)` pair
 * collapses bursts to at most one re-fetch per pair per coalesce window.
 */

import { act, renderHook } from "@testing-library/react";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
  type Mock,
} from "vitest";

import type { ClaimReceipt, ReceiptTarget } from "../contracts";
import {
  CLAIM_RECEIPT_INVALIDATION_EVENT,
  RECEIPT_COALESCE_WINDOW_MS,
  useClaimReceiptSubscription,
} from "../useClaimReceiptSubscription";

type Listener = (event: {
  payload: { signalType: string; claimId: string };
}) => void;

interface Fixture {
  invoke: Mock;
  listen: Mock;
  emitSignal: (claimId: string) => void;
  receiptFor: (claimId: string, surface: string) => ClaimReceipt;
}

function makeFixture(): Fixture {
  const listeners: Array<{ event: string; handler: Listener }> = [];

  const receiptFor = (claimId: string, surface: string): ClaimReceipt =>
    ({
      target: {
        kind: "claim",
        claimId,
        subject: { account: "acct-1" },
        fieldPath: null,
      },
      surfaceContext: surface as ClaimReceipt["surfaceContext"],
      renderedText: null,
      trust: {
        band: "likely_current",
        sourceAsof: null,
        freshness: "current",
        caveat: null,
        rationale: null,
      },
      lifecycle: {
        claimState: "active",
        surfacingState: "active",
        verificationState: "active",
        updatedAt: null,
      },
      provenance: {
        sources: [],
        fieldPath: null,
        evidenceSummary: null,
        redaction: "none",
      },
      actions: [],
    });

  const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd !== "render_claim_receipt") throw new Error(`unexpected cmd ${cmd}`);
    const target = args?.target as ReceiptTarget;
    const surface = args?.surface as string;
    if (target.kind !== "claim") throw new Error("target_not_found");
    return receiptFor(target.claimId, surface);
  });

  const listen = vi.fn(async (event: string, handler: Listener) => {
    const entry = { event, handler };
    listeners.push(entry);
    return () => {
      const idx = listeners.indexOf(entry);
      if (idx !== -1) listeners.splice(idx, 1);
    };
  });

  const emitSignal = (claimId: string) => {
    for (const { event, handler } of listeners) {
      if (event !== CLAIM_RECEIPT_INVALIDATION_EVENT) continue;
      handler({
        payload: { signalType: "claim_verification_state_changed", claimId },
      });
    }
  };

  return { invoke, listen, emitSignal, receiptFor };
}

const claimTarget = (claimId: string): ReceiptTarget => ({
  kind: "claim",
  claimId,
  subject: { account: "acct-1" },
  fieldPath: null,
});

/**
 * Yield the microtask queue + macrotask queue so promises returned by
 * the mocked `invoke` / `listen` can settle. `vi.useFakeTimers({ toFake: [...] })`
 * leaves real `queueMicrotask` and `Promise` semantics intact, so a simple
 * `await Promise.resolve()` chain works.
 */
const flushMicrotasks = async () => {
  for (let i = 0; i < 4; i++) {
    await Promise.resolve();
  }
};

describe("useClaimReceiptSubscription — AC-339.6 fan-out + coalesce", () => {
  beforeEach(() => {
    // Fake only the timer APIs the hook uses for debouncing — leave promise
    // microtasks alone so `invoke`/`listen` await chains can still resolve.
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("performs initial fetch and surfaces the receipt", async () => {
    const fx = makeFixture();
    const { result } = renderHook(() =>
      useClaimReceiptSubscription(claimTarget("c1"), "actions_work", {
        invoke: fx.invoke,
        listen: fx.listen,
      }),
    );

    await act(async () => {
      await flushMicrotasks();
    });

    expect(fx.invoke).toHaveBeenCalledTimes(1);
    expect(result.current.receipt?.target).toMatchObject({ claimId: "c1" });
    expect(result.current.loading).toBe(false);
    expect(result.current.pending).toBe(false);
  });

  it("fans out a single signal to two surfaces within one invalidation cycle", async () => {
    const fx = makeFixture();

    const actionsHook = renderHook(() =>
      useClaimReceiptSubscription(claimTarget("c1"), "actions_work", {
        invoke: fx.invoke,
        listen: fx.listen,
      }),
    );
    const entityHook = renderHook(() =>
      useClaimReceiptSubscription(claimTarget("c1"), "entity_detail", {
        invoke: fx.invoke,
        listen: fx.listen,
      }),
    );

    // Drain initial fetches for both surfaces.
    await act(async () => {
      await flushMicrotasks();
    });

    expect(fx.invoke).toHaveBeenCalledTimes(2);
    const initialActions = actionsHook.result.current.receipt;
    const initialEntity = entityHook.result.current.receipt;
    expect(initialActions?.surfaceContext).toBe("actions_work");
    expect(initialEntity?.surfaceContext).toBe("entity_detail");

    // Simulate ActionsWork emitting `claim_verification_state_changed`. Both
    // subscribed surfaces must observe the signal and queue a debounced
    // re-fetch.
    act(() => {
      fx.emitSignal("c1");
    });
    expect(actionsHook.result.current.pending).toBe(true);
    expect(entityHook.result.current.pending).toBe(true);

    // Advance past the 250 ms coalesce window — both surfaces re-fetch exactly
    // once.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECEIPT_COALESCE_WINDOW_MS + 1);
    });

    expect(fx.invoke).toHaveBeenCalledTimes(4); // 2 initial + 2 fan-out
    expect(actionsHook.result.current.pending).toBe(false);
    expect(entityHook.result.current.pending).toBe(false);
  });

  it("coalesces a burst of N signals into one re-fetch per (claim, surface)", async () => {
    const fx = makeFixture();
    renderHook(() =>
      useClaimReceiptSubscription(claimTarget("c1"), "actions_work", {
        invoke: fx.invoke,
        listen: fx.listen,
      }),
    );

    await act(async () => {
      await flushMicrotasks();
    });
    expect(fx.invoke).toHaveBeenCalledTimes(1); // initial only

    // Burst: 5 rapid signals within the coalesce window.
    act(() => {
      for (let i = 0; i < 5; i++) {
        fx.emitSignal("c1");
      }
    });

    // Move just under the coalesce window — nothing should have fired yet.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECEIPT_COALESCE_WINDOW_MS - 10);
    });
    expect(fx.invoke).toHaveBeenCalledTimes(1);

    // Cross the trailing edge — exactly one re-fetch fires.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    expect(fx.invoke).toHaveBeenCalledTimes(2);
  });

  it("ignores signals for unrelated claims", async () => {
    const fx = makeFixture();
    renderHook(() =>
      useClaimReceiptSubscription(claimTarget("c1"), "actions_work", {
        invoke: fx.invoke,
        listen: fx.listen,
      }),
    );

    await act(async () => {
      await flushMicrotasks();
    });
    expect(fx.invoke).toHaveBeenCalledTimes(1);

    act(() => {
      fx.emitSignal("some-other-claim");
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECEIPT_COALESCE_WINDOW_MS + 1);
    });
    expect(fx.invoke).toHaveBeenCalledTimes(1);
  });

  it("respects two consecutive coalesce windows", async () => {
    const fx = makeFixture();
    renderHook(() =>
      useClaimReceiptSubscription(claimTarget("c1"), "actions_work", {
        invoke: fx.invoke,
        listen: fx.listen,
      }),
    );

    await act(async () => {
      await flushMicrotasks();
    });
    expect(fx.invoke).toHaveBeenCalledTimes(1);

    // Window 1
    act(() => {
      fx.emitSignal("c1");
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECEIPT_COALESCE_WINDOW_MS + 1);
    });
    expect(fx.invoke).toHaveBeenCalledTimes(2);

    // Window 2 (separated from window 1 — distinct user intent)
    act(() => {
      fx.emitSignal("c1");
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(RECEIPT_COALESCE_WINDOW_MS + 1);
    });
    expect(fx.invoke).toHaveBeenCalledTimes(3);
  });
});

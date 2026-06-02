/** @vitest-environment jsdom */

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import type { ProjectedCompositionCommandResponse } from "@/services/composition/contracts";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve;
    reject = innerReject;
  });
  return { promise, resolve, reject };
}

function response(
  overrides: {
    accountId?: string;
    requestId?: string;
    version?: number;
    cacheHintToken?: string;
  } = {},
): ProjectedCompositionCommandResponse {
  const accountId = overrides.accountId ?? "acct-fixture";
  return {
    ok: true,
    request_id: overrides.requestId ?? "request-fixture",
    cache_hint_token: overrides.cacheHintToken ?? "cache-fixture",
    served_from_cache: false,
    projection: {
      composition_id: `dailyos/account-overview:account:${accountId}`,
      composition_version: overrides.version ?? 7,
      fallback_policy_version: 3,
      sections: [],
      blocks: [],
      diagnostics: [],
      unknown_block_count: 0,
      unknown_block_cap: 5,
      dropped_unknown_block_count: 0,
    },
    rendered_provenance: { surface: "tauri_app", value: { ok: true } },
  };
}

describe("useProjectedComposition", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("invokes projected composition once for an account load", async () => {
    invokeMock.mockResolvedValueOnce(response());

    const { result } = renderHook(() => useProjectedComposition("acct-fixture"));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("get_projected_composition", {
      compositionId: "dailyos/account-overview:account:acct-fixture",
      compositionVersion: 0,
      cacheHintToken: null,
    });
    expect(result.current.data?.projection.composition_version).toBe(7);
    expect(result.current.renderedProvenance?.surface).toBe("tauri_app");
  });

  it("ignores stale responses after account navigation", async () => {
    const first = deferred<ProjectedCompositionCommandResponse>();
    const second = deferred<ProjectedCompositionCommandResponse>();
    invokeMock.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);

    const { result, rerender } = renderHook(
      ({ accountId }: { accountId: string | undefined }) => useProjectedComposition(accountId),
      { initialProps: { accountId: "acct-a" } },
    );

    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));
    rerender({ accountId: "acct-b" });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));

    await act(async () => {
      first.resolve(response({ accountId: "acct-a", requestId: "old-request", version: 1 }));
      await first.promise;
    });

    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();

    await act(async () => {
      second.resolve(response({ accountId: "acct-b", requestId: "new-request", version: 9 }));
      await second.promise;
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data?.request_id).toBe("new-request");
    expect(result.current.data?.projection.composition_id).toBe("dailyos/account-overview:account:acct-b");
  });

  it("clears previous account data while the next account loads", async () => {
    const second = deferred<ProjectedCompositionCommandResponse>();
    invokeMock
      .mockResolvedValueOnce(response({ accountId: "acct-a", requestId: "first-request", version: 1 }))
      .mockReturnValueOnce(second.promise);

    const { result, rerender } = renderHook(
      ({ accountId }: { accountId: string | undefined }) => useProjectedComposition(accountId),
      { initialProps: { accountId: "acct-a" } },
    );

    await waitFor(() => expect(result.current.data?.request_id).toBe("first-request"));

    rerender({ accountId: "acct-b" });
    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.renderedProvenance).toBeNull();
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));

    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();

    await act(async () => {
      second.resolve(response({ accountId: "acct-b", requestId: "second-request", version: 2 }));
      await second.promise;
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data?.request_id).toBe("second-request");
  });

  it("surfaces first-load errors instead of keeping the account loading", async () => {
    invokeMock.mockRejectedValueOnce(new Error("projection unavailable"));

    const { result } = renderHook(() => useProjectedComposition("acct-fixture"));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("projection unavailable");
    expect(result.current.data).toBeNull();
    expect(result.current.renderedProvenance).toBeNull();
  });

  it("keeps loading visible when navigating away from a failed account", async () => {
    const second = deferred<ProjectedCompositionCommandResponse>();
    invokeMock.mockRejectedValueOnce(new Error("projection unavailable")).mockReturnValueOnce(second.promise);

    const { result, rerender } = renderHook(
      ({ accountId }: { accountId: string | undefined }) => useProjectedComposition(accountId),
      { initialProps: { accountId: "acct-a" } },
    );

    await waitFor(() => expect(result.current.error).toBe("projection unavailable"));
    expect(result.current.loading).toBe(false);

    rerender({ accountId: "acct-b" });
    expect(result.current.loading).toBe(true);
    expect(result.current.error).toBeNull();
    expect(result.current.data).toBeNull();

    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));
    await act(async () => {
      second.resolve(response({ accountId: "acct-b", requestId: "second-request", version: 2 }));
      await second.promise;
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.data?.request_id).toBe("second-request");
  });
});

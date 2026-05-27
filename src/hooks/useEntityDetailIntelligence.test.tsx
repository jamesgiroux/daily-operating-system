/** @vitest-environment jsdom */

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  invokeEntityIntelligenceAbility,
  type EntityIntelligenceAbilityResponse,
} from "@/services/entity-intelligence/invoke";
import type { EntityKind } from "@/services/entity-intelligence/contracts";
import { useEntityDetailIntelligence } from "./useEntityDetailIntelligence";
import { useTauriEvent } from "./useTauriEvent";

vi.mock("@/services/entity-intelligence/invoke", () => ({
  invokeEntityIntelligenceAbility: vi.fn(),
}));

vi.mock("./useTauriEvent", () => ({
  useTauriEvent: vi.fn(),
}));

const invokeMock = vi.mocked(invokeEntityIntelligenceAbility);
const useTauriEventMock = vi.mocked(useTauriEvent);
const eventHandlers = new Map<string, (payload?: unknown) => void>();

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

function responseFor(
  entityType: Exclude<EntityKind, "meeting">,
  entityId: string,
): EntityIntelligenceAbilityResponse {
  return {
    invocation_id: `inv-${entityId}`,
    ability_name: "get_entity_intelligence",
    ability_version: "0.1.0",
    schema_version: 1,
    data: {
      schemaVersion: 1,
      subject: {
        kind: entityType,
        id: entityId,
        subjectRef: { [entityType]: entityId } as { account: string },
        displayLabel: `${entityType}:${entityId}`,
      },
      facts: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      healthStory: null,
      metadataProposals: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      openLoops: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      touchpoints: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      threads: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      sections: {
        facts: { kind: "present", item_count: 0 },
        metadata_proposals: { kind: "present", item_count: 0 },
        open_loops: { kind: "present", item_count: 0 },
        touchpoints: { kind: "present", item_count: 0 },
        record: { kind: "present", item_count: 0 },
      },
      recordEntries: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      trust: {
        aggregateBand: "unscored",
        sectionCaveats: {},
      },
      provenance: { sources: [], redactionApplied: false },
      sensitivity: "internal",
    },
    rendered_provenance: { value: {} },
  };
}

describe("useEntityDetailIntelligence", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    eventHandlers.clear();
    vi.useRealTimers();
    useTauriEventMock.mockReset();
    useTauriEventMock.mockImplementation((event, handler) => {
      eventHandlers.set(event, handler);
    });
  });

  it("requests claim-backed account detail intelligence through get_entity_intelligence", async () => {
    invokeMock.mockResolvedValueOnce(responseFor("account", "account-1"));

    const { result } = renderHook(() =>
      useEntityDetailIntelligence("account", "account-1"),
    );

    await waitFor(() => {
      expect(result.current.response?.data.subject.id).toBe("account-1");
    });

    expect(invokeMock).toHaveBeenCalledWith({
      entityType: "account",
      entityId: "account-1",
      depth: "shallow",
      sections: ["facts", "open_loops"],
      renderSurface: "tauri_entity_detail",
    });
  });

  it("lets a new entity request supersede an in-flight older request", async () => {
    const first = deferred<EntityIntelligenceAbilityResponse>();
    const second = responseFor("project", "project-b");
    invokeMock
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce(second);

    const { result, rerender } = renderHook(
      ({ projectId }) => useEntityDetailIntelligence("project", projectId),
      { initialProps: { projectId: "project-a" } },
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(expect.objectContaining({
        entityId: "project-a",
      }));
    });

    rerender({ projectId: "project-b" });

    await waitFor(() => {
      expect(result.current.response?.data.subject.id).toBe("project-b");
    });

    await act(async () => {
      first.resolve(responseFor("project", "project-a"));
      await first.promise;
    });

    expect(result.current.response?.data.subject.id).toBe("project-b");
  });

  it("queues a same-entity refresh that arrives while a request is in flight", async () => {
    vi.useFakeTimers();
    const first = deferred<EntityIntelligenceAbilityResponse>();
    invokeMock
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce(responseFor("account", "account-1"));

    const { result } = renderHook(() =>
      useEntityDetailIntelligence("account", "account-1"),
    );

    expect(invokeMock).toHaveBeenCalledTimes(1);

    act(() => {
      eventHandlers.get("entity-updated")?.({
        entity_type: "account",
        entity_id: "account-1",
      });
      vi.advanceTimersByTime(300);
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      first.resolve(responseFor("account", "account-1"));
      await first.promise;
    });

    act(() => {
      vi.runOnlyPendingTimers();
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(result.current.response?.data.subject.id).toBe("account-1");
  });

  it("cancels a pending debounced refresh when the entity changes", async () => {
    invokeMock
      .mockResolvedValueOnce(responseFor("account", "account-a"))
      .mockResolvedValueOnce(responseFor("account", "account-b"));

    const { result, rerender } = renderHook(
      ({ accountId }) => useEntityDetailIntelligence("account", accountId),
      { initialProps: { accountId: "account-a" } },
    );

    await waitFor(() => {
      expect(result.current.response?.data.subject.id).toBe("account-a");
    });

    vi.useFakeTimers();
    act(() => {
      eventHandlers.get("entity-updated")?.({
        entity_type: "account",
        entity_id: "account-a",
      });
    });

    await act(async () => {
      rerender({ accountId: "account-b" });
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(result.current.response?.data.subject.id).toBe("account-b");

    act(() => {
      vi.advanceTimersByTime(300);
    });

    await act(async () => {
      await Promise.resolve();
    });

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(result.current.response?.data.subject.id).toBe("account-b");
  });

  it("rejects a response whose subject does not match the requested entity", async () => {
    invokeMock.mockResolvedValueOnce(responseFor("account", "account-b"));

    const { result } = renderHook(() =>
      useEntityDetailIntelligence("account", "account-a"),
    );

    await waitFor(() => {
      expect(result.current.error).toBe(
        "Entity intelligence response did not match the requested entity.",
      );
    });
    expect(result.current.response).toBeNull();
  });

  it("does not invoke the ability without an entity id", async () => {
    const { result } = renderHook(() =>
      useEntityDetailIntelligence("person", undefined),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

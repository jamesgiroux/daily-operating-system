/** @vitest-environment jsdom */

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  invokeEntityIntelligenceAbility,
  type EntityIntelligenceAbilityResponse,
} from "@/services/entity-intelligence/invoke";
import { useMeetingEntityIntelligence } from "./useMeetingEntityIntelligence";

vi.mock("@/services/entity-intelligence/invoke", () => ({
  invokeEntityIntelligenceAbility: vi.fn(),
}));

vi.mock("./useTauriEvent", () => ({
  useTauriEvent: vi.fn(),
}));

const invokeMock = vi.mocked(invokeEntityIntelligenceAbility);

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

function responseFor(entityId: string): EntityIntelligenceAbilityResponse {
  return {
    invocation_id: `inv-${entityId}`,
    ability_name: "get_entity_intelligence",
    ability_version: "0.1.0",
    schema_version: 1,
    data: {
      schemaVersion: 1,
      subject: {
        kind: "meeting",
        id: entityId,
        subjectRef: { meeting: entityId },
        displayLabel: entityId,
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
        health: { kind: "empty", reason: "not_requested" },
        metadata_proposals: { kind: "empty", reason: "not_requested" },
        open_loops: { kind: "present", item_count: 0 },
        touchpoints: { kind: "empty", reason: "not_requested" },
        threads: { kind: "empty", reason: "not_requested" },
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

describe("useMeetingEntityIntelligence", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("lets a new meeting request supersede an in-flight older request", async () => {
    const first = deferred<EntityIntelligenceAbilityResponse>();
    const second = responseFor("meeting-b");
    invokeMock
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce(second);

    const { result, rerender } = renderHook(
      ({ meetingId }) => useMeetingEntityIntelligence(meetingId),
      { initialProps: { meetingId: "meeting-a" } },
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(expect.objectContaining({ entityId: "meeting-a" }));
    });

    rerender({ meetingId: "meeting-b" });

    await waitFor(() => {
      expect(result.current.response?.data.subject.id).toBe("meeting-b");
    });

    await act(async () => {
      first.resolve(responseFor("meeting-a"));
      await first.promise;
    });

    expect(result.current.response?.data.subject.id).toBe("meeting-b");
  });

  it("clears the previous meeting response while a new meeting is loading", async () => {
    const second = deferred<EntityIntelligenceAbilityResponse>();
    invokeMock
      .mockResolvedValueOnce(responseFor("meeting-a"))
      .mockReturnValueOnce(second.promise);

    const { result, rerender } = renderHook(
      ({ meetingId }) => useMeetingEntityIntelligence(meetingId),
      { initialProps: { meetingId: "meeting-a" } },
    );

    await waitFor(() => {
      expect(result.current.response?.data.subject.id).toBe("meeting-a");
    });

    rerender({ meetingId: "meeting-b" });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(expect.objectContaining({ entityId: "meeting-b" }));
      expect(result.current.response).toBeNull();
      expect(result.current.loading).toBe(true);
    });

    await act(async () => {
      second.resolve(responseFor("meeting-b"));
      await second.promise;
    });

    expect(result.current.response?.data.subject.id).toBe("meeting-b");
  });
});

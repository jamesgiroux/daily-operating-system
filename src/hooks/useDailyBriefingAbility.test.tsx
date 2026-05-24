/** @vitest-environment jsdom */

import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useDailyBriefingAbility } from "./useDailyBriefingAbility";
import type { DailyBriefingAbilityResponse } from "@/services/daily-briefing/invoke";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("./useTauriEvent", () => ({
  useTauriEvent: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

function responseFor(date: string): DailyBriefingAbilityResponse {
  return {
    invocation_id: `inv-${date}`,
    ability_name: "get_daily_briefing",
    ability_version: "0.1.0",
    schema_version: 1,
    data: {
      schemaVersion: 1,
      date,
      state: {
        availability: { kind: "available" },
        freshness: { kind: "fresh" },
        integrity: { kind: "clean" },
        advisories: [],
      },
      currentMeeting: null,
      nextMeeting: null,
      upcomingMeetings: {
        items: [],
        nextCursor: null,
        totalHint: 0,
        cursorState: { kind: "stable" },
      },
      candidateSet: {
        windowStart: null,
        windowEnd: null,
        filterDescription: `date=${date}`,
      },
      watchProposals: [],
      trustSummary: {
        aggregateBand: "unscored",
        likelyCurrentCount: 0,
        useWithCautionCount: 0,
        needsVerificationCount: 0,
      },
      provenance: { sources: [], redactionApplied: false },
      sensitivity: "internal",
      sourceAsofInputs: [],
    },
    rendered_provenance: { value: {} },
  };
}

describe("useDailyBriefingAbility", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("lets a new date request supersede an in-flight older request", async () => {
    const first = deferred<DailyBriefingAbilityResponse>();
    const second = responseFor("2026-05-24");
    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "get_config") {
        return Promise.resolve({ workspacePath: "/workspace" });
      }
      if (
        command === "invoke_ability"
        && (args as { inputJson?: { date?: string } }).inputJson?.date === "2026-05-23"
      ) {
        return first.promise;
      }
      if (
        command === "invoke_ability"
        && (args as { inputJson?: { date?: string } }).inputJson?.date === "2026-05-24"
      ) {
        return Promise.resolve(second);
      }
      throw new Error(`Unexpected invoke ${command}`);
    });

    const { result, rerender } = renderHook(
      ({ date }) => useDailyBriefingAbility({ date }),
      { initialProps: { date: "2026-05-23" } },
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "invoke_ability",
        expect.objectContaining({
          inputJson: expect.objectContaining({ date: "2026-05-23" }),
        }),
      );
    });

    rerender({ date: "2026-05-24" });

    await waitFor(() => {
      expect(result.current.response?.data.date).toBe("2026-05-24");
    });

    await act(async () => {
      first.resolve(responseFor("2026-05-23"));
      await first.promise;
    });

    expect(result.current.response?.data.date).toBe("2026-05-24");
  });

  it("clears the previous date response while a new date is loading", async () => {
    const second = deferred<DailyBriefingAbilityResponse>();
    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "get_config") {
        return Promise.resolve({ workspacePath: "/workspace" });
      }
      if (
        command === "invoke_ability"
        && (args as { inputJson?: { date?: string } }).inputJson?.date === "2026-05-23"
      ) {
        return Promise.resolve(responseFor("2026-05-23"));
      }
      if (
        command === "invoke_ability"
        && (args as { inputJson?: { date?: string } }).inputJson?.date === "2026-05-24"
      ) {
        return second.promise;
      }
      throw new Error(`Unexpected invoke ${command}`);
    });

    const { result, rerender } = renderHook(
      ({ date }) => useDailyBriefingAbility({ date }),
      { initialProps: { date: "2026-05-23" } },
    );

    await waitFor(() => {
      expect(result.current.response?.data.date).toBe("2026-05-23");
    });

    rerender({ date: "2026-05-24" });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "invoke_ability",
        expect.objectContaining({
          inputJson: expect.objectContaining({ date: "2026-05-24" }),
        }),
      );
      expect(result.current.response).toBeNull();
      expect(result.current.loading).toBe(true);
    });

    await act(async () => {
      second.resolve(responseFor("2026-05-24"));
      await second.promise;
    });

    expect(result.current.response?.data.date).toBe("2026-05-24");
  });
});

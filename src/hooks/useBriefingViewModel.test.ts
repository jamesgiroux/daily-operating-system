/** @vitest-environment jsdom */

import { renderHook, waitFor, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useBriefingViewModel } from "./useBriefingViewModel";
import type { BriefingLoadState } from "@/types/briefing";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("useBriefingViewModel", () => {
  it("starts in loading state on first render", () => {
    mockInvoke.mockImplementation(() => new Promise(() => {})); // never resolves
    const { result } = renderHook(() => useBriefingViewModel());
    expect(result.current.state.status).toBe("loading");
  });

  it("resolves to success when invoke returns success envelope", async () => {
    const success: BriefingLoadState = {
      status: "success",
      model: {
        date: { isoDate: "2026-04-23", displayDate: "Thursday" },
        folio: {
          label: "Daily Briefing",
          crumbs: [],
          dateLabel: "X",
          readiness: [],
          actions: [],
        },
        dayStrip: {
          prev: { label: "Yesterday", isoDate: "2026-04-22", preview: "", href: "/" },
          current: { label: "Today", isoDate: "2026-04-23", ariaLabel: "Today" },
          next: { label: "Tomorrow", isoDate: "2026-04-24", preview: "", href: "/" },
        },
        lead: {
          headline: { lead: "Hi" },
          focusCapacity: "x",
        },
        schedule: {
          label: "Today",
          heading: "Today's schedule",
          countLabel: "0",
          meetingMix: { customer: 0, partner: 0, internal: 0, personal: 0, oneOnOne: 0, cancelled: 0 },
          summary: "x",
          dayChart: { rangeStartHour: 8, rangeEndHour: 20, hourTicks: [], legend: [], bars: [], nowLine: null },
          meetings: [],
        },
        predictions: {
          label: "Predictions",
          countLabel: "0 today",
          collapsedLabel: "0 predictions today",
          expandHint: "expand",
          count: 0,
          predictions: [],
        },
        moving: { label: "Moving", heading: "What's moving", countLabel: "0", summary: "x", entities: [] },
        watch: { label: "Watch", heading: "Worth a look", countLabel: "0", summary: "x", rows: [] },
      },
      freshness: { freshness: "unknown" } as unknown as BriefingLoadState extends { status: "success"; freshness: infer F } ? F : never,
    };
    mockInvoke.mockResolvedValueOnce(success);

    const { result } = renderHook(() => useBriefingViewModel());
    await waitFor(() => expect(result.current.state.status).toBe("success"));
  });

  it("resolves to error state when invoke rejects", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("backend offline"));

    const { result } = renderHook(() => useBriefingViewModel());

    await waitFor(() => expect(result.current.state.status).toBe("error"));
    if (result.current.state.status === "error") {
      expect(result.current.state.message).toBe("backend offline");
    }
  });

  it("calls invoke with the expected command name", async () => {
    mockInvoke.mockResolvedValueOnce({ status: "loading" });
    renderHook(() => useBriefingViewModel());
    await waitFor(() => expect(mockInvoke).toHaveBeenCalled());
    expect(mockInvoke).toHaveBeenCalledWith("get_briefing_view_model");
  });

  it("refresh() re-invokes the command without flipping to loading", async () => {
    mockInvoke
      .mockResolvedValueOnce({ status: "empty", message: "first" })
      .mockResolvedValueOnce({ status: "empty", message: "second" });

    const { result } = renderHook(() => useBriefingViewModel());
    await waitFor(() =>
      expect(
        result.current.state.status === "empty"
          ? result.current.state.message
          : null,
      ).toBe("first"),
    );

    act(() => result.current.refresh());

    await waitFor(() =>
      expect(
        result.current.state.status === "empty"
          ? result.current.state.message
          : null,
      ).toBe("second"),
    );
    expect(mockInvoke).toHaveBeenCalledTimes(2);
  });
});

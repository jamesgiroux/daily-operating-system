/** @vitest-environment jsdom */

import { renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useEntityContextEntries } from "./useEntityContextEntries";
import type { AbilityResponseJson, EntityContextOutput, TrajectoryBundle } from "@/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
  },
}));

const invokeMock = vi.mocked(invoke);

describe("useEntityContextEntries", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("requests get_entity_context schema v2 and keeps the trajectory response", async () => {
    const trajectory: TrajectoryBundle = {
      engagement_curve: {
        kind: "engagement_curve",
        entity_id: "acct-1",
        computed_at: "2026-05-09T12:00:00Z",
        confidence: 1,
        series: [
          {
            at: "2026-05-04T00:00:00Z",
            value: {
              meetings_count: 1,
              emails_count: 2,
              bidirectional_ratio: 1,
            },
            source_refs: [],
          },
        ],
      },
    };
    const response: AbilityResponseJson<EntityContextOutput> = {
      invocation_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
      ability_name: "get_entity_context",
      ability_version: "1.0.0",
      schema_version: 2,
      data: {
        entries: [
          {
            id: "ctx-1",
            entityType: "account",
            entityId: "acct-1",
            title: "Renewal risk",
            content: "Champion has changed roles.",
            createdAt: "2026-05-01T12:00:00Z",
            updatedAt: "2026-05-01T12:00:00Z",
          },
        ],
        trajectory,
      },
      rendered_provenance: {
        value: {
          field_attributions: {
            "/entries/0/content": { trust_band: "likely_current" },
            "/entries/0/title": { trust_band: "likely_current" },
          },
        },
      },
    };
    invokeMock.mockResolvedValueOnce(response);

    const { result } = renderHook(() => useEntityContextEntries("account", "acct-1"));

    await waitFor(() => {
      expect(result.current.entries).toHaveLength(1);
    });

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("invoke_ability", {
      abilityName: "get_entity_context",
      inputJson: {
        schema_version: 2,
        entity_type: "account",
        entity_id: "acct-1",
        depth: "standard",
      },
      dryRun: false,
      confirmation: null,
    });
    expect(result.current.trajectory).toEqual(trajectory);
    expect(result.current.entries[0].trustBand).toBe("likely_current");
  });
});

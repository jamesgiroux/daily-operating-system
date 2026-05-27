import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { invokeDailyBriefingAbility } from "./invoke";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("invokeDailyBriefingAbility", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("uses the claim-backed get_daily_briefing ability envelope", async () => {
    const response = {
      invocation_id: "inv-1",
      ability_name: "get_daily_briefing",
      ability_version: "0.1.0",
      schema_version: 1,
      data: {},
      rendered_provenance: { value: {} },
    };
    invokeMock.mockResolvedValueOnce(response);

    await expect(
      invokeDailyBriefingAbility({
        date: "2026-05-23",
        workspaceId: "/workspace",
        sections: ["state", "trust_summary"],
      }),
    ).resolves.toBe(response);

    expect(invokeMock).toHaveBeenCalledWith("invoke_ability", {
      abilityName: "get_daily_briefing",
      inputJson: {
        schemaVersion: 1,
        date: "2026-05-23",
        workspaceId: "/workspace",
        sections: ["state", "trust_summary"],
      },
      renderSurface: "tauri_briefing_prep",
      dryRun: false,
      confirmation: null,
    });
  });
});

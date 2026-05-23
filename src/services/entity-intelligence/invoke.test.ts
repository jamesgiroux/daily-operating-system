import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { invokeEntityIntelligenceAbility } from "./invoke";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

describe("invokeEntityIntelligenceAbility", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("uses get_entity_intelligence with the requested render surface", async () => {
    const response = {
      invocation_id: "inv-1",
      ability_name: "get_entity_intelligence",
      ability_version: "0.1.0",
      schema_version: 1,
      data: {},
      rendered_provenance: { value: {} },
    };
    invokeMock.mockResolvedValueOnce(response);

    await expect(
      invokeEntityIntelligenceAbility({
        entityType: "meeting",
        entityId: "meeting-1",
        depth: "standard",
        sections: ["facts", "health", "open_loops", "record"],
        renderSurface: "tauri_meeting_detail",
      }),
    ).resolves.toBe(response);

    expect(invokeMock).toHaveBeenCalledWith("invoke_ability", {
      abilityName: "get_entity_intelligence",
      inputJson: {
        schemaVersion: 1,
        entityType: "meeting",
        entityId: "meeting-1",
        depth: "standard",
        sections: ["facts", "health", "open_loops", "record"],
      },
      renderSurface: "tauri_meeting_detail",
      dryRun: false,
      confirmation: null,
    });
  });
});

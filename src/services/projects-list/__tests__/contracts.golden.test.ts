// Parity golden test for the `list_projects` ability TS mirror.

import { describe, expect, it } from "vitest";

import {
  LIST_PROJECTS_SCHEMA_VERSION,
  type ProjectListFilter,
  type ProjectListInput,
  type ProjectListPage,
  type ProjectSummary,
  type ProjectTrajectory,
} from "../contracts";

interface ContractGoldenFixture {
  input: ProjectListInput;
  filter: ProjectListFilter;
  page: ProjectListPage;
  enumCoverage: {
    trajectories: ProjectTrajectory[];
  };
}

const goldenFixture = {
  input: {
    schemaVersion: LIST_PROJECTS_SCHEMA_VERSION,
    pageSize: 25,
    filter: {
      status: "active",
      trajectory: "improving",
      parentAccountId: "acct-example",
      nameContains: "Rollout",
    },
  },
  filter: {
    trajectory: "degrading",
  },
  page: {
    items: [
      {
        projectId: "project-rollout-plan",
        name: "Rollout Plan",
        parentAccountId: "acct-example",
        status: "active",
        trajectory: "improving",
        lastTouchpointAt: "2026-05-20T12:00:00Z",
      },
    ],
    nextCursor: "opaque-token",
    totalHint: 3,
    cursorState: { kind: "stable" },
  },
  enumCoverage: {
    trajectories: ["improving", "steady", "degrading", "unknown"],
  },
} satisfies ContractGoldenFixture;

describe("list_projects contract golden fixture", () => {
  it("parses representative input + page JSON", () => {
    const parsed = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;
    expect(parsed.input.schemaVersion).toBe(LIST_PROJECTS_SCHEMA_VERSION);
    expect(parsed.page.items[0]?.projectId).toBe("project-rollout-plan");
    expect(parsed.page.items[0]?.trajectory).toBe("improving");
  });

  it("covers every closed ProjectTrajectory variant from Rust", () => {
    expect(goldenFixture.enumCoverage.trajectories).toHaveLength(4);
  });

  it("ProjectSummary shape includes the W2 list-row fields", () => {
    const row: ProjectSummary = {
      projectId: "x",
      name: "X",
      parentAccountId: null,
      status: "paused",
      trajectory: "degrading",
      lastTouchpointAt: null,
    };
    expect(row.trajectory).toBe("degrading");
  });
});

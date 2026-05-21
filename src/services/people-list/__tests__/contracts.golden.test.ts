// Parity golden test for the `list_people` ability TS mirror.

import { describe, expect, it } from "vitest";

import {
  LIST_PEOPLE_SCHEMA_VERSION,
  type PersonListFilter,
  type PersonListInput,
  type PersonListPage,
  type PersonSummary,
} from "../contracts";

interface ContractGoldenFixture {
  input: PersonListInput;
  filter: PersonListFilter;
  page: PersonListPage;
}

const goldenFixture = {
  input: {
    schemaVersion: LIST_PEOPLE_SCHEMA_VERSION,
    pageSize: 25,
    filter: {
      role: "Engineer",
      primaryAccountId: "acct-example",
      nameContains: "Casey",
    },
  },
  filter: {
    role: "Engineer",
    primaryAccountId: "acct-example",
  },
  page: {
    items: [
      {
        personId: "person-casey",
        displayName: "Casey Chen",
        primaryAccountId: "acct-example",
        role: "Engineer",
        lastTouchpointAt: "2026-05-20T12:00:00Z",
      },
    ],
    nextCursor: null,
    totalHint: 1,
    cursorState: { kind: "stable" },
  },
} satisfies ContractGoldenFixture;

describe("list_people contract golden fixture", () => {
  it("parses representative input + page JSON", () => {
    const parsed = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;
    expect(parsed.input.schemaVersion).toBe(LIST_PEOPLE_SCHEMA_VERSION);
    expect(parsed.page.items[0]?.personId).toBe("person-casey");
    expect(parsed.page.cursorState).toEqual({ kind: "stable" });
  });

  it("PersonSummary shape includes the W2 list-row fields", () => {
    const row: PersonSummary = {
      personId: "x",
      displayName: "X",
      primaryAccountId: null,
      role: "Engineer",
      lastTouchpointAt: null,
    };
    expect(row.primaryAccountId).toBeNull();
    expect(row.role).toBe("Engineer");
  });
});

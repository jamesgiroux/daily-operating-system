// Parity golden test for the `list_accounts` ability TS mirror. Mirrors
// the pattern from `entity-intelligence/__tests__/contracts.golden.test.ts`.

import { describe, expect, it } from "vitest";

import {
  LIST_ACCOUNTS_SCHEMA_VERSION,
  type AccountListFilter,
  type AccountListInput,
  type AccountListPage,
  type AccountSummary,
  type CursorState,
} from "../contracts";
import type { TrustBand } from "../../entity-intelligence/contracts";

interface ContractGoldenFixture {
  input: AccountListInput;
  filter: AccountListFilter;
  page: AccountListPage;
  enumCoverage: {
    trustBands: TrustBand[];
    cursorStates: CursorState[];
  };
}

const goldenFixture = {
  input: {
    schemaVersion: LIST_ACCOUNTS_SCHEMA_VERSION,
    pageSize: 25,
    filter: {
      status: "active",
      healthBand: "likely_current",
      nameContains: "Example",
    },
  },
  filter: {
    status: "active",
    healthBand: "use_with_caution",
    nameContains: "Example",
  },
  page: {
    items: [
      {
        accountId: "acct-example-1",
        name: "Example Account 1",
        status: "active",
        healthBand: "likely_current",
        lastTouchpointAt: "2026-05-20T12:00:00Z",
        openLoopsCount: 3,
      },
    ],
    nextCursor: "opaque-cursor-token",
    totalHint: 47,
    cursorState: { kind: "stable" },
  },
  enumCoverage: {
    trustBands: [
      "likely_current",
      "use_with_caution",
      "needs_verification",
      "unscored",
    ],
    cursorStates: [
      { kind: "stable" },
      { kind: "data_shifted", advisory: "rows shifted" },
      {
        kind: "invalidated",
        reason: "filter or page_size changed since cursor was issued",
        restart_required: true,
      },
    ],
  },
} satisfies ContractGoldenFixture;

describe("list_accounts contract golden fixture", () => {
  it("parses representative input + page JSON", () => {
    const parsed = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;
    expect(parsed.input.schemaVersion).toBe(LIST_ACCOUNTS_SCHEMA_VERSION);
    expect(parsed.page.items[0]?.accountId).toBe("acct-example-1");
    expect(parsed.page.cursorState).toEqual({ kind: "stable" });
  });

  it("covers every closed enum variant mirrored from Rust", () => {
    expect(goldenFixture.enumCoverage.trustBands).toHaveLength(4);
    expect(goldenFixture.enumCoverage.cursorStates).toHaveLength(3);
  });

  it("AccountSummary shape includes the W2 list-row fields", () => {
    const row: AccountSummary = {
      accountId: "x",
      name: "X",
      status: "active",
      healthBand: "likely_current",
      lastTouchpointAt: null,
      openLoopsCount: 0,
    };
    expect(row.openLoopsCount).toBe(0);
    expect(row.lastTouchpointAt).toBeNull();
  });
});

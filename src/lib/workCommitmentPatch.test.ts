import { describe, expect, it } from "vitest";

import { workCommitmentOwnerPatch } from "./workCommitmentPatch";

describe("workCommitmentOwnerPatch", () => {
  it("omits ownerRaw when clearing an existing Work-card owner", () => {
    expect(workCommitmentOwnerPatch("   ")).toEqual({ clearOwner: true });
  });

  it("trims non-empty owner edits", () => {
    expect(workCommitmentOwnerPatch("  Alex Chen  ")).toEqual({ ownerRaw: "Alex Chen" });
  });
});

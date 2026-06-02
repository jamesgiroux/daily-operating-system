import { describe, expect, it } from "vitest";
import { archiveResultNeedsAttention, type ArchiveEntityCommandResult } from "./archive-command-result";

function result(
  status: ArchiveEntityCommandResult["status"],
  folderStatus = "succeeded",
): ArchiveEntityCommandResult {
  return {
    status,
    changedIds: ["entity-1"],
    childrenRestored: 0,
    itemResults: [{ folderStatus }],
  };
}

describe("archiveResultNeedsAttention", () => {
  it("flags partial single-entity archive command results", () => {
    expect(archiveResultNeedsAttention(result("partial", "succeeded"))).toBe(true);
  });

  it("flags folder statuses that need repair even when status is not partial", () => {
    expect(archiveResultNeedsAttention(result("succeeded", "missing_source"))).toBe(true);
  });

  it("does not flag clean archive command results", () => {
    expect(archiveResultNeedsAttention(result("succeeded"))).toBe(false);
  });
});

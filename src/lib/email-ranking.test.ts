import { describe, expect, it } from "vitest";
import { compareEmailRank } from "./email-ranking";
import type { Email, EmailPriority } from "@/types";

/** Minimal email fixture for ranking tests. */
function makeEmail(overrides: Partial<Email> & { id: string }): Email {
  return {
    sender: "Test Sender",
    senderEmail: "test@example.com",
    subject: "Test Subject",
    priority: "medium" as EmailPriority,
    commitments: [],
    questions: [],
    trackedCommitments: [],
    ...overrides,
  };
}

describe("compareEmailRank", () => {
  it("pinned emails sort before unpinned", () => {
    const a = makeEmail({ id: "1", pinnedAt: "2026-03-24T00:00:00Z" });
    const b = makeEmail({ id: "2" });
    expect(compareEmailRank(a, b)).toBeLessThan(0);
  });

  it("higher relevance score sorts first", () => {
    const a = makeEmail({ id: "1", relevanceScore: 0.8 });
    const b = makeEmail({ id: "2", relevanceScore: 0.3 });
    expect(compareEmailRank(a, b)).toBeLessThan(0);
  });
});

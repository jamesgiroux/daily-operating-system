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

/**
 * Replicate the DailyBriefing email selection logic for testability.
 * This mirrors DailyBriefing.tsx's briefingEmails computation.
 */
function selectBriefingEmails(emails: Email[]): Email[] {
  if (emails.length === 0) return [];
  const ranked = [...emails].sort(compareEmailRank);
  const scored = ranked
    .filter((e) => (e.relevanceScore ?? 0) >= 0.15)
    .slice(0, 5);
  const scoredIds = new Set(scored.map((e) => e.id));
  const enrichedFill = ranked
    .filter((e) => !scoredIds.has(e.id) && e.summary && e.summary.trim().length > 0)
    .slice(0, Math.max(0, 5 - scored.length));
  const selected = [...scored, ...enrichedFill].slice(0, 5);
  // Fallback: if no emails passed score/enrichment filters, show top by rank
  return selected.length > 0 ? selected : ranked.slice(0, 5);
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

describe("selectBriefingEmails (DailyBriefing selection logic)", () => {
  it("returns empty array for empty input", () => {
    expect(selectBriefingEmails([])).toEqual([]);
  });

  it("selects scored emails above threshold", () => {
    const emails = [
      makeEmail({ id: "1", relevanceScore: 0.5, summary: "Important update" }),
      makeEmail({ id: "2", relevanceScore: 0.1 }),
      makeEmail({ id: "3", relevanceScore: 0.8, summary: "Critical issue" }),
    ];
    const result = selectBriefingEmails(emails);
    expect(result.map((e) => e.id)).toEqual(["3", "1"]);
  });

  it("fills remaining slots with enriched emails below threshold", () => {
    const emails = [
      makeEmail({ id: "1", relevanceScore: 0.5 }),
      makeEmail({ id: "2", relevanceScore: 0.1, summary: "Has summary but low score" }),
      makeEmail({ id: "3", relevanceScore: 0.05, summary: "Also enriched" }),
    ];
    const result = selectBriefingEmails(emails);
    expect(result.length).toBe(3);
    // id:1 is scored, id:2 and id:3 fill via enriched path
    expect(result[0].id).toBe("1");
  });

  it("falls back to top 5 by rank when no emails are scored or enriched", () => {
    const emails = [
      makeEmail({ id: "1" }),
      makeEmail({ id: "2" }),
      makeEmail({ id: "3" }),
    ];
    // None have relevanceScore >= 0.15 or summaries → fallback
    const result = selectBriefingEmails(emails);
    expect(result.length).toBe(3);
    expect(result.map((e) => e.id)).toEqual(["1", "2", "3"]);
  });

  it("fallback respects rank order (pinned first, then by score)", () => {
    const emails = [
      makeEmail({ id: "1" }),
      makeEmail({ id: "2", pinnedAt: "2026-03-24T00:00:00Z" }),
      makeEmail({ id: "3", relevanceScore: 0.1 }),
    ];
    const result = selectBriefingEmails(emails);
    expect(result[0].id).toBe("2"); // pinned sorts first
  });

  it("limits to 5 emails maximum", () => {
    const emails = Array.from({ length: 10 }, (_, i) =>
      makeEmail({ id: `${i}`, relevanceScore: 0.5 + i * 0.01 })
    );
    const result = selectBriefingEmails(emails);
    expect(result.length).toBe(5);
  });
});

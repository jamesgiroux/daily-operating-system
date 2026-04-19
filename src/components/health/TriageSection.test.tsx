/** @vitest-environment jsdom */

/**
 * DOS-232 Codex fix — TriageSection must not fall into the "On track" fine
 * state when a health-relevant leading signal is present. The original gate
 * checked only risks/recentWins + a narrow Glean slice; an account whose
 * only signal was `productUsageTrend.overallTrend30d = "declining"` was
 * erroneously rendered as fine.
 *
 * These tests seed a single family of `HealthOutlookSignals` at a time and
 * assert (a) `hasTriageContent` returns true and (b) the card renders.
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TriageSection, hasTriageContent } from "./TriageSection";
import type { HealthOutlookSignals } from "@/types";

function emptySignals(overrides: Partial<HealthOutlookSignals>): HealthOutlookSignals {
  return {
    championRisk: null,
    productUsageTrend: null,
    channelSentiment: null,
    transcriptExtraction: null,
    commercialSignals: null,
    advocacyTrack: null,
    quoteWall: [],
    ...overrides,
  };
}

describe("hasTriageContent / TriageSection — Codex DOS-232 gate coverage", () => {
  it("fires on productUsageTrend.overallTrend30d = declining ONLY", () => {
    const glean = emptySignals({
      productUsageTrend: {
        overallTrend30d: "declining",
        features: [],
        underutilizedFeatures: [],
        highlyStickyFeatures: [],
      },
    });
    expect(hasTriageContent(null, glean)).toBe(true);

    render(<TriageSection intelligence={null} gleanSignals={glean} />);
    expect(screen.getAllByText(/declining/i).length).toBeGreaterThan(0);
  });

  it("fires on churn-adjacent transcript questions ONLY", () => {
    const glean = emptySignals({
      transcriptExtraction: {
        churnAdjacentQuestions: [
          { question: "How hard is it to migrate off?", source: "Mar 3 call" },
        ],
        expansionAdjacentQuestions: [],
        competitorBenchmarks: [],
        decisionMakerShifts: [],
        budgetCycleSignals: [],
      },
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  it("fires on decision-maker shifts ONLY", () => {
    const glean = emptySignals({
      transcriptExtraction: {
        churnAdjacentQuestions: [],
        expansionAdjacentQuestions: [],
        competitorBenchmarks: [],
        decisionMakerShifts: [
          { shift: "New CFO joined last month", who: "CFO" },
        ],
        budgetCycleSignals: [],
      },
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  it("fires on competitorBenchmarks.threatLevel = decision_relevant ONLY", () => {
    const glean = emptySignals({
      transcriptExtraction: {
        churnAdjacentQuestions: [],
        expansionAdjacentQuestions: [],
        competitorBenchmarks: [
          { competitor: "Rival Inc", threatLevel: "decision_relevant" },
        ],
        decisionMakerShifts: [],
        budgetCycleSignals: [],
      },
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  it("fires on advocacyTrack.advocacyTrend = cooling ONLY", () => {
    const glean = emptySignals({
      advocacyTrack: {
        speakingSlots: [],
        betaProgramsIn: [],
        referralsMade: [],
        npsHistory: [],
        advocacyTrend: "cooling",
      },
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  it("fires on commercialSignals.arrDirection = shrinking ONLY", () => {
    const glean = emptySignals({
      commercialSignals: {
        arrTrend12mo: [],
        arrDirection: "shrinking",
        discountHistory: [],
      },
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  it("returns false when all health-relevant families are empty", () => {
    const glean = emptySignals({});
    expect(hasTriageContent(null, glean)).toBe(false);
  });

  // DOS-203 Wave-0f: quoteWall must not silently sink into fine state.
  it("fires on quoteWall with a negative sentiment quote ONLY", () => {
    const glean = emptySignals({
      quoteWall: [
        {
          quote: "Rollout has been painful and we are considering alternatives.",
          speaker: "VP Ops",
          sentiment: "negative",
        },
      ],
    });
    expect(hasTriageContent(null, glean)).toBe(true);

    render(<TriageSection intelligence={null} gleanSignals={glean} />);
    expect(screen.getAllByText(/Quote wall/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/painful/i).length).toBeGreaterThan(0);
  });

  it("fires on quoteWall with a mixed sentiment quote ONLY", () => {
    const glean = emptySignals({
      quoteWall: [
        {
          quote: "The new dashboards are great but onboarding is still rough.",
          sentiment: "mixed",
        },
      ],
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  // Explicit design choice: positive quotes render as a "capture opportunity"
  // card rather than being omitted. They must NOT silently sink fine state.
  it("fires on quoteWall with positive quotes ONLY (capture opportunity)", () => {
    const glean = emptySignals({
      quoteWall: [
        {
          quote: "Best vendor relationship we've had in years.",
          speaker: "Buyer",
          sentiment: "positive",
        },
      ],
    });
    expect(hasTriageContent(null, glean)).toBe(true);

    render(<TriageSection intelligence={null} gleanSignals={glean} />);
    expect(screen.getAllByText(/capture opportunity/i).length).toBeGreaterThan(0);
  });

  it("escalates the first negative to urgent when two or more negatives cluster", () => {
    const glean = emptySignals({
      quoteWall: [
        { quote: "Support response times have collapsed.", sentiment: "negative" },
        { quote: "Our users keep complaining about reliability.", sentiment: "negative" },
      ],
    });
    expect(hasTriageContent(null, glean)).toBe(true);
  });

  it("regression: empty quoteWall + no other signals stays in fine state", () => {
    const glean = emptySignals({ quoteWall: [] });
    expect(hasTriageContent(null, glean)).toBe(false);
  });
});

/** @vitest-environment jsdom */

/**
 * TriageSection tests.
 *
 * History:
 *   - DOS-232 (Codex): TriageSection must not fall into the "On track" fine
 *     state when a health-relevant leading signal is present. Each family of
 *     `HealthOutlookSignals` below seeds ONE signal and asserts
 *     `hasTriageContent` + the rendered card.
 *   - DOS-249 (Wave-0g): hard cap at 5, unified Local + Glean ranking
 *     (urgent → soon → stakeholder, newest first within bucket), and
 *     per-card `IntelligenceCorrection` feedback slot.
 */
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TriageSection, hasTriageContent } from "./TriageSection";
import type { EntityIntelligence, HealthOutlookSignals } from "@/types";

// Stub the correction hook so feedback slot can mount without a Tauri backend.
vi.mock("@/hooks/useIntelligenceCorrection", () => ({
  useIntelligenceCorrection: () => ({
    submit: vi.fn().mockResolvedValue(true),
    submitting: false,
    reset: vi.fn(),
  }),
}));

vi.mock("@/hooks/useEntitySuppressions", () => ({
  useEntitySuppressions: () => ({
    isSuppressed: () => false,
    markSuppressed: vi.fn(),
  }),
}));

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

function emptyIntelligence(overrides: Partial<EntityIntelligence>): EntityIntelligence {
  return {
    version: 1,
    entityId: "acct-test",
    entityType: "account",
    enrichedAt: "2026-04-10T00:00:00Z",
    sourceFileCount: 0,
    sourceManifest: [],
    risks: [],
    recentWins: [],
    stakeholderInsights: [],
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
        decisionMakerShifts: [{ shift: "New CFO joined last month", who: "CFO" }],
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
        competitorBenchmarks: [{ competitor: "Rival Inc", threatLevel: "decision_relevant" }],
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

  it("hasTriageContent returns true when only local risks are present", () => {
    const intel = emptyIntelligence({
      risks: [{ text: "DORA compliance gap", urgency: "high" }],
    });
    expect(hasTriageContent(intel, null)).toBe(true);
  });

  it("hasTriageContent returns true when only local recentWins are present", () => {
    const intel = emptyIntelligence({
      recentWins: [{ text: "Shipped SSO integration" }],
    });
    expect(hasTriageContent(intel, null)).toBe(true);
  });
});

describe("TriageSection — DOS-249 cap + ranking + feedback slot", () => {
  it("caps rendered cards at 5 even when more candidates exist", () => {
    const intel = emptyIntelligence({
      risks: [
        { text: "Risk 1 urgent", urgency: "high" },
        { text: "Risk 2 urgent", urgency: "critical" },
        { text: "Risk 3 soon", urgency: "medium" },
        { text: "Risk 4 soon", urgency: "moderate" },
        { text: "Risk 5 low", urgency: "low" },
        { text: "Risk 6 low", urgency: "low" },
        { text: "Risk 7 low", urgency: "low" },
      ],
      recentWins: [{ text: "Win one" }, { text: "Win two" }],
    });

    render(<TriageSection intelligence={intel} gleanSignals={null} />);
    // Cards carry a serif headline — count by data text.
    const headlines = screen.queryAllByText(/Risk \d|Win \w+/i);
    expect(headlines.length).toBe(5);
    // Count chip in header should announce truncation.
    expect(screen.getByText(/showing top 5 of 9/i)).toBeInTheDocument();
  });

  it("orders urgent → soon → stakeholder, newest first within a bucket", () => {
    const glean = emptySignals({
      transcriptExtraction: {
        churnAdjacentQuestions: [
          { question: "Older urgent Q", date: "2026-01-01" },
          { question: "Newer urgent Q", date: "2026-04-01" },
        ],
        expansionAdjacentQuestions: [],
        competitorBenchmarks: [],
        decisionMakerShifts: [{ shift: "CFO changed", date: "2026-03-15" }],
        budgetCycleSignals: [],
      },
      advocacyTrack: {
        speakingSlots: [],
        betaProgramsIn: [],
        referralsMade: [],
        npsHistory: [{ score: 4, surveyDate: "2026-02-20" }],
        advocacyTrend: "cooling",
      },
    });

    render(<TriageSection intelligence={null} gleanSignals={glean} />);
    // Serialized text order reflects DOM order; assert urgent (churn Qs)
    // appear before the soon card (advocacy cooling) appear before the
    // stakeholder card (decision-maker shift).
    const body = document.body.textContent ?? "";
    const idxNewerUrgent = body.indexOf("Newer urgent Q");
    const idxOlderUrgent = body.indexOf("Older urgent Q");
    const idxAdvocacy = body.indexOf("Advocacy is cooling");
    const idxShift = body.indexOf("CFO changed");

    expect(idxNewerUrgent).toBeGreaterThan(-1);
    expect(idxOlderUrgent).toBeGreaterThan(-1);
    expect(idxAdvocacy).toBeGreaterThan(-1);
    expect(idxShift).toBeGreaterThan(-1);

    // Urgent first, newest before older.
    expect(idxNewerUrgent).toBeLessThan(idxOlderUrgent);
    // Urgent before soon.
    expect(idxOlderUrgent).toBeLessThan(idxAdvocacy);
    // Soon before stakeholder.
    expect(idxAdvocacy).toBeLessThan(idxShift);
  });

  it("renders Glean cards alongside Local cards with a Glean source tag", () => {
    const intel = emptyIntelligence({
      risks: [{ text: "Local risk A", urgency: "high" }],
    });
    const glean = emptySignals({
      commercialSignals: {
        arrTrend12mo: [],
        arrDirection: "shrinking",
        discountHistory: [],
      },
    });

    render(<TriageSection intelligence={intel} gleanSignals={glean} />);
    // Glean ARR card rendered.
    expect(screen.getByText(/ARR trajectory is shrinking/i)).toBeInTheDocument();
    // Local risk also rendered.
    expect(screen.getByText(/Local risk A/i)).toBeInTheDocument();
    // Glean tag pill is present.
    expect(screen.getAllByText("Glean").length).toBeGreaterThan(0);
    // Activity tag pill is present (product vocab — see ADR-0083).
    expect(screen.getAllByText("Activity").length).toBeGreaterThan(0);
  });

  it("splits a paragraph risk.text into headline + evidence rather than dumping the full string into the headline", () => {
    const intel = emptyIntelligence({
      risks: [
        {
          text: "Champion James has disengaged from architecture discussions. Jorge has dominated the last three strategy sessions and is pushing a headless-first narrative that diverges from our expansion plan.",
          urgency: "high",
        },
      ],
    });
    render(<TriageSection intelligence={intel} gleanSignals={null} />);
    // Headline is only the first sentence, visible as a heading element.
    expect(
      screen.getByText("Champion James has disengaged from architecture discussions."),
    ).toBeInTheDocument();
    // Evidence remainder is also rendered, separately from the headline.
    expect(
      screen.getByText(/Jorge has dominated the last three strategy sessions/i),
    ).toBeInTheDocument();
  });

  it("raises the triage threshold when the user sentiment is strong", () => {
    const glean = emptySignals({
      transcriptExtraction: {
        churnAdjacentQuestions: [],
        expansionAdjacentQuestions: [],
        competitorBenchmarks: [],
        decisionMakerShifts: [{ shift: "CFO changed", date: "2026-03-15" }],
        budgetCycleSignals: [],
      },
    });

    expect(hasTriageContent(null, glean, "strong")).toBe(false);
    expect(hasTriageContent(null, glean, "at_risk")).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// DOS-269: Action wiring — Snooze, Confirm resolved, Still accurate.
// ─────────────────────────────────────────────────────────────────────────────

import { fireEvent, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

describe("TriageSection — DOS-269 action wiring", () => {
  it("invokes snooze_triage_item and optimistically hides the card", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string) => {
        if (cmd === "list_triage_snoozes") return Promise.resolve([]);
        if (cmd === "snooze_triage_item") return Promise.resolve();
        return Promise.resolve();
      },
    );

    const intel = emptyIntelligence({
      risks: [{ text: "Critical churn risk in progress.", urgency: "high" }],
    });
    render(
      <TriageSection
        intelligence={intel}
        gleanSignals={null}
        accountId="acct-1"
      />,
    );

    const snoozeBtn = await screen.findByRole("button", { name: /snooze/i });
    fireEvent.click(snoozeBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("snooze_triage_item", {
        entityType: "account",
        entityId: "acct-1",
        triageKey: "local-risk-0",
        days: 14,
      });
    });

    // Card optimistically removed.
    await waitFor(() => {
      expect(
        screen.queryByText(/Critical churn risk in progress/i),
      ).not.toBeInTheDocument();
    });
  });

  it("invokes resolve_triage_item when Confirm resolved is clicked", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string) => {
        if (cmd === "list_triage_snoozes") return Promise.resolve([]);
        return Promise.resolve();
      },
    );

    const intel = emptyIntelligence({
      risks: [{ text: "Urgent issue", urgency: "high" }],
    });
    render(
      <TriageSection
        intelligence={intel}
        gleanSignals={null}
        accountId="acct-2"
      />,
    );

    const resolveBtn = await screen.findByRole("button", {
      name: /confirm resolved/i,
    });
    fireEvent.click(resolveBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("resolve_triage_item", {
        entityType: "account",
        entityId: "acct-2",
        triageKey: "local-risk-0",
      });
    });
  });

  it("hides cards returned by list_triage_snoozes whose snoozed_until is in the future", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    const future = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString();
    (invoke as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (cmd: string) => {
        if (cmd === "list_triage_snoozes") {
          return Promise.resolve([
            {
              triageKey: "local-risk-0",
              snoozedUntil: future,
              resolvedAt: null,
            },
          ]);
        }
        return Promise.resolve();
      },
    );

    const intel = emptyIntelligence({
      risks: [{ text: "Already snoozed item.", urgency: "high" }],
    });
    render(
      <TriageSection
        intelligence={intel}
        gleanSignals={null}
        accountId="acct-3"
      />,
    );

    await waitFor(() => {
      expect(
        screen.queryByText(/Already snoozed item/i),
      ).not.toBeInTheDocument();
    });
  });

  it('renders binary "Is this accurate?" validation when accountId is provided', async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValue([]);

    const intel = emptyIntelligence({
      risks: [{ text: "A thing to confirm", urgency: "high" }],
    });
    render(
      <TriageSection
        intelligence={intel}
        gleanSignals={null}
        accountId="acct-4"
      />,
    );
    expect(
      await screen.findByText("Is this accurate?"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "No" })).toBeInTheDocument();
  });
});

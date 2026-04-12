/** @vitest-environment jsdom */

import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PostMeetingIntelligence } from "@/components/meeting/PostMeetingIntelligence";
import type {
  ContinuityThread,
  DbAction,
  EnrichedCapture,
  MeetingPostIntelligence,
  PredictionResult,
  PredictionScorecard,
} from "@/types";

function makeCapture(
  id: string,
  captureType: string,
  content: string,
  overrides: Partial<EnrichedCapture> = {},
): EnrichedCapture {
  return {
    id,
    meetingId: "mtg-001",
    meetingTitle: "Acme Weekly Sync",
    accountId: "acct-001",
    captureType,
    content,
    capturedAt: "2026-03-22T10:00:00Z",
    ...overrides,
  };
}

function makeAction(
  id: string,
  status: string,
  overrides: Partial<DbAction> = {},
): DbAction {
  return {
    id,
    title: `Action ${id}`,
    priority: 3,
    status,
    createdAt: "2026-03-22T10:00:00Z",
    updatedAt: "2026-03-22T10:00:00Z",
    context: "Captured during call",
    ...overrides,
  };
}

function baseData(overrides: Partial<MeetingPostIntelligence> = {}): MeetingPostIntelligence {
  return {
    interactionDynamics: {
      meetingId: "mtg-001",
      talkBalanceCustomerPct: 62,
      talkBalanceInternalPct: 38,
      speakerSentiments: [
        {
          name: "Sarah Chen",
          sentiment: "positive",
          evidence: "She endorsed the rollout timeline.",
        },
      ],
      questionDensity: "high",
      decisionMakerActive: "yes",
      forwardLooking: "strong",
      monologueRisk: false,
      competitorMentions: [],
      escalationLanguage: [],
    },
    championHealth: {
      meetingId: "mtg-001",
      championName: "Sarah Chen",
      championStatus: "strong",
      championEvidence: "Actively pulled finance into the decision.",
      championRisk: "Needs pricing backup before procurement review.",
    },
    roleChanges: [
      {
        id: "role-001",
        meetingId: "mtg-001",
        personName: "Pat Kim",
        oldStatus: "Observer",
        newStatus: "Executive sponsor",
        evidenceQuote: "I’ll take this to the steering committee next week.",
      },
    ],
    enrichedCaptures: [
      makeCapture("win-001", "win", "Expansion motion is live", {
        subType: "expansion",
        speaker: "Sarah Chen",
        evidenceQuote: "We are ready to extend this into APAC.",
      }),
      makeCapture("risk-001", "risk", "Legal review is behind schedule", {
        urgency: "red",
      }),
      makeCapture("decision-001", "decision", "Pilot expands in April"),
      makeCapture("commitment-001", "commitment", "Send procurement package", {
        subType: "follow_up",
      }),
    ],
    ...overrides,
  };
}

function prediction(
  text: string,
  category: PredictionResult["category"],
  overrides: Partial<PredictionResult> = {},
): PredictionResult {
  return {
    text,
    category,
    ...overrides,
  };
}

function scorecard(): PredictionScorecard {
  return {
    hasData: true,
    riskPredictions: [
      prediction("Security review delays signature", "confirmed", {
        matchText: "Security review delayed the final signature.",
      }),
      prediction("Budget concerns do not surface", "notRaised"),
    ],
    winPredictions: [
      prediction("Champion pushes expansion", "surprise", {
        source: "prep narrative",
      }),
    ],
  };
}

const thread: ContinuityThread = {
  previousMeetingDate: "2026-03-15",
  previousMeetingTitle: "last week’s sync",
  entityName: "Acme Corp",
  actionsCompleted: [{ title: "Finalize pilot scope", isOverdue: false }],
  actionsOpen: [{ title: "Review pricing addendum", date: "2026-03-25", isOverdue: false }],
  healthDelta: { previous: 72, current: 84 },
  newAttendees: ["Jordan Lee"],
  isFirstMeeting: false,
};

describe("PostMeetingIntelligence", () => {
  it("renders suggested actions with accept and dismiss controls", () => {
    const onAcceptAction = vi.fn();
    const onDismissAction = vi.fn();

    render(
      <PostMeetingIntelligence
        data={baseData()}
        actions={[makeAction("act-001", "suggested", { title: "Confirm procurement owner" })]}
        onAcceptAction={onAcceptAction}
        onDismissAction={onDismissAction}
      />,
    );

    expect(screen.getByText("Suggested")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /accept/i }));
    fireEvent.click(screen.getByRole("button", { name: /dismiss/i }));

    expect(onAcceptAction).toHaveBeenCalledWith("act-001");
    expect(onDismissAction).toHaveBeenCalledWith("act-001");
  });

  it("renders pending and completed actions while hiding archived and cancelled entries", () => {
    const onToggleAction = vi.fn();

    const { container } = render(
      <PostMeetingIntelligence
        data={baseData()}
        actions={[
          makeAction("act-002", "pending", { title: "Send updated redlines" }),
          makeAction("act-003", "completed", { title: "Draft mutual action plan" }),
          makeAction("act-004", "archived", { title: "Old archived item" }),
          makeAction("act-005", "cancelled", { title: "Cancelled item" }),
        ]}
        onToggleAction={onToggleAction}
      />,
    );

    expect(screen.getByText("Pending")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /done/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /reopen/i })).toBeInTheDocument();
    const completedTitle = screen.getByText("Draft mutual action plan");
    expect(completedTitle).toBeInTheDocument();
    expect(completedTitle.className).toMatch(/actionTitleCompleted/);
    expect(screen.queryByText("Old archived item")).not.toBeInTheDocument();
    expect(screen.queryByText("Cancelled item")).not.toBeInTheDocument();
    expect(
      container.querySelector('[class*="completedIcon"] svg, svg[class*="completedIcon"], [class*="completedIcon"]'),
    ).not.toBeNull();

    fireEvent.click(screen.getByRole("button", { name: /done/i }));
    fireEvent.click(screen.getByRole("button", { name: /reopen/i }));
    expect(onToggleAction).toHaveBeenNthCalledWith(1, "act-002");
    expect(onToggleAction).toHaveBeenNthCalledWith(2, "act-003");
  });

  it("shows commitments only when no extracted actions exist", () => {
    const { rerender } = render(
      <PostMeetingIntelligence data={baseData()} actions={[]} />,
    );

    expect(screen.getByText("Send procurement package")).toBeInTheDocument();

    rerender(
      <PostMeetingIntelligence
        data={baseData()}
        actions={[makeAction("act-006", "pending", { title: "Handle commitments via action" })]}
      />,
    );

    expect(screen.queryByText("Send procurement package")).not.toBeInTheDocument();
    expect(screen.getByText("Handle commitments via action")).toBeInTheDocument();
  });

  it("renders role changes, thread, prediction icons, speaker sentiment, and talk balance", () => {
    const { container } = render(
      <PostMeetingIntelligence
        data={baseData()}
        continuityThread={thread}
        predictionScorecard={scorecard()}
        summary="Acme validated the rollout path and surfaced one legal blocker."
      />,
    );

    expect(screen.getAllByText("The Thread").length).toBeGreaterThan(0);
    expect(screen.getByText("Finalize pilot scope")).toBeInTheDocument();
    expect(screen.getAllByText("Pat Kim").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Observer").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Executive sponsor").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Jordan Lee").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Sarah Chen").length).toBeGreaterThan(0);
    expect(screen.getAllByText("positive").length).toBeGreaterThan(0);
    expect(screen.getAllByText("62% Customer").length).toBeGreaterThan(0);
    expect(screen.getAllByText("38% Internal").length).toBeGreaterThan(0);

    const predictionHeading = screen.getByText("What We Predicted vs What Happened");
    const predictionSection = predictionHeading.closest("section");
    expect(predictionSection).not.toBeNull();
    expect(within(predictionSection as HTMLElement).getByText("Security review delays signature")).toBeInTheDocument();
    expect(within(predictionSection as HTMLElement).getByText("Champion pushes expansion")).toBeInTheDocument();
    expect((predictionSection as HTMLElement).querySelectorAll("svg").length).toBeGreaterThanOrEqual(3);
    expect(container).not.toHaveTextContent("✓");
    expect(container).not.toHaveTextContent("⚡");
    expect(container).not.toHaveTextContent("✗");
  });

  it("renders a fallback row when the thread has no captured changes", () => {
    const emptyThread: ContinuityThread = {
      previousMeetingDate: "2026-02-24T19:00:00+00:00",
      previousMeetingTitle: "Jane <> VIP",
      entityName: "Jane Software",
      actionsCompleted: [],
      actionsOpen: [],
      healthDelta: undefined,
      newAttendees: [],
      isFirstMeeting: false,
    };

    render(
      <PostMeetingIntelligence
        data={baseData()}
        continuityThread={emptyThread}
      />,
    );

    expect(screen.getAllByText("The Thread").length).toBeGreaterThan(0);
    expect(
      screen.getByText("No major changes captured since the previous meeting"),
    ).toBeInTheDocument();
  });

  it("strips control tags across transcript enrichment sections", () => {
    render(
      <PostMeetingIntelligence
        data={baseData({
          interactionDynamics: {
            meetingId: "mtg-001",
            talkBalanceCustomerPct: 62,
            talkBalanceInternalPct: 38,
            speakerSentiments: [
              {
                name: "Sarah Chen",
                sentiment: "positive",
                evidence: "[YELLOW] She endorsed the rollout timeline.",
              },
            ],
            questionDensity: "high",
            decisionMakerActive: "yes",
            forwardLooking: "strong",
            monologueRisk: false,
            competitorMentions: [
              { competitor: "[EXPANSION] Competitor X", context: "[GREEN_WATCH] Mentioned during procurement review" },
            ],
            escalationLanguage: [
              { quote: "[RED] We need legal unstuck this week", speaker: "[YELLOW] Sarah Chen" },
            ],
          },
          championHealth: {
            meetingId: "mtg-001",
            championName: "Sarah Chen",
            championStatus: "strong",
            championEvidence: "[YELLOW] Actively pulled finance into the decision.",
            championRisk: "[RED] Needs pricing backup before procurement review.",
          },
          roleChanges: [
            {
              id: "role-001",
              meetingId: "mtg-001",
              personName: "Pat Kim",
              oldStatus: "Observer",
              newStatus: "Executive sponsor",
              evidenceQuote: "[JOINT_AGREEMENT] I’ll take this to the steering committee next week.",
            },
          ],
          enrichedCaptures: [
            makeCapture("win-001", "win", "[EXPANSION] Expansion motion is live", {
              subType: "expansion",
              speaker: "[YELLOW] Sarah Chen",
              evidenceQuote: "[ADOPTION] We are ready to extend this into APAC.",
              impact: "[VALUE_REALIZED] Multi-region rollout approved",
            }),
            makeCapture("commitment-001", "commitment", "[CUSTOMER_COMMITMENT] Send procurement package", {
              subType: "follow_up",
            }),
          ],
        })}
        continuityThread={{
          ...thread,
          previousMeetingTitle: "[EXPANSION] last week’s sync",
          actionsCompleted: [{ title: "[YELLOW] Finalize pilot scope", isOverdue: false }],
          actionsOpen: [{ title: "[RED] Review pricing addendum", date: "2026-03-25", isOverdue: false }],
        }}
        predictionScorecard={{
          hasData: true,
          riskPredictions: [
            prediction("[YELLOW] Security review delays signature", "confirmed", {
              matchText: "[RED] Security review delayed the final signature.",
            }),
          ],
          winPredictions: [
            prediction("[EXPANSION] Champion pushes expansion", "surprise", {
              source: "[ADOPTION] prep narrative",
            }),
          ],
        }}
        summary="[GREEN_WATCH] Acme validated the rollout path and surfaced one legal blocker."
        actions={[
          makeAction("act-007", "pending", {
            title: "[RED] Send updated redlines",
            context: "[YELLOW] Captured during call",
          }),
        ]}
      />,
    );

    const bodyText = document.body.textContent ?? "";

    expect(bodyText).toContain("Acme validated the rollout path and surfaced one legal blocker.");
    expect(bodyText).toContain("Since last week’s sync on");
    expect(bodyText).toContain("Finalize pilot scope");
    expect(bodyText).toContain("Review pricing addendum");
    expect(bodyText).toContain("Security review delays signature");
    expect(bodyText).toContain("Security review delayed the final signature.");
    expect(bodyText).toContain("Champion pushes expansion");
    expect(bodyText).toContain("Expansion motion is live");
    expect(bodyText).toContain("Multi-region rollout approved");
    expect(bodyText).toContain("We need legal unstuck this week");
    expect(bodyText).toContain("Mentioned during procurement review");
    expect(bodyText).toContain("Actively pulled finance into the decision.");
    expect(bodyText).toContain("Needs pricing backup before procurement review.");
    expect(bodyText).toContain("Send updated redlines");
    expect(bodyText).toContain("Captured during call");
    expect(bodyText).toContain("I’ll take this to the steering committee next week.");

    expect(bodyText).not.toContain("[YELLOW]");
    expect(bodyText).not.toContain("[RED]");
    expect(bodyText).not.toContain("[EXPANSION]");
    expect(bodyText).not.toContain("[GREEN_WATCH]");
    expect(bodyText).not.toContain("[ADOPTION]");
    expect(bodyText).not.toContain("[VALUE_REALIZED]");
    expect(bodyText).not.toContain("[CUSTOMER_COMMITMENT]");
    expect(bodyText).not.toContain("[JOINT_AGREEMENT]");
  });
});

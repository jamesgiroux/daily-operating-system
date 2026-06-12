/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DailyBriefing } from "./DailyBriefing";
import type { DashboardData, DataFreshness, Meeting } from "@/types";
import type { UseDailyBriefingAbilityResult } from "@/hooks/useDailyBriefingAbility";
import type { ProjectedBlock, ProjectedComposition, ProjectedCompositionCommandResponse } from "@/services/composition/contracts";

// ── Mocks ──────────────────────────────────────────────────────────────────────

const invokeMock = vi.fn();
const calendarMock = vi.hoisted(() => ({
  now: new Date("2026-06-12T09:30:00").getTime(),
}));
const projectedCompositionMock = vi.hoisted(() => ({
  refetch: vi.fn(),
  calls: [] as unknown[],
  state: {
    data: null as ProjectedCompositionCommandResponse | null,
    loading: false,
    error: null as string | null,
    renderedProvenance: null,
  },
}));
const dailyBriefingAbilityMock = vi.hoisted((): { state: UseDailyBriefingAbilityResult } => ({
  state: {
    response: null,
    loading: false,
    error: null as string | null,
    refresh: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, ...props }: Record<string, unknown>) => (
    <a href={String(props.to ?? "#")}>{children as React.ReactNode}</a>
  ),
  useNavigate: () => vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn(), warning: vi.fn() },
}));

vi.mock("@/hooks/useCalendar", () => ({
  useCalendar: () => ({
    now: calendarMock.now,
    currentMeeting: null,
  }),
}));

vi.mock("@/hooks/useDailyBriefingAbility", () => ({
  useDailyBriefingAbility: () => dailyBriefingAbilityMock.state,
}));

vi.mock("@/hooks/useProjectedComposition", () => ({
  useProjectedComposition: (subject: unknown) => {
    projectedCompositionMock.calls.push(subject);
    return {
      ...projectedCompositionMock.state,
      refetch: projectedCompositionMock.refetch,
    };
  },
}));

vi.mock("@/hooks/useMagazineShell", () => ({
  useRegisterMagazineShell: vi.fn(),
}));

vi.mock("@/hooks/useSuggestedActions", () => ({
  useSuggestedActions: () => ({
    suggestedActions: [],
    acceptAction: vi.fn(),
    rejectAction: vi.fn(),
  }),
}));

vi.mock("./BriefingMeetingCard", () => ({
  BriefingMeetingCard: ({
    meeting,
    onComplete,
  }: {
    meeting: Meeting;
    onComplete?: (id: string) => void;
  }) => (
    <div data-testid="meeting-card">
      <span>{meeting.title}</span>
      <button type="button" onClick={() => onComplete?.("action-1")}>
        Complete action
      </button>
    </div>
  ),
  getTemporalState: () => "future",
}));

vi.mock("@/components/ui/folio-refresh-button", () => ({
  FolioRefreshButton: () => <button data-testid="refresh-btn">Refresh</button>,
}));

vi.mock("@/components/editorial/FinisMarker", () => ({
  FinisMarker: () => <div data-testid="finis-marker" />,
}));

vi.mock("@/components/shared/SuggestedActionRow", () => ({
  SuggestedActionRow: () => <div data-testid="suggested-action-row" />,
}));

vi.mock("@/components/shared/HealthBadge", () => ({
  HealthBadge: () => <span data-testid="health-badge" />,
}));

vi.mock("@/components/ui/email-entity-chip", () => ({
  EmailEntityChip: () => <span data-testid="email-entity-chip" />,
}));

// ── Test Data ──────────────────────────────────────────────────────────────────

function makeMeeting(overrides: Partial<Meeting> = {}): Meeting {
  return {
    id: "mtg-1",
    title: "Acme QBR",
    time: "2:00 PM",
    type: "customer",
    hasPrep: true,
    ...overrides,
  };
}

function makeDashboardData(overrides: Partial<DashboardData> = {}): DashboardData {
  return {
    overview: {
      greeting: "Good morning",
      date: "Monday, March 31, 2026",
      summary: "Three meetings today, one QBR.",
      focus: "Focus on Acme renewal prep.",
    },
    stats: {
      totalMeetings: 3,
      customerMeetings: 1,
      actionsDue: 2,
      inboxCount: 5,
    },
    meetings: [],
    actions: [],
    ...overrides,
  };
}

const freshness: DataFreshness = {
  freshness: "fresh",
  generatedAt: "2026-03-31T08:00:00Z",
};

function projectedBlock(overrides: Partial<ProjectedBlock>): ProjectedBlock {
  return {
    block_id: "block-fixture",
    block_index: 0,
    original_type_id: "claim_summary",
    selected_known_type_id: "claim_summary",
    payload: {},
    banner: null,
    trust_band: "likely_current",
    claim_refs: [],
    provenance: [],
    edit_routes: [],
    diagnostics: [],
    ...overrides,
  };
}

function projectedResponse(projection: ProjectedComposition): ProjectedCompositionCommandResponse {
  return {
    ok: true,
    request_id: "projection-request",
    cache_hint_token: "cache-token",
    served_from_cache: false,
    projection,
    rendered_provenance: { surface: "tauri_app", value: { ok: true } },
  };
}

function dSpineProjection(): ProjectedComposition {
  const lead = projectedBlock({
    block_id: "lead",
    block_index: 0,
    payload: { text: "Two meetings today. The renewal is the one to nail." },
  });
  const readiness = projectedBlock({
    block_id: "readiness",
    block_index: 1,
    payload: {
      availability_label: "Available",
      freshness_label: "Fresh",
      integrity_label: "Clean",
    },
  });
  const schedule = projectedBlock({
    block_id: "schedule",
    block_index: 2,
    payload: {
      items: [
        {
          text: "Customer renewal",
          status: "briefing_fresh",
          status_label: "Briefing fresh",
          label: "Current",
          starts_at: "2026-06-12T10:00:00",
          ends_at: "2026-06-12T10:45:00",
          linked_entity_type: "account",
          linked_entity_id: "acct-example",
          linked_entity_name: "Example Co",
          source_asof: "2026-06-12T08:00:00",
        },
      ],
    },
  });
  const moving = projectedBlock({
    block_id: "moving",
    block_index: 3,
    payload: {
      items: [
        {
          text: "Renewal moved forward",
          body: "Legal review is starting, so the meeting needs clean terms language.",
          severity: "high",
          entity_name: "Example Co",
          entity_type: "account",
          entity_id: "acct-example",
          source_asof: "2026-06-12T08:05:00",
          claim_id: "claim-moving",
        },
      ],
    },
    claim_refs: [{ claim_id: "claim-moving", claim_version: 1, field_path: "/items/0/text" }],
    edit_routes: [
      {
        field_path: "/items/0/text",
        role: "feedback_target",
        claim_refs: [{ claim_id: "claim-moving", claim_version: 1, field_path: "/items/0/text" }],
        feedback_allowed: true,
        refusal_reason: null,
      },
    ],
  });
  const watch = projectedBlock({
    block_id: "watch",
    block_index: 4,
    selected_known_type_id: "action_list",
    payload: {
      items: [
        {
          text: "Follow up after legal review",
          status: "blocked",
          status_label: "Parked",
          source_asof: "2026-06-12T08:10:00",
        },
      ],
    },
  });
  return {
    composition_id: "dailyos/daily-briefing:briefing:local~2026-06-12",
    composition_version: 3,
    fallback_policy_version: 1,
    sections: [
      {
        section_id: "lead",
        section_index: 0,
        label: "Lead",
        layout: "stacked",
        salience: { weight: 1, band: "critical", reason: "fixture" },
        block_ids: ["lead", "readiness"],
        block_indexes: [0, 1],
      },
      {
        section_id: "schedule",
        section_index: 1,
        label: "Today",
        layout: "stacked",
        salience: { weight: 1, band: "critical", reason: "fixture" },
        block_ids: ["schedule"],
        block_indexes: [2],
      },
      {
        section_id: "moving",
        section_index: 2,
        label: "Moving",
        layout: "stacked",
        salience: { weight: 1, band: "important", reason: "fixture" },
        block_ids: ["moving"],
        block_indexes: [3],
      },
      {
        section_id: "watch",
        section_index: 3,
        label: "Watch",
        layout: "stacked",
        salience: { weight: 1, band: "contextual", reason: "fixture" },
        block_ids: ["watch"],
        block_indexes: [4],
      },
    ],
    blocks: [lead, readiness, schedule, moving, watch],
    diagnostics: [],
    unknown_block_count: 0,
    unknown_block_cap: 5,
    dropped_unknown_block_count: 0,
  };
}

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("DailyBriefing", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    projectedCompositionMock.refetch.mockReset();
    projectedCompositionMock.calls = [];
    projectedCompositionMock.state = {
      data: null,
      loading: false,
      error: null,
      renderedProvenance: null,
    };
    calendarMock.now = new Date("2026-06-12T09:30:00").getTime();
    dailyBriefingAbilityMock.state = {
      response: null,
      loading: false,
      error: null,
      refresh: vi.fn(),
    };
  });

  it("renders without crashing with minimal data", () => {
    render(
      <DailyBriefing data={makeDashboardData()} freshness={freshness} />,
    );

    expect(screen.getByText("Three meetings today, one QBR.")).toBeInTheDocument();
  });

  it("requests projected briefing for the selected DayStrip date", async () => {
    render(
      <DailyBriefing
        data={makeDashboardData({
          meetings: [makeMeeting({ id: "legacy-mtg", title: "Legacy today meeting", type: "customer" })],
        })}
        freshness={freshness}
      />,
    );

    expect(projectedCompositionMock.calls.at(-1)).toMatchObject({
      entityType: "briefing",
      entityId: "local~2026-06-12",
    });

    fireEvent.click(screen.getByRole("link", { name: "Tomorrow" }));

    await waitFor(() => {
      expect(projectedCompositionMock.calls.at(-1)).toMatchObject({
        entityType: "briefing",
        entityId: "local~2026-06-13",
      });
    });
    expect(screen.getByText("Legacy today meeting")).toBeInTheDocument();
  });

  it("renders the projected D-spine once from schedule, attention, and follow-through payloads", () => {
    projectedCompositionMock.state.data = projectedResponse(dSpineProjection());

    render(
      <DailyBriefing data={makeDashboardData()} freshness={freshness} />,
    );

    expect(screen.getByText("Two meetings today. The renewal is the one to nail.")).toBeInTheDocument();
    expect(screen.getByText("Available")).toBeInTheDocument();
    expect(screen.getByText("Fresh")).toBeInTheDocument();
    expect(screen.getByText("Clean")).toBeInTheDocument();

    expect(screen.getAllByRole("heading", { name: "Today's schedule" })).toHaveLength(1);
    expect(screen.getByLabelText("Shape of the day")).toBeInTheDocument();
    expect(screen.getAllByText("Customer renewal").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Example Co").length).toBeGreaterThan(0);
    expect(screen.getByText("Current")).toBeInTheDocument();
    expect(screen.getByText("Briefing fresh")).toBeInTheDocument();
    expect(screen.queryByText("briefing_fresh")).not.toBeInTheDocument();

    expect(screen.getAllByRole("heading", { name: "What's moving" })).toHaveLength(1);
    expect(screen.getByText("Renewal moved forward")).toBeInTheDocument();
    expect(screen.getByText("Legal review is starting, so the meeting needs clean terms language.")).toBeInTheDocument();
    expect(screen.getByText("Is this accurate?")).toBeInTheDocument();

    expect(screen.getAllByRole("heading", { name: "Watch" })).toHaveLength(1);
    expect(screen.getByText("Follow up after legal review")).toBeInTheDocument();
    expect(screen.getByText("Parked")).toBeInTheDocument();
    expect(screen.queryByText("blocked")).not.toBeInTheDocument();
  });

  it("renders hero headline from overview summary", () => {
    render(
      <DailyBriefing
        data={makeDashboardData({
          overview: {
            greeting: "Good morning",
            date: "Monday, March 31, 2026",
            summary: "Your day is packed. Two renewals need attention.",
          },
        })}
        freshness={freshness}
      />,
    );

    expect(screen.getByText("Your day is packed. Two renewals need attention.")).toBeInTheDocument();
  });

  it("renders clear day message when no meetings and no summary", () => {
    render(
      <DailyBriefing
        data={makeDashboardData({
          overview: {
            greeting: "Good morning",
            date: "Monday, March 31, 2026",
            summary: "",
          },
          meetings: [],
        })}
        freshness={freshness}
      />,
    );

    expect(screen.getByText("A clear day. Nothing needs you.")).toBeInTheDocument();
  });

  it("renders schedule section with meetings", () => {
    const meetings = [
      makeMeeting({ id: "m1", title: "Acme QBR", time: "10:00 AM", type: "customer" }),
      makeMeeting({ id: "m2", title: "Partner Sync", time: "11:00 AM", type: "external" }),
    ];

    render(
      <DailyBriefing
        data={makeDashboardData({ meetings })}
        freshness={freshness}
      />,
    );

    expect(screen.getByText("Schedule")).toBeInTheDocument();
    const meetingCards = screen.getAllByTestId("meeting-card");
    expect(meetingCards.length).toBe(2);
  });

  it("force-refreshes projected briefing after completing an action", async () => {
    invokeMock.mockResolvedValueOnce(null);

    render(
      <DailyBriefing
        data={makeDashboardData({
          meetings: [makeMeeting({ id: "m1", title: "Acme QBR", type: "customer" })],
          actions: [{ id: "action-1", title: "Follow up", status: "unstarted", source: "m1", priority: 3 }],
        })}
        freshness={freshness}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Complete action" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("complete_action", { id: "action-1" });
      expect(projectedCompositionMock.refetch).toHaveBeenCalledWith({ forceRefresh: true });
    });
  });

  it("force-refreshes projected briefing when dashboard data refreshes", async () => {
    const initialData = makeDashboardData();
    const { rerender } = render(
      <DailyBriefing data={initialData} freshness={freshness} />,
    );

    expect(projectedCompositionMock.refetch).not.toHaveBeenCalled();

    rerender(
      <DailyBriefing
        data={makeDashboardData({
          overview: {
            ...initialData.overview,
            summary: "Fresh briefing data has landed.",
          },
        })}
        freshness={{ freshness: "fresh", generatedAt: "2026-03-31T08:05:00Z" }}
      />,
    );

    await waitFor(() => {
      expect(projectedCompositionMock.refetch).toHaveBeenCalledTimes(1);
      expect(projectedCompositionMock.refetch).toHaveBeenCalledWith({ forceRefresh: true });
    });
  });

  it("does not render personal/solo meetings in schedule", () => {
    const meetings = [
      makeMeeting({ id: "m1", title: "Acme QBR", type: "customer" }),
      makeMeeting({ id: "m2", title: "Lunch Block", type: "personal" }),
    ];

    render(
      <DailyBriefing
        data={makeDashboardData({ meetings })}
        freshness={freshness}
      />,
    );

    const meetingCards = screen.getAllByTestId("meeting-card");
    expect(meetingCards.length).toBe(1);
    expect(screen.getByText("Acme QBR")).toBeInTheDocument();
    expect(screen.queryByText("Lunch Block")).not.toBeInTheDocument();
  });

  it("renders focus block when focus text provided", () => {
    render(
      <DailyBriefing
        data={makeDashboardData({
          overview: {
            greeting: "Good morning",
            date: "Monday, March 31, 2026",
            summary: "Your day is ready.",
            focus: "Prepare for the Acme renewal conversation.",
          },
        })}
        freshness={freshness}
      />,
    );

    expect(screen.getByText("Prepare for the Acme renewal conversation.")).toBeInTheDocument();
  });

  it("renders ability-backed briefing trust and provenance state", () => {
    dailyBriefingAbilityMock.state = {
      response: {
        invocation_id: "inv-1",
        ability_name: "get_daily_briefing",
        ability_version: "0.1.0",
        schema_version: 1,
        data: {
          schemaVersion: 1,
          date: "2026-05-23",
          state: {
            availability: { kind: "available" },
            freshness: { kind: "needs_preparation", meetingIds: ["m-1"] },
            integrity: { kind: "clean" },
            advisories: [
              { kind: "unlinked_meetings", meetingIds: ["m-2"] },
            ],
          },
          currentMeeting: null,
          nextMeeting: null,
          upcomingMeetings: {
            items: [],
            nextCursor: null,
            totalHint: 0,
            cursorState: { kind: "stable" },
          },
          candidateSet: {
            windowStart: null,
            windowEnd: null,
            filterDescription: "daily_briefing date=2026-05-23 workspace=/workspace meetings=1",
          },
          watchProposals: [],
          trustSummary: {
            aggregateBand: "use_with_caution",
            likelyCurrentCount: 0,
            useWithCautionCount: 1,
            needsVerificationCount: 0,
          },
          provenance: { sources: [], redactionApplied: false },
          sensitivity: "internal",
          sourceAsofInputs: [],
        },
        rendered_provenance: {
          value: {
            sources: [
              { source_asof: "2026-05-23T12:00:00Z" },
              { source_asof: "2026-05-23T13:00:00Z" },
            ],
          },
        },
      },
      loading: false,
      error: null,
      refresh: vi.fn(),
    };

    render(
      <DailyBriefing data={makeDashboardData()} freshness={freshness} />,
    );

    expect(screen.getByTestId("daily-briefing-ability-strip")).toHaveTextContent("1 briefing need prep");
    expect(screen.getByTestId("daily-briefing-ability-strip")).toHaveTextContent("use with caution");
    expect(screen.getByTestId("daily-briefing-ability-strip")).toHaveTextContent("2 sources");
    expect(screen.getByTestId("daily-briefing-ability-strip")).toHaveTextContent("Link 1 meeting for fuller context");
  });

  it("does not render staleness indicator (removed for v1.1.1)", () => {
    const staleFreshness: DataFreshness = {
      freshness: "stale",
      dataDate: "2026-03-30",
      generatedAt: "2026-03-30T18:00:00Z",
    };

    render(
      <DailyBriefing
        data={makeDashboardData()}
        freshness={staleFreshness}
      />,
    );

    expect(screen.queryByText(/Last updated/)).not.toBeInTheDocument();
  });

  it("renders finis marker at the end", () => {
    render(
      <DailyBriefing data={makeDashboardData()} freshness={freshness} />,
    );

    expect(screen.getByTestId("finis-marker")).toBeInTheDocument();
  });

  it("renders capacity info when focus data present", () => {
    render(
      <DailyBriefing
        data={makeDashboardData({
          focus: {
            availableMinutes: 180,
            deepWorkMinutes: 120,
            meetingMinutes: 300,
            meetingCount: 5,
            prioritizedActions: [],
            topThree: [],
            implications: { achievableCount: 3, totalCount: 5, atRiskCount: 1, summary: "" },
            availableBlocks: [
              { day: "Monday", start: "09:00", end: "10:00", durationMinutes: 60 },
              { day: "Monday", start: "14:00", end: "15:30", durationMinutes: 90 },
            ],
          },
        })}
        freshness={freshness}
      />,
    );

    expect(screen.getByText(/3h available/)).toBeInTheDocument();
    expect(screen.getByText(/2 deep work blocks/)).toBeInTheDocument();
    expect(screen.getByText(/5 meetings/)).toBeInTheDocument();
  });

  it("renders with empty actions and emails", () => {
    const { container } = render(
      <DailyBriefing
        data={makeDashboardData({ actions: [], emails: [] })}
        freshness={freshness}
      />,
    );

    expect(container.querySelector("section")).not.toBeNull();
  });

  it("renders email summary trust and source context", () => {
    render(
      <DailyBriefing
        data={makeDashboardData({
          emails: [
            {
              id: "email-1",
              sender: "Alex",
              senderEmail: "alex@example.com",
              subject: "Renewal",
              priority: "high",
              entityId: "acct-1",
              entityType: "account",
              entityName: "Example Co",
              summary: "Renewal risk moved because the buyer asked for a new timeline.",
              summaryContextTrustBand: "use_with_caution",
              summaryContextSourceCount: 2,
              relevanceScore: 0.9,
            },
          ],
        })}
        freshness={freshness}
      />,
    );

    expect(screen.getByRole("img", { name: /trust band: use with caution/i })).toBeInTheDocument();
    expect(screen.getByText("claim context · 2 sources")).toBeInTheDocument();
  });
});

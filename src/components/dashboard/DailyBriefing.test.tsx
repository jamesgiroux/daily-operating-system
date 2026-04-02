/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DailyBriefing } from "./DailyBriefing";
import type { DashboardData, DataFreshness, Meeting } from "@/types";

// ── Mocks ──────────────────────────────────────────────────────────────────────

const invokeMock = vi.fn();

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
    now: Date.now(),
    currentMeeting: null,
  }),
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
  BriefingMeetingCard: ({ meeting }: { meeting: Meeting }) => (
    <div data-testid="meeting-card">{meeting.title}</div>
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

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("DailyBriefing", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("renders without crashing with minimal data", () => {
    render(
      <DailyBriefing data={makeDashboardData()} freshness={freshness} />,
    );

    expect(screen.getByText("Three meetings today, one QBR.")).toBeInTheDocument();
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
});

/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DailyBriefingRedesign from "./DailyBriefingRedesign";
import { useBriefingViewModel } from "@/hooks/useBriefingViewModel";
import type {
  BriefingLoadState,
  BriefingViewModel,
  MovingEntityViewModel,
  ScheduleMeeting,
  WatchRowViewModel,
} from "@/types/briefing";

vi.mock("@/hooks/useBriefingViewModel", () => ({
  useBriefingViewModel: vi.fn(),
}));

vi.mock("@/components/layout", async () => {
  const React = await import("react");

  type Crumb = { label: string };
  type FolioProps = {
    publicationLabel?: string;
    dateText?: string;
    breadcrumbs?: Crumb[];
  };

  return {
    AtmosphereLayer: ({ color }: { color?: string }) =>
      React.createElement("div", {
        "data-ds-name": "AtmosphereLayer",
        "data-color": color,
      }),
    FloatingNavIsland: () =>
      React.createElement("nav", { "data-ds-name": "FloatingNavIsland" }),
    FolioBar: ({ publicationLabel, dateText, breadcrumbs }: FolioProps) =>
      React.createElement(
        "header",
        { "data-ds-name": "FolioBar" },
        React.createElement("span", null, publicationLabel),
        React.createElement("time", null, dateText),
        breadcrumbs?.map((crumb) =>
          React.createElement("span", { key: crumb.label }, crumb.label),
        ),
      ),
  };
});

const useBriefingViewModelMock = vi.mocked(useBriefingViewModel);

const baseTrust = {
  trustBand: "unscored" as const,
};

function mockBriefing(state: BriefingLoadState) {
  const refresh = vi.fn();
  useBriefingViewModelMock.mockReturnValue({
    state,
    refresh,
    isRefreshing: false,
  });
  return refresh;
}

function makeMeeting(partial: Partial<ScheduleMeeting> = {}): ScheduleMeeting {
  return {
    ...baseTrust,
    id: "meeting-1",
    href: "/meeting/meeting-1",
    accentType: "customer",
    state: "upcoming",
    time: {
      startsAtIso: "2026-05-07T13:00:00.000Z",
      endsAtIso: "2026-05-07T13:30:00.000Z",
      startLabel: "9:00 AM",
      durationLabel: "30m",
    },
    stateTags: ["upcoming"],
    title: "Acme renewal prep",
    eyebrow: { entityName: "Acme", relationship: "customer" },
    context: "Bring pricing terms.",
    attendeeSummary: "3 attendees",
    intelligenceQuality: { level: "ready", label: "Ready" },
    briefingAction: {
      kind: "link",
      label: "Open briefing",
      href: "/meeting/meeting-1",
    },
    ...partial,
  };
}

function makeMovingEntity(
  partial: Partial<MovingEntityViewModel> = {},
): MovingEntityViewModel {
  return {
    kind: "customer",
    entity: {
      id: "acct-1",
      name: "Acme Corp",
      entityType: "account",
    },
    href: "/accounts/acct-1",
    statePill: { label: "Moving", tone: "sage" },
    lede: "Renewal activity increased across the account.",
    signals: [
      {
        ...baseTrust,
        kind: "meeting",
        when: "Today",
        whatSegments: [{ text: "Renewal prep on the calendar." }],
        urgency: "normal",
      },
    ],
    provenanceStats: [
      {
        ...baseTrust,
        label: "Signals",
        value: "3",
        trend: "up",
      },
    ],
    ...partial,
  };
}

function makeWatchRow(
  partial: Partial<Extract<WatchRowViewModel, { kind: "openAction" }>> = {},
): Extract<WatchRowViewModel, { kind: "openAction" }> {
  return {
    ...baseTrust,
    kind: "openAction",
    actionId: "action-1",
    who: "Acme Corp",
    what: "Send the revised pricing appendix.",
    checkButtonLabel: "Mark complete",
    ...partial,
  };
}

function makeModel(): BriefingViewModel {
  return {
    date: {
      isoDate: "2026-05-07",
      displayDate: "Thursday, May 7",
    },
    folio: {
      label: "Daily Briefing",
      crumbs: ["DailyOS", "Today"],
      dateLabel: "Thu, May 7",
      readiness: [{ label: "2 ready", semantic: "healthy" }],
      actions: [],
      status: "ready",
    },
    dayStrip: {
      prev: {
        label: "Wed",
        isoDate: "2026-05-06",
        preview: "3 meetings",
        href: "/?date=2026-05-06",
      },
      current: {
        label: "Today",
        isoDate: "2026-05-07",
        ariaLabel: "Thursday, May 7, 2026",
      },
      next: {
        label: "Fri",
        isoDate: "2026-05-08",
        preview: "2 meetings",
        href: "/?date=2026-05-08",
      },
    },
    lead: {
      headline: {
        lead: "Two customer meetings need sharper prep.",
        punchLine: "Acme is the one to nail.",
      },
      focusCapacity: "2h 15m open after lunch",
      focusBlock: "Block pricing review before 11:00.",
    },
    schedule: {
      label: "Schedule",
      heading: "Your day",
      countLabel: "2 meetings",
      meetingMix: {
        customer: 1,
        partner: 0,
        internal: 1,
        personal: 0,
        oneOnOne: 0,
        cancelled: 0,
      },
      summary: "Two meetings, one customer moment.",
      dayChart: {
        rangeStartHour: 8,
        rangeEndHour: 17,
        hourTicks: [],
        legend: [],
        bars: [],
        nowLine: null,
      },
      meetings: [
        makeMeeting(),
        makeMeeting({
          id: "meeting-2",
          title: "Internal launch review",
          accentType: "internal",
          time: {
            startsAtIso: "2026-05-07T16:00:00.000Z",
            endsAtIso: "2026-05-07T17:00:00.000Z",
            startLabel: "12:00 PM",
            durationLabel: "1h",
          },
        }),
      ],
    },
    predictions: {
      label: "Predictions",
      countLabel: "1 today",
      collapsedLabel: "1 prediction today",
      expandHint: "expand",
      count: 1,
      predictions: [
        {
          ...baseTrust,
          id: "prediction-1",
          text: "Acme will ask about renewal timing.",
          confidence: { value: 0.74, label: "74%" },
          abilitySource: { id: "predict", label: "predict" },
          basisLink: { label: "basis", href: "/predictions/prediction-1" },
        },
      ],
    },
    moving: {
      label: "Moving",
      heading: "What is moving",
      countLabel: "2 entities",
      summary: "Two entities changed enough to watch.",
      entities: [
        makeMovingEntity(),
        makeMovingEntity({
          kind: "project",
          entity: {
            id: "project-1",
            name: "Atlas Launch",
            entityType: "project",
          },
          href: "/projects/project-1",
          statePill: { label: "At risk", tone: "terracotta" },
        }),
      ],
    },
    watch: {
      label: "Watch",
      heading: "What to watch",
      countLabel: "2 rows",
      summary: "A compact list of open loops.",
      rows: [
        makeWatchRow(),
        {
          ...baseTrust,
          kind: "parked",
          who: "Atlas Launch",
          what: "Launch deck parked until the customer dates settle.",
          parkedLabel: "Parked",
        },
      ],
    },
  };
}

describe("DailyBriefingRedesign", () => {
  beforeEach(() => {
    useBriefingViewModelMock.mockReset();
  });

  it("renders BriefingLoadingState for the loading branch", () => {
    mockBriefing({ status: "loading" });

    const { container } = render(<DailyBriefingRedesign />);

    expect(useBriefingViewModelMock).toHaveBeenCalledOnce();
    expect(
      container.querySelector('[data-ds-name="BriefingLoadingState"]'),
    ).toBeInTheDocument();
  });

  it("renders BriefingErrorState and wires retry to refresh", () => {
    const refresh = mockBriefing({
      status: "error",
      message: "Could not assemble the briefing.",
      detailMessage: "Calendar is unavailable.",
      code: "dependency_failed",
      service: "schedule",
    });

    const { container } = render(<DailyBriefingRedesign />);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(
      container.querySelector('[data-ds-name="BriefingErrorState"]'),
    ).toBeInTheDocument();
    expect(screen.getByText("Calendar is unavailable.")).toBeInTheDocument();
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("renders BriefingEmptyState with checklist and CTA", () => {
    const refresh = mockBriefing({
      status: "empty",
      message: "Connect your calendar and mail to generate the briefing.",
      checklistItems: [
        { label: "Connect Google Calendar", status: "todo" },
        { label: "Sync recent mail", status: "done" },
      ],
    });

    const { container } = render(<DailyBriefingRedesign />);
    fireEvent.click(screen.getByRole("button", { name: "Check again" }));

    expect(
      container.querySelector('[data-ds-name="BriefingEmptyState"]'),
    ).toBeInTheDocument();
    expect(screen.getByText("Connect Google Calendar")).toBeInTheDocument();
    expect(screen.getByText("Sync recent mail")).toBeInTheDocument();
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("renders the success branch composition", () => {
    const model = makeModel();
    mockBriefing({
      status: "success",
      model,
      freshness: { freshness: "fresh", generatedAt: "2026-05-07T12:00:00Z" },
    });

    const { container } = render(<DailyBriefingRedesign />);

    expect(screen.getByText("Two customer meetings need sharper prep.")).toBeInTheDocument();
    expect(screen.getByText("Acme renewal prep")).toBeInTheDocument();
    expect(screen.getByText("9:00 AM")).toBeInTheDocument();
    expect(screen.getByText("1 prediction today")).toBeInTheDocument();
    expect(container.querySelectorAll('[data-ds-name="MovingRow"]')).toHaveLength(2);
    expect(container.querySelectorAll('[data-ds-name="WatchRow"]')).toHaveLength(2);
  });

  it("links schedule rows to their meeting detail route", () => {
    mockBriefing({
      status: "success",
      model: makeModel(),
      freshness: { freshness: "fresh", generatedAt: "2026-05-07T12:00:00Z" },
    });

    render(<DailyBriefingRedesign />);

    expect(screen.getByRole("link", { name: /9:00 AM\s*Acme renewal prep/i })).toHaveAttribute(
      "href",
      "/meeting/meeting-1",
    );
  });

  it("renders TrustBandBadge for scored schedule meetings", () => {
    const model = makeModel();
    model.schedule.meetings = [
      makeMeeting({ trustBand: "likely_current" }),
      makeMeeting({ id: "meeting-2", trustBand: "unscored" }),
    ];
    mockBriefing({
      status: "success",
      model,
      freshness: { freshness: "fresh", generatedAt: "2026-05-07T12:00:00Z" },
    });

    const { container } = render(<DailyBriefingRedesign />);
    const badges = container.querySelectorAll('[data-ds-name="TrustBandBadge"]');

    expect(badges).toHaveLength(1);
    expect(badges[0]).toHaveAttribute("data-band", "likely_current");
  });

  it("omits TrustBandBadge for unscored schedule meetings", () => {
    mockBriefing({
      status: "success",
      model: makeModel(),
      freshness: { freshness: "fresh", generatedAt: "2026-05-07T12:00:00Z" },
    });

    const { container } = render(<DailyBriefingRedesign />);

    expect(
      container.querySelector('[data-ds-name="TrustBandBadge"]'),
    ).not.toBeInTheDocument();
  });

  it("renders all scored schedule trust variants", () => {
    const model = makeModel();
    model.schedule.meetings = [
      makeMeeting({ id: "likely", trustBand: "likely_current" }),
      makeMeeting({ id: "caution", trustBand: "use_with_caution" }),
      makeMeeting({ id: "verify", trustBand: "needs_verification" }),
      makeMeeting({ id: "unscored", trustBand: "unscored" }),
    ];
    mockBriefing({
      status: "success",
      model,
      freshness: { freshness: "fresh", generatedAt: "2026-05-07T12:00:00Z" },
    });

    const { container } = render(<DailyBriefingRedesign />);
    const bands = Array.from(
      container.querySelectorAll('[data-ds-name="TrustBandBadge"]'),
    ).map((badge) => badge.getAttribute("data-band"));

    expect(bands).toEqual([
      "likely_current",
      "use_with_caution",
      "needs_verification",
    ]);
  });

  it("renders folio crumbs, date label, and day strip on success", () => {
    mockBriefing({
      status: "success",
      model: makeModel(),
      freshness: { freshness: "fresh", generatedAt: "2026-05-07T12:00:00Z" },
    });

    const { container } = render(<DailyBriefingRedesign />);

    expect(screen.getByText("DailyOS")).toBeInTheDocument();
    expect(screen.getAllByText("Today").length).toBeGreaterThan(0);
    expect(screen.getByText("Thu, May 7")).toBeInTheDocument();
    expect(
      container.querySelector('[data-ds-name="DayStrip"]'),
    ).toBeInTheDocument();
  });

  it("emits ds-inspector attributes on the surface root", () => {
    mockBriefing({ status: "loading" });

    const { container } = render(<DailyBriefingRedesign />);
    const root = container.querySelector(
      '[data-ds-name="DailyBriefingRedesign"]',
    );

    expect(root).toHaveAttribute("data-ds-tier", "surface");
    expect(root).toHaveAttribute(
      "data-ds-spec",
      "surfaces/DailyBriefingRedesign.md",
    );
  });
});

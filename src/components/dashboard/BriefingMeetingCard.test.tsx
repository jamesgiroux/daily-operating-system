/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  BriefingMeetingCard,
  parsePrepGridItem,
  partitionLegacyPrepGrid,
} from "./BriefingMeetingCard";
import {
  clearShowAllEvidenceStateForTests,
  renderedProvenanceFrom,
} from "@/lib/trust-band";
import type { Meeting } from "@/types";

const navigateMock = vi.hoisted(() => vi.fn());

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, params, to }: Record<string, unknown>) => {
    const raw = String(to ?? "#");
    const meetingId = (params as { meetingId?: string } | undefined)?.meetingId;
    const href = meetingId ? raw.replace("$meetingId", meetingId) : raw;
    return (
      <a
        href={href}
        onClick={(event) => {
          event.preventDefault();
          navigateMock(href);
        }}
      >
        {children as React.ReactNode}
      </a>
    );
  },
}));

function makeMeeting(prep?: Meeting["prep"], partial: Partial<Meeting> = {}): Meeting {
  return {
    id: "mtg-1",
    title: "Customer Sync",
    time: "9:00 AM",
    endTime: "9:30 AM",
    type: "customer",
    hasPrep: true,
    prep,
    ...partial,
  };
}

beforeEach(() => {
  clearShowAllEvidenceStateForTests();
  navigateMock.mockReset();
});

describe("prep grid helpers", () => {
  it("renders impact tails as badges instead of inline text", () => {
    expect(parsePrepGridItem("Support mitigated a malicious traffic spike — high")).toEqual({
      text: "Support mitigated a malicious traffic spike",
      impact: "high",
    });
  });

  it("does not duplicate the same item in Discuss and Wins", () => {
    const item =
      "Support quickly identified and mitigated a malicious traffic spike for the customer site — high";
    const prep = {
      actions: [item],
      wins: [item],
    };

    const partition = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), false);

    expect(partition.current.some((entry) => entry.section === "discuss")).toBe(false);
    expect(partition.current).toMatchObject([
      {
        section: "wins",
        text: "Support quickly identified and mitigated a malicious traffic spike for the customer site",
        impact: "high",
      },
    ]);
  });

  it("marks use-with-caution evidence as background", () => {
    const prep: NonNullable<Meeting["prep"]> = {
      actions: ["Confirm rollout owner"],
      risks: ["Renewal signal is older than the current quarter"],
      renderedProvenance: {
        value: {
          field_attributions: {
            "/actions/0": { trust_band: "likely_current" as const },
            "/risks/0": { trust_band: "use_with_caution" as const },
          },
        },
      },
    };

    const partition = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), false);

    expect(partition.current).toMatchObject([{ text: "Confirm rollout owner" }]);
    expect(partition.caution).toMatchObject([
      {
        text: "Renewal signal is older than the current quarter",
        trustBand: "use_with_caution",
      },
    ]);
  });

  it("collapses needs-verification evidence until show-all", () => {
    const prep: NonNullable<Meeting["prep"]> = {
      wins: ["Verify whether the expansion pilot is still active"],
      renderedProvenance: {
        value: {
          field_attributions: {
            "/wins/0": { trust_band: "needs_verification" as const },
          },
        },
      },
    };

    const hidden = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), false);
    const shown = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), true);

    expect(hidden.current).toHaveLength(0);
    expect(hidden.revealedNeedsVerification).toHaveLength(0);
    expect(shown.revealedNeedsVerification).toMatchObject([
      {
        text: "Verify whether the expansion pilot is still active",
        trustBand: "needs_verification",
      },
    ]);
  });
});

describe("BriefingMeetingCard", () => {
  it("navigates non-cancelled rows to meeting detail", () => {
    render(<BriefingMeetingCard meeting={makeMeeting()} now={Date.parse("2026-05-07T08:00:00")} />);

    fireEvent.click(screen.getByRole("link"));

    expect(navigateMock).toHaveBeenCalledWith("/meeting/mtg-1");
  });

  it("keeps cancelled rows inert", () => {
    render(
      <BriefingMeetingCard
        meeting={makeMeeting(undefined, { overlayStatus: "cancelled" })}
        now={Date.parse("2026-05-07T08:00:00")}
      />,
    );

    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.getByText(/Cancelled/)).toBeInTheDocument();
  });
});

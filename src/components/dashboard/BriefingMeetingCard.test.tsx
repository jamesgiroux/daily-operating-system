/** @vitest-environment jsdom */

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PrepGrid, parsePrepGridItem } from "./BriefingMeetingCard";
import { clearShowAllEvidenceStateForTests } from "@/lib/trust-band";
import type { Meeting } from "@/types";

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, ...props }: Record<string, unknown>) => (
    <a href={String(props.to ?? "#")}>{children as React.ReactNode}</a>
  ),
  useNavigate: () => vi.fn(),
}));

function makeMeeting(prep: Meeting["prep"]): Meeting {
  return {
    id: "mtg-1",
    title: "Customer Sync",
    time: "9:00 AM",
    type: "customer",
    hasPrep: true,
    prep,
  };
}

beforeEach(() => {
  clearShowAllEvidenceStateForTests();
});

describe("PrepGrid", () => {
  it("renders impact tails as badges instead of inline text", () => {
    expect(parsePrepGridItem("Support mitigated a malicious traffic spike — high")).toEqual({
      text: "Support mitigated a malicious traffic spike",
      impact: "high",
    });
  });

  it("does not duplicate the same item in Discuss and Wins", () => {
    const item =
      "Support quickly identified and mitigated a malicious traffic spike for the customer site — high";

    render(
      <PrepGrid
        meeting={makeMeeting({
          actions: [item],
          wins: [item],
        })}
      />,
    );

    expect(screen.queryByText("Discuss")).not.toBeInTheDocument();
    const winsSection = screen.getByText("Wins").closest("div");
    expect(winsSection).not.toBeNull();
    expect(within(winsSection!.parentElement!).getByText(/Support quickly identified/)).toBeInTheDocument();
    expect(within(winsSection!.parentElement!).getByText("high")).toBeInTheDocument();
    expect(screen.queryByText(/— high/)).not.toBeInTheDocument();
  });

  it("PrepGrid_marks_use_with_caution_without_inline_color_only_state", () => {
    render(
      <PrepGrid
        meeting={makeMeeting({
          actions: ["Confirm rollout owner"],
          risks: ["Renewal signal is older than the current quarter"],
          renderedProvenance: {
            value: {
              field_attributions: {
                "/actions/0": { trust_band: "likely_current" },
                "/risks/0": { trust_band: "use_with_caution" },
              },
            },
          },
        })}
      />,
    );

    expect(screen.getByText("Confirm rollout owner")).toBeVisible();
    const background = screen.getByText("Background").closest("details");
    expect(background).not.toBeNull();
    expect(
      within(background!).getByText("Renewal signal is older than the current quarter"),
    ).toBeInTheDocument();
    expect(
      within(background!).getByRole("img", { name: /Use with caution/ }),
    ).toBeInTheDocument();
  });

  it("PrepGrid_collapses_needs_verification_until_show_all", () => {
    render(
      <PrepGrid
        meeting={makeMeeting({
          wins: ["Verify whether the expansion pilot is still active"],
          renderedProvenance: {
            value: {
              field_attributions: {
                "/wins/0": { trust_band: "needs_verification" },
              },
            },
          },
        })}
      />,
    );

    expect(screen.getByText("No high-confidence current-state evidence.")).toBeVisible();
    expect(screen.queryByText("Verify whether the expansion pilot is still active")).not.toBeInTheDocument();
    const button = screen.getByRole("button", { name: /show all evidence/i });

    fireEvent.click(button);

    expect(screen.getByText("Showing low-confidence evidence")).toBeVisible();
    expect(screen.getByText("Verify whether the expansion pilot is still active")).toBeVisible();
    expect(screen.getByRole("img", { name: /Needs verification/ })).toBeInTheDocument();
  });
});

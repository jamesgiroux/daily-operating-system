/** @vitest-environment jsdom */

import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PrepGrid, parsePrepGridItem } from "./BriefingMeetingCard";
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
});

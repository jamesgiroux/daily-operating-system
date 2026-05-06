/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MeetingSpineItem } from "./MeetingSpineItem";

describe("MeetingSpineItem", () => {
  it("exposes design-system metadata and composed status/prep primitives", () => {
    render(
      <MeetingSpineItem
        time="10:00"
        duration="45m"
        state="in-progress"
        type="customer"
        entityName="Acme Corp - Renewal"
        title="Acme renewal - pricing and tier 3"
        context="Pricing memo is still the blocker."
        attendees="Jen Park, Dan Mitchell, +2"
        prepState="ready"
        briefingUrl="/meeting/acme-renewal"
        threadMarkContext="Acme renewal pricing memo"
      />,
    );

    const item = screen.getByText("Acme renewal - pricing and tier 3").closest("article");
    expect(item).toHaveAttribute("data-ds-name", "MeetingSpineItem");
    expect(item).toHaveAttribute("data-ds-tier", "pattern");
    expect(screen.getByText("Now")).toHaveAttribute("data-ds-name", "MeetingStatusPill");
    expect(screen.getByText("Briefing fresh")).toHaveAttribute("data-ds-name", "Pill");
    expect(screen.getByRole("button", { name: /talk about acme renewal pricing memo/i })).toHaveAttribute(
      "data-ds-name",
      "ThreadMark",
    );
  });

  it("renders a create action for meetings that need briefing prep", () => {
    const onCreateBriefing = vi.fn();

    render(
      <MeetingSpineItem
        time="2:00"
        duration="60m"
        type="one_on_one"
        warn
        entityName="Priya Raman - 1:1"
        title="1:1 with Priya"
        prepState="needs"
        onCreateBriefing={onCreateBriefing}
      />,
    );

    screen.getByRole("button", { name: "Create briefing" }).click();
    expect(onCreateBriefing).toHaveBeenCalledTimes(1);
    expect(screen.getByText("1:1 with Priya").closest("article")).toHaveAttribute(
      "data-type",
      "one_on_one",
    );
    expect(screen.getByText("No briefing yet")).toHaveAttribute("data-tone", "terracotta");
  });
});

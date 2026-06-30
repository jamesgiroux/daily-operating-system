/** @vitest-environment jsdom */

import type React from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MeetingSpineItem } from "./MeetingSpineItem";

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, ...props }: Record<string, unknown>) => (
    <a href={String(props.to ?? "#")}>{children as React.ReactNode}</a>
  ),
}));

describe("MeetingSpineItem", () => {
  it("exposes design-system metadata and keeps the state label in the time rail", () => {
    render(
      <MeetingSpineItem
        time="10:00"
        duration="45m"
        state="in-progress"
        type="customer"
        entityName="Acme Corp - Renewal"
        title="Acme renewal - pricing and tier 3"
        context="Legal needs final terms language before the MSA review."
        attendees="Jen Park, Dan Mitchell, +2"
        prepState="ready"
        meetingId="acme-renewal"
      />,
    );

    const item = screen.getByText("Acme renewal - pricing and tier 3").closest("article");
    expect(item).toHaveAttribute("data-ds-name", "MeetingSpineItem");
    expect(item).toHaveAttribute("data-ds-tier", "pattern");
    expect(screen.getByText("Now")).toBeInTheDocument();
    expect(screen.getByText("Briefing fresh")).toHaveAttribute("data-ds-name", "Pill");
  });

  it("renders a create-briefing link for meetings that need prep", () => {
    render(
      <MeetingSpineItem
        time="2:00"
        duration="60m"
        type="one_on_one"
        warn
        entityName="Priya Raman - 1:1"
        title="1:1 with Priya"
        prepState="needs"
        meetingId="priya-one-on-one"
      />,
    );

    const createLink = screen.getByText("Create briefing").closest("a");
    expect(createLink).toHaveAttribute("href", "/meeting/$meetingId");
    expect(screen.getByText("1:1 with Priya").closest("article")).toHaveAttribute(
      "data-type",
      "one_on_one",
    );
    expect(screen.getByText("No briefing yet")).toHaveAttribute("data-tone", "terracotta");
  });

  it("supports partner meeting identity", () => {
    render(
      <MeetingSpineItem
        time="2:00"
        duration="60m"
        type="partner"
        entityName="Northwind Traders - Partner"
        title="Northwind partner sync"
        prepState="needs"
      />,
    );

    expect(screen.getByText("Northwind partner sync").closest("article")).toHaveAttribute(
      "data-type",
      "partner",
    );
  });
});

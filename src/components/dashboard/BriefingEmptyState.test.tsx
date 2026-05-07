/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { BriefingEmptyState } from "./BriefingEmptyState";

describe("BriefingEmptyState", () => {
  it("renders eyebrow + headline + lede at minimum", () => {
    render(
      <BriefingEmptyState
        eyebrow="DAILY BRIEFING"
        headline="Your day, when DailyOS can read it."
        lede="The briefing is a synthesis of your calendar, mail, and signal sources."
      />,
    );
    expect(screen.getByText("DAILY BRIEFING")).toBeTruthy();
    expect(screen.getByText("Your day, when DailyOS can read it.")).toBeTruthy();
    expect(screen.getByText(/synthesis of your calendar/)).toBeTruthy();
  });

  it("renders checklist items when provided", () => {
    render(
      <BriefingEmptyState
        eyebrow="x"
        headline="y"
        lede="z"
        checklistItems={[
          { label: "Connect Google", status: "todo" },
          { label: "Optional: Glean", status: "done" },
        ]}
      />,
    );
    expect(screen.getByText("Connect Google")).toBeTruthy();
    expect(screen.getByText("Optional: Glean")).toBeTruthy();
  });

  it("checklist done item carries the done class", () => {
    const { container } = render(
      <BriefingEmptyState
        eyebrow="x"
        headline="y"
        lede="z"
        checklistItems={[{ label: "Done thing", status: "done" }]}
      />,
    );
    const items = container.querySelectorAll("li");
    expect(items[0].className).toMatch(/(^|_)checklistItemDone(_|$)/);
  });

  it("omits checklist when not provided or empty", () => {
    const { container, rerender } = render(
      <BriefingEmptyState eyebrow="x" headline="y" lede="z" />,
    );
    expect(
      container.querySelector('[data-ds-name="BriefingEmptyState.checklist"]'),
    ).toBeNull();

    rerender(
      <BriefingEmptyState
        eyebrow="x"
        headline="y"
        lede="z"
        checklistItems={[]}
      />,
    );
    expect(
      container.querySelector('[data-ds-name="BriefingEmptyState.checklist"]'),
    ).toBeNull();
  });

  it("CTA button fires onClick", () => {
    const onClick = vi.fn();
    const { container } = render(
      <BriefingEmptyState
        eyebrow="x"
        headline="y"
        lede="z"
        cta={{ label: "Connect Google", onClick }}
      />,
    );
    const button = container.querySelector(
      '[data-ds-name="BriefingEmptyState.cta"]',
    ) as HTMLButtonElement;
    expect(button.textContent).toBe("Connect Google");
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("omits CTA when not provided", () => {
    const { container } = render(
      <BriefingEmptyState eyebrow="x" headline="y" lede="z" />,
    );
    expect(
      container.querySelector('[data-ds-name="BriefingEmptyState.cta"]'),
    ).toBeNull();
  });

  it("emits ds-inspector attributes for design-system audit", () => {
    const { container } = render(
      <BriefingEmptyState eyebrow="x" headline="y" lede="z" />,
    );
    const root = container.querySelector('[data-ds-name="BriefingEmptyState"]');
    expect(root?.getAttribute("data-ds-tier")).toBe("pattern");
    expect(root?.getAttribute("data-ds-spec")).toBe(
      "patterns/BriefingEmptyState.md",
    );
  });
});

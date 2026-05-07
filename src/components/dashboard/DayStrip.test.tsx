/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DayStrip } from "./DayStrip";
import type { DayStripViewModel } from "@/types/briefing";

const dayStrip: DayStripViewModel = {
  prev: {
    label: "Yesterday",
    isoDate: "2026-04-22",
    preview: "Wed - Acme call captured - 2 actions logged",
    href: "/briefing/2026-04-22",
  },
  current: {
    label: "Today",
    isoDate: "2026-04-23",
    ariaLabel: "Today, Thursday April 23",
  },
  next: {
    label: "Tomorrow",
    isoDate: "2026-04-24",
    preview: "Fri - Northwind follow-up at 9:00",
    href: "/briefing/2026-04-24",
  },
};

describe("DayStrip", () => {
  it("renders previous, current, and next labels", () => {
    render(<DayStrip {...dayStrip} />);

    expect(screen.getByText("Yesterday")).toBeTruthy();
    expect(screen.getByText("Today")).toBeTruthy();
    expect(screen.getByText("Tomorrow")).toBeTruthy();
  });

  it("renders isoDate attributes for previous, current, and next days", () => {
    const { container } = render(<DayStrip {...dayStrip} />);

    expect(container.querySelector('[data-iso-date="2026-04-22"]')).toBeTruthy();
    expect(container.querySelector('[data-iso-date="2026-04-23"]')).toBeTruthy();
    expect(container.querySelector('[data-iso-date="2026-04-24"]')).toBeTruthy();
  });

  it("uses the current aria label from the contract", () => {
    render(<DayStrip {...dayStrip} />);

    const current = screen.getByLabelText("Today, Thursday April 23");
    expect(current.textContent).toContain("Today");
    expect(current.getAttribute("aria-current")).toBe("date");
  });

  it("renders the current dateTime attribute from the contract isoDate", () => {
    render(<DayStrip {...dayStrip} />);

    const current = screen.getByLabelText("Today, Thursday April 23");
    expect(current.getAttribute("dateTime")).toBe("2026-04-23");
  });

  it("uses contract hrefs for neighbor links", () => {
    render(<DayStrip {...dayStrip} />);

    const prev = screen.getByRole("link", { name: /Yesterday/ });
    const next = screen.getByRole("link", { name: /Tomorrow/ });

    expect(prev.getAttribute("href")).toBe("/briefing/2026-04-22");
    expect(next.getAttribute("href")).toBe("/briefing/2026-04-24");
  });

  it("renders neighbor preview text", () => {
    render(<DayStrip {...dayStrip} />);

    expect(screen.getByText("Wed - Acme call captured - 2 actions logged")).toBeTruthy();
    expect(screen.getByText("Fri - Northwind follow-up at 9:00")).toBeTruthy();
  });

  it("renders ds inspector attributes for design-system audit", () => {
    const { container } = render(<DayStrip {...dayStrip} />);
    const wrapper = container.querySelector('[data-ds-name="DayStrip"]');

    expect(wrapper?.getAttribute("data-ds-spec")).toBe("patterns/DayStrip.md");
  });
});

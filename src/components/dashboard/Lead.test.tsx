/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Lead } from "./Lead";
import type { LeadViewModel } from "@/types/briefing";

function makeLead(partial: Partial<LeadViewModel> = {}): LeadViewModel {
  return {
    headline: {
      lead: "Four meetings today, two with customers.",
    },
    focusCapacity: "4h 30m available - 2 deep work blocks - 4 meetings",
    ...partial,
  };
}

describe("Lead", () => {
  it("renders headline.lead", () => {
    render(<Lead lead={makeLead()} />);
    expect(
      screen.getByText("Four meetings today, two with customers."),
    ).toBeTruthy();
  });

  it("renders headline.punchLine when present", () => {
    const { container } = render(
      <Lead
        lead={makeLead({
          headline: {
            lead: "Four meetings today, two with customers.",
            punchLine: "The Acme renewal is the one to nail.",
          },
        })}
      />,
    );

    const punchLine = screen.getByText("The Acme renewal is the one to nail.");
    expect(punchLine).toBeTruthy();
    expect(container.querySelector('[data-ds-name="Lead.punchLine"]')).toBe(
      punchLine,
    );
  });

  it("renders focusCapacity", () => {
    render(<Lead lead={makeLead({ focusCapacity: "90m open after 2:00." })} />);
    expect(screen.getByText("90m open after 2:00.")).toBeTruthy();
  });

  it("renders focusBlock when present", () => {
    render(
      <Lead
        lead={makeLead({
          focusBlock: "Prep legal terms before the MSA review.",
        })}
      />,
    );
    expect(
      screen.getByText("Prep legal terms before the MSA review."),
    ).toBeTruthy();
  });

  it("omits focusBlock when absent", () => {
    const { container } = render(<Lead lead={makeLead()} />);
    expect(container.querySelector('[class*="focusBlock"]')).toBeNull();
  });

  it("renders ds inspector attributes for design-system audit", () => {
    const { container } = render(<Lead lead={makeLead()} />);
    const wrapper = container.querySelector('[data-ds-name="Lead"]');

    expect(wrapper?.getAttribute("data-ds-tier")).toBe("pattern");
    expect(wrapper?.getAttribute("data-ds-spec")).toBe("patterns/Lead.md");
  });
});

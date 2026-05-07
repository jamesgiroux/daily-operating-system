/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { InferredActionSelector } from "./InferredActionSelector";
import type { InferredActionSelectorViewModel } from "@/types/briefing";

function makeSelector(
  partial: Partial<InferredActionSelectorViewModel> = {},
): InferredActionSelectorViewModel {
  return {
    triggerLabel: "Suggested action",
    selectedOptionId: "snooze",
    options: [
      {
        id: "snooze",
        label: "Snooze until Q3 review",
        confidence: { value: 0.86, label: "86%" },
      },
      {
        id: "add-to-sync",
        label: "Add to Friday CSM sync",
        confidence: { value: 0.71, label: "71%" },
      },
      { id: "surface", label: "Surface tomorrow" },
      { id: "dismiss", label: "Dismiss", divider: true },
    ],
    ...partial,
  };
}

describe("InferredActionSelector", () => {
  it("renders the selected option label in the trigger", () => {
    render(
      <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />,
    );

    expect(
      screen.getByRole("button", { name: "Snooze until Q3 review" }),
    ).toBeInTheDocument();
  });

  it("toggles the dropdown with aria-expanded on trigger click", () => {
    render(
      <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />,
    );

    const trigger = screen.getByRole("button", {
      name: "Snooze until Q3 review",
    });
    expect(trigger).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("menu")).toBeInTheDocument();

    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("renders options with confidence labels", () => {
    render(
      <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Snooze until Q3 review" }),
    );

    expect(
      screen.getByRole("menuitem", { name: "Snooze until Q3 review 86%" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("menuitem", { name: "Add to Friday CSM sync 71%" }),
    ).toBeInTheDocument();
  });

  it("renders a separator before divider options", () => {
    render(
      <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Snooze until Q3 review" }),
    );

    expect(screen.getByRole("separator")).toBeInTheDocument();
  });

  it("selecting an option fires onSelect with the option id", () => {
    const onSelect = vi.fn();
    render(
      <InferredActionSelector selector={makeSelector()} onSelect={onSelect} />,
    );

    const trigger = screen.getByRole("button", {
      name: "Snooze until Q3 review",
    });
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("menuitem", { name: "Dismiss" }));

    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith("dismiss");
    expect(trigger).toHaveAttribute("aria-expanded", "false");
  });

  it("closes the dropdown on Escape", () => {
    render(
      <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />,
    );

    const trigger = screen.getByRole("button", {
      name: "Snooze until Q3 review",
    });
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");

    fireEvent.keyDown(document, { key: "Escape" });

    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("closes the dropdown on outside click", () => {
    render(
      <>
        <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />
        <button type="button">Outside target</button>
      </>,
    );

    const trigger = screen.getByRole("button", {
      name: "Snooze until Q3 review",
    });
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");

    fireEvent.mouseDown(screen.getByRole("button", { name: "Outside target" }));

    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("renders design-system inspector attributes", () => {
    const { container } = render(
      <InferredActionSelector selector={makeSelector()} onSelect={vi.fn()} />,
    );

    const wrapper = container.querySelector(
      '[data-ds-name="InferredActionSelector"]',
    );
    expect(wrapper).toHaveAttribute("data-ds-tier", "pattern");
    expect(wrapper).toHaveAttribute(
      "data-ds-spec",
      "patterns/InferredActionSelector.md",
    );
  });
});

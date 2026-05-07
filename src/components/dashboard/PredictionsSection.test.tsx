/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PredictionsSection } from "./PredictionsSection";
import type {
  PredictionItem,
  PredictionsViewModel,
  TrustBandWire,
} from "@/types/briefing";

function makeItem(
  partial: Partial<PredictionItem> = {},
  band: TrustBandWire = "likely_current",
): PredictionItem {
  return {
    trustBand: band,
    id: "pred_1",
    text: "Northwind QBR raises pricing pushback once Kevin sees the renewal terms.",
    confidence: { value: 0.72, label: "72%" },
    abilitySource: {
      id: "predict_meeting_friction",
      label: "predict_meeting_friction",
    },
    basisLink: { label: "basis", href: "/predictions/pred_1" },
    ...partial,
  };
}

function makeVM(
  items: PredictionItem[] = [],
  partial: Partial<PredictionsViewModel> = {},
): PredictionsViewModel {
  return {
    label: "Predictions",
    countLabel: items.length === 0 ? "0 today" : `${items.length} today`,
    collapsedLabel:
      items.length === 0
        ? "0 predictions today"
        : `${items.length} predictions today`,
    expandHint: "expand",
    count: items.length,
    predictions: items,
    ...partial,
  };
}

describe("PredictionsSection", () => {
  it("renders collapsed by default with the count line", () => {
    render(<PredictionsSection predictions={makeVM([makeItem()])} />);
    expect(screen.getByText("1 predictions today")).toBeTruthy();
    expect(screen.getByRole("button").getAttribute("aria-expanded")).toBe("false");
  });

  it("clicking the trigger expands inline", () => {
    render(<PredictionsSection predictions={makeVM([makeItem()])} />);
    const trigger = screen.getByRole("button");
    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText(/Northwind QBR/)).toBeTruthy();
  });

  it("expanded list renders confidence + ability + basis link per item", () => {
    render(<PredictionsSection predictions={makeVM([makeItem()])} />);
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText("72%")).toBeTruthy();
    expect(screen.getByText("via predict_meeting_friction")).toBeTruthy();
    const basis = screen.getByText("basis") as HTMLAnchorElement;
    expect(basis.tagName).toBe("A");
    expect(basis.getAttribute("href")).toBe("/predictions/pred_1");
  });

  it("renders TrustBandBadge for scored bands", () => {
    const { container } = render(
      <PredictionsSection
        predictions={makeVM([makeItem({}, "needs_verification")])}
      />,
    );
    fireEvent.click(screen.getByRole("button"));
    const badge = container.querySelector('[data-ds-name="TrustBandBadge"]');
    expect(badge).not.toBeNull();
    expect(badge?.getAttribute("data-band")).toBe("needs_verification");
  });

  it("omits TrustBandBadge for unscored items", () => {
    const { container } = render(
      <PredictionsSection
        predictions={makeVM([makeItem({}, "unscored")])}
      />,
    );
    fireEvent.click(screen.getByRole("button"));
    const badge = container.querySelector('[data-ds-name="TrustBandBadge"]');
    expect(badge).toBeNull();
  });

  it("count=0 disables the trigger and hides the expand hint", () => {
    render(<PredictionsSection predictions={makeVM([])} />);
    const trigger = screen.getByRole("button") as HTMLButtonElement;
    expect(trigger.disabled).toBe(true);
    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("emits ds-inspector attributes for design-system audit", () => {
    const { container } = render(<PredictionsSection predictions={makeVM([])} />);
    const wrapper = container.querySelector('[data-ds-name="PredictionsSection"]');
    expect(wrapper?.getAttribute("data-ds-tier")).toBe("pattern");
    expect(wrapper?.getAttribute("data-ds-spec")).toBe(
      "patterns/PredictionsSection.md",
    );
  });
});

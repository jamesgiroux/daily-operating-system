/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ProvenanceStat } from "./ProvenanceStat";
import type { ProvenanceStat as ProvenanceStatViewModel } from "@/types/briefing";

function makeStat(
  partial: Partial<ProvenanceStatViewModel> = {},
): ProvenanceStatViewModel {
  return {
    trustBand: "unscored",
    label: "Health",
    value: "71 +3",
    ...partial,
  };
}

describe("ProvenanceStat", () => {
  it("renders label and value", () => {
    render(<ProvenanceStat stat={makeStat()} />);
    expect(screen.getByText("Health")).toBeTruthy();
    expect(screen.getByText("71 +3")).toBeTruthy();
  });

  it("default (no trend) carries no trend class", () => {
    const { container } = render(<ProvenanceStat stat={makeStat()} />);
    const valueEl = container.querySelector(`.${"value"}`);
    // Class names are scoped by CSS Modules — we look for the element and
    // assert it doesn't carry the up/down/flat marker classes.
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).not.toMatch(/_up_|_down_|_flat_/);
    void valueEl;
  });

  it("trend='up' applies the up class", () => {
    const { container } = render(
      <ProvenanceStat stat={makeStat({ trend: "up" })} />,
    );
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).toMatch(/_up_|up$/);
  });

  it("trend='down' applies the down class", () => {
    const { container } = render(
      <ProvenanceStat stat={makeStat({ trend: "down" })} />,
    );
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).toMatch(/_down_|down$/);
  });

  it("trend='flat' applies the flat class", () => {
    const { container } = render(
      <ProvenanceStat stat={makeStat({ trend: "flat" })} />,
    );
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).toMatch(/_flat_|flat$/);
  });

  it("emits ds-inspector attributes for design-system audit", () => {
    const { container } = render(<ProvenanceStat stat={makeStat()} />);
    const wrapper = container.querySelector('[data-ds-name="ProvenanceStat"]');
    expect(wrapper?.getAttribute("data-ds-tier")).toBe("primitive");
    expect(wrapper?.getAttribute("data-ds-spec")).toBe(
      "primitives/ProvenanceStat.md",
    );
  });
});

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
    // CSS Module class names are scoped (hashed) — assert the value element's
    // className doesn't contain up/down/flat markers.
    const { container } = render(<ProvenanceStat stat={makeStat()} />);
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).not.toMatch(/(^|_)(up|down|flat)(_|$)/);
  });

  it("trend='up' applies the up class", () => {
    const { container } = render(
      <ProvenanceStat stat={makeStat({ trend: "up" })} />,
    );
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).toMatch(/(^|_)up(_|$)/);
  });

  it("trend='down' applies the down class", () => {
    const { container } = render(
      <ProvenanceStat stat={makeStat({ trend: "down" })} />,
    );
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).toMatch(/(^|_)down(_|$)/);
  });

  it("trend='flat' applies the flat class", () => {
    const { container } = render(
      <ProvenanceStat stat={makeStat({ trend: "flat" })} />,
    );
    const valueClassName =
      container.querySelectorAll("span")[1]?.className ?? "";
    expect(valueClassName).toMatch(/(^|_)flat(_|$)/);
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

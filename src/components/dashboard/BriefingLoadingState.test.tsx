/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BriefingLoadingState } from "./BriefingLoadingState";

describe("BriefingLoadingState", () => {
  it("renders headline and eyebrow from props", () => {
    render(
      <BriefingLoadingState
        headline="Reading your day…"
        eyebrow="GATHERING TODAY'S SIGNALS"
      />,
    );
    expect(screen.getByText("Reading your day…")).toBeTruthy();
    expect(screen.getByText("GATHERING TODAY'S SIGNALS")).toBeTruthy();
  });

  it("renders pulsing dot by default", () => {
    const { container } = render(
      <BriefingLoadingState headline="x" eyebrow="y" />,
    );
    expect(
      container.querySelector('[data-ds-name="BriefingLoadingState.pulse"]'),
    ).not.toBeNull();
  });

  it("withPulse=false suppresses the pulsing dot", () => {
    const { container } = render(
      <BriefingLoadingState headline="x" eyebrow="y" withPulse={false} />,
    );
    expect(
      container.querySelector('[data-ds-name="BriefingLoadingState.pulse"]'),
    ).toBeNull();
  });

  it("uses role=status with aria-live=polite for screen readers", () => {
    const { container } = render(
      <BriefingLoadingState headline="x" eyebrow="y" />,
    );
    const root = container.querySelector('[data-ds-name="BriefingLoadingState"]');
    expect(root?.getAttribute("role")).toBe("status");
    expect(root?.getAttribute("aria-live")).toBe("polite");
  });

  it("emits ds-inspector attributes for design-system audit", () => {
    const { container } = render(
      <BriefingLoadingState headline="x" eyebrow="y" />,
    );
    const root = container.querySelector('[data-ds-name="BriefingLoadingState"]');
    expect(root?.getAttribute("data-ds-tier")).toBe("pattern");
    expect(root?.getAttribute("data-ds-spec")).toBe(
      "patterns/BriefingLoadingState.md",
    );
  });
});

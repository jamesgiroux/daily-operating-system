/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { BriefingErrorState } from "./BriefingErrorState";

describe("BriefingErrorState", () => {
  it("renders eyebrow + message at minimum", () => {
    render(
      <BriefingErrorState
        eyebrow="BRIEFING UNAVAILABLE"
        message="We couldn't load your briefing."
      />,
    );
    expect(screen.getByText("BRIEFING UNAVAILABLE")).toBeTruthy();
    expect(screen.getByText("We couldn't load your briefing.")).toBeTruthy();
  });

  it("renders detailMessage when provided", () => {
    render(
      <BriefingErrorState
        eyebrow="x"
        message="primary"
        detailMessage="A signal source isn't responding."
      />,
    );
    expect(screen.getByText("A signal source isn't responding.")).toBeTruthy();
  });

  it("omits detailMessage when not provided", () => {
    const { container } = render(
      <BriefingErrorState eyebrow="x" message="primary" />,
    );
    // Look for the detail paragraph by checking text content; absence means
    // the optional <p> isn't rendered.
    expect(container.querySelectorAll("p")).toHaveLength(1); // only eyebrow
  });

  it("renders meta line with code + service when both provided", () => {
    render(
      <BriefingErrorState
        eyebrow="x"
        message="y"
        code="dependency_failed"
        service="predictions"
      />,
    );
    expect(screen.getByText(/code: dependency_failed/)).toBeTruthy();
    expect(screen.getByText(/service: predictions/)).toBeTruthy();
  });

  it("omits meta line when neither code nor service provided", () => {
    const { container } = render(
      <BriefingErrorState eyebrow="x" message="y" />,
    );
    expect(
      container.querySelector('[data-ds-name="BriefingErrorState.meta"]'),
    ).toBeNull();
  });

  it("retry button fires onRetry", () => {
    const onRetry = vi.fn();
    const { container } = render(
      <BriefingErrorState eyebrow="x" message="y" onRetry={onRetry} />,
    );
    const button = container.querySelector(
      '[data-ds-name="BriefingErrorState.retry"]',
    ) as HTMLButtonElement;
    fireEvent.click(button);
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("diagnostics button fires onDiagnostics", () => {
    const onDiagnostics = vi.fn();
    const { container } = render(
      <BriefingErrorState
        eyebrow="x"
        message="y"
        onDiagnostics={onDiagnostics}
      />,
    );
    const button = container.querySelector(
      '[data-ds-name="BriefingErrorState.diagnostics"]',
    ) as HTMLButtonElement;
    fireEvent.click(button);
    expect(onDiagnostics).toHaveBeenCalledOnce();
  });

  it("omits buttons entirely when no callback provided", () => {
    const { container } = render(
      <BriefingErrorState eyebrow="x" message="y" />,
    );
    expect(container.querySelectorAll("button")).toHaveLength(0);
  });

  it("uses role=alert for screen readers", () => {
    const { container } = render(
      <BriefingErrorState eyebrow="x" message="y" />,
    );
    const root = container.querySelector('[data-ds-name="BriefingErrorState"]');
    expect(root?.getAttribute("role")).toBe("alert");
  });

  it("emits ds-inspector attributes for design-system audit", () => {
    const { container } = render(
      <BriefingErrorState eyebrow="x" message="y" />,
    );
    const root = container.querySelector('[data-ds-name="BriefingErrorState"]');
    expect(root?.getAttribute("data-ds-tier")).toBe("pattern");
    expect(root?.getAttribute("data-ds-spec")).toBe(
      "patterns/BriefingErrorState.md",
    );
  });
});

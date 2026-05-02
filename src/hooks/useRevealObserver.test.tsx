/** @vitest-environment jsdom */

import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useRevealObserver } from "@/hooks/useRevealObserver";

class MockIntersectionObserver {
  readonly root = null;
  readonly rootMargin = "";
  readonly thresholds = [];
  readonly observe = vi.fn();
  readonly unobserve = vi.fn();
  readonly disconnect = vi.fn();
  readonly takeRecords = vi.fn(() => []);

  constructor(private readonly callback: IntersectionObserverCallback) {
    intersectionObservers.push(this);
  }

  trigger(target: Element) {
    this.callback(
      [{ isIntersecting: true, target } as IntersectionObserverEntry],
      this as unknown as IntersectionObserver,
    );
  }
}

const intersectionObservers: MockIntersectionObserver[] = [];

function RevealHarness({ showLate }: { showLate: boolean }) {
  useRevealObserver(true);

  return (
    <div>
      <div data-testid="initial" className="editorial-reveal">
        Initial
      </div>
      {showLate ? (
        <section>
          <div data-testid="late" className="editorial-reveal">
            Late
          </div>
        </section>
      ) : null}
    </div>
  );
}

function VisibleRevealHarness() {
  useRevealObserver(true);

  return (
    <div data-testid="already-visible" className="editorial-reveal visible">
      Already visible
    </div>
  );
}

describe("useRevealObserver", () => {
  beforeEach(() => {
    intersectionObservers.length = 0;
    vi.useFakeTimers();
    vi.stubGlobal("IntersectionObserver", MockIntersectionObserver);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("observes reveal nodes inserted after the initial setup pass", async () => {
    const { rerender } = render(<RevealHarness showLate={false} />);

    act(() => {
      vi.advanceTimersByTime(50);
    });

    expect(intersectionObservers).toHaveLength(1);
    const observer = intersectionObservers[0];
    const initial = screen.getByTestId("initial");
    expect(observer.observe).toHaveBeenCalledWith(initial);

    act(() => {
      observer.trigger(initial);
    });
    expect(initial).toHaveClass("visible");

    await act(async () => {
      rerender(<RevealHarness showLate />);
      await Promise.resolve();
    });

    const late = screen.getByTestId("late");
    expect(observer.observe).toHaveBeenCalledWith(late);

    act(() => {
      observer.trigger(late);
    });
    expect(late).toHaveClass("visible");
  });

  it("does not re-observe reveal nodes that are already visible", () => {
    render(<VisibleRevealHarness />);

    act(() => {
      vi.advanceTimersByTime(50);
    });

    expect(intersectionObservers).toHaveLength(1);
    expect(intersectionObservers[0].observe).not.toHaveBeenCalledWith(
      screen.getByTestId("already-visible"),
    );
  });
});

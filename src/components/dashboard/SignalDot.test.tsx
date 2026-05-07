/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SignalDot } from "./SignalDot";
import type {
  MovingSignalViewModel,
  SignalDotKind,
  TrustMixin,
} from "@/types/briefing";

const baseTrust: TrustMixin = {
  trustBand: "unscored",
};

function makeSignal(
  partial: Partial<MovingSignalViewModel> = {},
): MovingSignalViewModel {
  return {
    ...baseTrust,
    kind: "meeting",
    when: "10:00",
    whatSegments: [{ text: "Pricing alignment" }],
    urgency: "normal",
    ...partial,
  };
}

const ALL_KINDS: SignalDotKind[] = [
  "meeting",
  "action",
  "email",
  "lifecycle",
  "gong-call",
  "zendesk-ticket",
  "slack-thread",
  "linear-issue",
];

describe("SignalDot", () => {
  it.each(ALL_KINDS)("renders kind=%s with data-kind attribute", (kind) => {
    render(<SignalDot signal={makeSignal({ kind })} />);
    const el = screen.getByText("Pricing alignment").closest('[data-ds-name="SignalDot"]');
    expect(el).not.toBeNull();
    expect(el?.getAttribute("data-kind")).toBe(kind);
  });

  it("renders when label and what text", () => {
    render(
      <SignalDot
        signal={makeSignal({
          when: "2d",
          whatSegments: [{ text: "Send pricing memo — overdue" }],
        })}
      />,
    );
    expect(screen.getByText("2d")).toBeTruthy();
    expect(screen.getByText("Send pricing memo — overdue")).toBeTruthy();
  });

  it("emphasizes segments with emphasized=true via <em>", () => {
    const { container } = render(
      <SignalDot
        signal={makeSignal({
          whatSegments: [
            { text: "Legal flagged " },
            { text: "3 MSA clauses", emphasized: true },
          ],
        })}
      />,
    );
    const em = container.querySelector("em");
    expect(em?.textContent).toBe("3 MSA clauses");
  });

  it("urgency=overdue applies the overdue modifier class", () => {
    const { container } = render(
      <SignalDot signal={makeSignal({ urgency: "overdue" })} />,
    );
    const wrapper = container.querySelector('[data-ds-name="SignalDot"]');
    expect(wrapper?.className).toMatch(/overdue/);
  });

  it("correctionState=corrected applies the corrected modifier class", () => {
    const { container } = render(
      <SignalDot
        signal={makeSignal({ correctionState: "corrected" })}
      />,
    );
    const wrapper = container.querySelector('[data-ds-name="SignalDot"]');
    expect(wrapper?.className).toMatch(/corrected/);
  });

  it("correctionState=contested applies the contested modifier class", () => {
    const { container } = render(
      <SignalDot
        signal={makeSignal({ correctionState: "contested" })}
      />,
    );
    const wrapper = container.querySelector('[data-ds-name="SignalDot"]');
    expect(wrapper?.className).toMatch(/contested/);
  });

  it("threadAction button calls onThreadAction with stop-propagation", () => {
    const parentClick = vi.fn();
    const onThreadAction = vi.fn();
    const signal = makeSignal({
      threadAction: { label: "→ thread", href: "/threads/abc" },
    });

    const { container } = render(
      <div onClick={parentClick}>
        <SignalDot signal={signal} onThreadAction={onThreadAction} />
      </div>,
    );

    const button = container.querySelector(
      '[data-ds-name="SignalDot.threadAction"]',
    ) as HTMLButtonElement;
    expect(button.tagName).toBe("BUTTON");

    fireEvent.click(button);

    expect(onThreadAction).toHaveBeenCalledTimes(1);
    expect(onThreadAction).toHaveBeenCalledWith(signal);
    expect(parentClick).not.toHaveBeenCalled();
  });

  it("threadAction button is safe to click without onThreadAction prop", () => {
    const { container } = render(
      <SignalDot
        signal={makeSignal({
          threadAction: { label: "→ thread", href: "/threads/abc" },
        })}
      />,
    );
    const button = container.querySelector(
      '[data-ds-name="SignalDot.threadAction"]',
    ) as HTMLButtonElement;
    expect(() => fireEvent.click(button)).not.toThrow();
  });

  it("omits threadAction button when not provided", () => {
    const { container } = render(<SignalDot signal={makeSignal()} />);
    expect(
      container.querySelector('[data-ds-name="SignalDot.threadAction"]'),
    ).toBeNull();
  });

  it("renders ds inspector attributes for design-system audit", () => {
    const { container } = render(<SignalDot signal={makeSignal()} />);
    const wrapper = container.querySelector('[data-ds-name="SignalDot"]');
    expect(wrapper?.getAttribute("data-ds-spec")).toBe(
      "primitives/SignalDot.md",
    );
    expect(wrapper?.getAttribute("data-ds-tier")).toBe("primitive");
  });
});

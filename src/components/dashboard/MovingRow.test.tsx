/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MovingRow } from "./MovingRow";
import type {
  MovingEntityKind,
  MovingEntityViewModel,
  MovingSignalViewModel,
  ProvenanceStat as ProvenanceStatViewModel,
  TrustMixin,
} from "@/types/briefing";

const baseTrust: TrustMixin = {
  trustBand: "unscored",
};

const ALL_KINDS: MovingEntityKind[] = [
  "customer",
  "person",
  "project",
  "internal",
  "lifecycle",
];

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

function makeStat(
  partial: Partial<ProvenanceStatViewModel> = {},
): ProvenanceStatViewModel {
  return {
    ...baseTrust,
    label: "Health",
    value: "71 +3",
    ...partial,
  };
}

function makeEntity(
  partial: Partial<MovingEntityViewModel["entity"]> = {},
): MovingEntityViewModel["entity"] {
  return {
    id: "acct_1",
    name: "Globex",
    entityType: "account",
    ...partial,
  };
}

function makeRow(
  partial: Partial<MovingEntityViewModel> = {},
): MovingEntityViewModel {
  return {
    kind: "customer",
    entity: makeEntity(),
    href: "/accounts/acct_1",
    statePill: { label: "Renewal up", tone: "turmeric" },
    lede:
      "Pricing memo went out Tuesday. Legal flagged 3 MSA clauses; champion still on track.",
    signals: [
      makeSignal({
        when: "10:00",
        whatSegments: [{ text: "Pricing alignment in progress" }],
      }),
      makeSignal({
        kind: "action",
        when: "2d",
        whatSegments: [{ text: "Send pricing memo overdue" }],
        urgency: "overdue",
      }),
    ],
    provenanceStats: [
      makeStat({ label: "Health", value: "71 +3", trend: "up" }),
      makeStat({ label: "Stage", value: "Renewal" }),
      makeStat({ label: "Confidence", value: "82%", trend: "up" }),
    ],
    ...partial,
  };
}

describe("MovingRow", () => {
  it.each(ALL_KINDS)("renders kind=%s with the data-kind attribute", (kind) => {
    render(<MovingRow {...makeRow({ kind })} onNavigate={vi.fn()} />);

    const row = screen.getByRole("link");
    expect(row.getAttribute("data-kind")).toBe(kind);
    expect(screen.getByText("Globex")).toBeTruthy();
  });

  it("renders the compact state pill with its tone", () => {
    const { container } = render(
      <MovingRow
        {...makeRow({ statePill: { label: "At Risk", tone: "terracotta" } })}
        onNavigate={vi.fn()}
      />,
    );

    const pill = screen.getByText("At Risk").closest('[data-ds-name="Pill"]');
    expect(pill).not.toBeNull();
    expect(pill?.getAttribute("data-tone")).toBe("terracotta");
    expect(container.querySelector('[data-ds-name="MovingRow"]')).not.toBeNull();
  });

  it("renders the lede", () => {
    render(
      <MovingRow
        {...makeRow({ lede: "Exec sponsor changed overnight." })}
        onNavigate={vi.fn()}
      />,
    );

    expect(screen.getByText("Exec sponsor changed overnight.")).toBeTruthy();
  });

  it("renders signals in service order", () => {
    const first = makeSignal({
      kind: "email",
      when: "3h",
      whatSegments: [{ text: "Legal flagged clauses" }],
    });
    const second = makeSignal({
      kind: "linear-issue",
      when: "1h",
      whatSegments: [{ text: "Issue moved to done" }],
    });
    const third = makeSignal({
      kind: "lifecycle",
      when: "New",
      whatSegments: [{ text: "Moved to renewing" }],
    });
    const { container } = render(
      <MovingRow
        {...makeRow({ signals: [first, second, third] })}
        onNavigate={vi.fn()}
      />,
    );

    const renderedSignals = Array.from(
      container.querySelectorAll('[data-ds-name="SignalDot"]'),
    );
    expect(renderedSignals).toHaveLength(3);
    expect(renderedSignals[0]?.textContent).toContain("Legal flagged clauses");
    expect(renderedSignals[1]?.textContent).toContain("Issue moved to done");
    expect(renderedSignals[2]?.textContent).toContain("Moved to renewing");
  });

  it("stacks provenance stats in the right column order", () => {
    const { container } = render(
      <MovingRow
        {...makeRow({
          provenanceStats: [
            makeStat({ label: "Owner", value: "You" }),
            makeStat({ label: "Stage", value: "Active" }),
            makeStat({ label: "Confidence", value: "74%" }),
          ],
        })}
        onNavigate={vi.fn()}
      />,
    );

    const stack = container.querySelector(
      '[data-ds-name="MovingRow.provenanceStats"]',
    );
    const stats = Array.from(
      stack?.querySelectorAll('[data-ds-name="ProvenanceStat"]') ?? [],
    );
    expect(stats).toHaveLength(3);
    expect(stats[0]?.textContent).toContain("Owner");
    expect(stats[1]?.textContent).toContain("Stage");
    expect(stats[2]?.textContent).toContain("Confidence");
  });

  it("fires onNavigate when the row is clicked", () => {
    const onNavigate = vi.fn();
    render(<MovingRow {...makeRow()} onNavigate={onNavigate} />);

    fireEvent.click(screen.getByRole("link"));

    expect(onNavigate).toHaveBeenCalledTimes(1);
    expect(onNavigate.mock.calls[0]?.[0]).toBe("/accounts/acct_1");
    expect(onNavigate.mock.calls[0]?.[1]).toMatchObject({
      kind: "customer",
      href: "/accounts/acct_1",
    });
  });

  it("does not fire row navigation from a thread action click", () => {
    const onNavigate = vi.fn();
    const onThreadAction = vi.fn();
    const signal = makeSignal({
      threadAction: { label: "Open thread", href: "/threads/thread_1" },
    });
    const { container } = render(
      <MovingRow
        {...makeRow({ signals: [signal] })}
        onNavigate={onNavigate}
        onThreadAction={onThreadAction}
      />,
    );

    const button = container.querySelector(
      '[data-ds-name="SignalDot.threadAction"]',
    ) as HTMLButtonElement;
    fireEvent.click(button);

    expect(onNavigate).not.toHaveBeenCalled();
    expect(onThreadAction).toHaveBeenCalledTimes(1);
    expect(onThreadAction).toHaveBeenCalledWith(signal);
  });

  it("supports keyboard navigation from row focus", () => {
    const onNavigate = vi.fn();
    render(<MovingRow {...makeRow()} onNavigate={onNavigate} />);

    fireEvent.keyDown(screen.getByRole("link"), { key: "Enter" });

    expect(onNavigate).toHaveBeenCalledTimes(1);
  });

  it("emits ds-inspector attributes for design-system audit", () => {
    const { container } = render(<MovingRow {...makeRow()} onNavigate={vi.fn()} />);
    const row = container.querySelector('[data-ds-name="MovingRow"]');

    expect(row?.getAttribute("data-ds-tier")).toBe("pattern");
    expect(row?.getAttribute("data-ds-spec")).toBe("patterns/MovingRow.md");
  });

  it("does not emit inline style attributes", () => {
    const { container } = render(<MovingRow {...makeRow()} onNavigate={vi.fn()} />);

    expect(container.querySelector("[style]")).toBeNull();
  });
});

/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { WatchRow, type WatchRowProps } from "./WatchRow";
import type {
  InferredActionSelectorViewModel,
  TrustMixin,
} from "@/types/briefing";

const baseTrust: TrustMixin = {
  trustBand: "unscored",
};

function makeSelector(
  partial: Partial<InferredActionSelectorViewModel> = {},
): InferredActionSelectorViewModel {
  return {
    triggerLabel: "Suggested action",
    selectedOptionId: "snooze",
    options: [
      { id: "snooze", label: "Snooze until Q3 review" },
      { id: "restore", label: "Restore to today" },
      { id: "dismiss", label: "Dismiss", divider: true },
    ],
    ...partial,
  };
}

type SuggestedActionProps = Extract<
  WatchRowProps,
  { kind: "suggestedAction" }
>;
type OpenActionProps = Extract<WatchRowProps, { kind: "openAction" }>;
type ParkedProps = Extract<WatchRowProps, { kind: "parked" }>;
type AgingProps = Extract<WatchRowProps, { kind: "aging" }>;

function makeSuggestedActionRow(
  partial: Partial<SuggestedActionProps> = {},
): SuggestedActionProps {
  return {
    ...baseTrust,
    kind: "suggestedAction",
    actionId: "act_suggested",
    who: "Globex Inc",
    what: "Pushing intro to Q3; not dead.",
    selector: makeSelector(),
    ...partial,
  };
}

function makeOpenActionRow(
  partial: Partial<OpenActionProps> = {},
): OpenActionProps {
  return {
    ...baseTrust,
    kind: "openAction",
    actionId: "act_open",
    who: "Acme Corp",
    what: "Send revised pricing appendix.",
    checkButtonLabel: "Mark complete",
    ...partial,
  };
}

function makeParkedRow(partial: Partial<ParkedProps> = {}): ParkedProps {
  return {
    ...baseTrust,
    kind: "parked",
    who: "Internal",
    what: "New tier 3 deck circulating.",
    parkedLabel: "Parked",
    ...partial,
  };
}

function makeAgingRow(partial: Partial<AgingProps> = {}): AgingProps {
  return {
    ...baseTrust,
    kind: "aging",
    actionId: "act_aging",
    who: "Stark",
    what: "Old support thread, no movement.",
    ageLabel: "2w",
    since: "2026-04-22",
    options: [
      { id: "restore", label: "Restore" },
      { id: "archive", label: "Archive" },
    ],
    ...partial,
  };
}

const ALL_ROWS = [
  makeSuggestedActionRow(),
  makeOpenActionRow(),
  makeParkedRow(),
  makeAgingRow(),
];

function renderRow(row: WatchRowProps) {
  return render(<WatchRow {...row} />);
}

function readKindSpecificValue(row: WatchRowProps): string {
  switch (row.kind) {
    case "suggestedAction":
      return row.selector.triggerLabel;
    case "openAction":
      return row.checkButtonLabel;
    case "parked":
      return row.parkedLabel;
    case "aging":
      return row.options[0].id;
    default: {
      const exhaustive: never = row;
      return exhaustive;
    }
  }
}

describe("WatchRow", () => {
  it("renders the suggestedAction variant with an InferredActionSelector trigger", () => {
    renderRow(makeSuggestedActionRow());

    expect(
      screen.getByRole("button", { name: "Snooze until Q3 review" }),
    ).toBeInTheDocument();
  });

  it("renders the openAction variant with a circular check button", () => {
    renderRow(makeOpenActionRow());

    expect(
      screen.getByRole("button", { name: "Mark complete" }),
    ).toBeInTheDocument();
  });

  it("renders the parked variant as a passive label", () => {
    renderRow(makeParkedRow());

    expect(screen.getByText("Parked")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders the aging variant with restore and archive choices", () => {
    renderRow(makeAgingRow());

    expect(screen.getByText("2w")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Restore" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Archive" })).toBeInTheDocument();
  });

  it.each(ALL_ROWS)("renders who and what for kind=$kind", (row) => {
    renderRow(row);

    expect(screen.getByText(row.who)).toBeInTheDocument();
    expect(screen.getByText(row.what)).toBeInTheDocument();
  });

  it("fires onSelectorOption with action id and option id", () => {
    const onSelectorOption = vi.fn();
    renderRow(makeSuggestedActionRow({ onSelectorOption }));

    fireEvent.click(
      screen.getByRole("button", { name: "Snooze until Q3 review" }),
    );
    fireEvent.click(screen.getByRole("menuitem", { name: "Dismiss" }));

    expect(onSelectorOption).toHaveBeenCalledTimes(1);
    expect(onSelectorOption).toHaveBeenCalledWith("act_suggested", "dismiss");
  });

  it("openAction check button fires onMarkComplete with the action id", () => {
    const onMarkComplete = vi.fn();
    renderRow(makeOpenActionRow({ onMarkComplete }));

    fireEvent.click(screen.getByRole("button", { name: "Mark complete" }));

    expect(onMarkComplete).toHaveBeenCalledTimes(1);
    expect(onMarkComplete).toHaveBeenCalledWith("act_open");
  });

  it("parked label is non-interactive and does not expose affordance buttons", () => {
    const { container } = renderRow(makeParkedRow());

    const parkedLabel = container.querySelector(
      '[data-ds-name="WatchRow.parkedLabel"]',
    );
    expect(parkedLabel).not.toHaveAttribute("role", "button");
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("aging Restore fires onAgingAction with the action id and restore option id", () => {
    const onAgingAction = vi.fn();
    renderRow(makeAgingRow({ onAgingAction }));

    fireEvent.click(screen.getByRole("button", { name: "Restore" }));

    expect(onAgingAction).toHaveBeenCalledTimes(1);
    expect(onAgingAction).toHaveBeenCalledWith("act_aging", "restore");
  });

  it("aging Archive fires onAgingAction with the action id and archive option id", () => {
    const onAgingAction = vi.fn();
    renderRow(makeAgingRow({ onAgingAction }));

    fireEvent.click(screen.getByRole("button", { name: "Archive" }));

    expect(onAgingAction).toHaveBeenCalledTimes(1);
    expect(onAgingAction).toHaveBeenCalledWith("act_aging", "archive");
  });

  it("kind discriminator narrows the prop union correctly", () => {
    expect(readKindSpecificValue(makeSuggestedActionRow())).toBe("Suggested action");
    expect(readKindSpecificValue(makeOpenActionRow())).toBe("Mark complete");
    expect(readKindSpecificValue(makeParkedRow())).toBe("Parked");
    expect(readKindSpecificValue(makeAgingRow())).toBe("restore");
  });

  it("renders design-system inspector attributes", () => {
    const { container } = renderRow(makeOpenActionRow());

    const wrapper = container.querySelector('[data-ds-name="WatchRow"]');
    expect(wrapper).toHaveAttribute("data-ds-tier", "pattern");
    expect(wrapper).toHaveAttribute("data-ds-spec", "patterns/WatchRow.md");
    expect(wrapper).toHaveAttribute("data-kind", "openAction");
  });
});

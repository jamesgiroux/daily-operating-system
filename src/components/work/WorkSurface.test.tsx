/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CommitmentCard, NudgeLeaveAsIs, NudgeRow, SuggestionCard, WorkButton } from "./WorkSurface";

/**
 * Wave 0e / DOS-13: Work tab CTA wiring regression tests.
 *
 * These tests lock in that CommitmentCard and SuggestionCard actually
 * call back to the handlers the page wires up. Prior to Wave 0e the page
 * rendered literal `<WorkButton>Mark done</WorkButton>` elements with no
 * onClick — clicking was a no-op. The failure mode wasn't a type error,
 * it was silent. These assertions keep the wiring honest.
 */

describe("CommitmentCard", () => {
  it("invokes handlers threaded through the actions slot", () => {
    const onMarkDone = vi.fn();
    const onDismiss = vi.fn();
    render(
      <CommitmentCard
        headline="Ship SOC 2 evidence"
        owner="Alex"
        due="no date set"
        audience="customer"
        visibility="private"
        actions={
          <>
            <WorkButton kind="primary" onClick={onMarkDone}>Mark done</WorkButton>
            <WorkButton kind="muted" onClick={onDismiss}>Dismiss</WorkButton>
          </>
        }
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /mark done/i }));
    expect(onMarkDone).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: /dismiss/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("disables buttons while in-flight (loading copy visible)", () => {
    render(
      <CommitmentCard
        headline="Pending mutation"
        owner={null}
        due={null}
        audience="customer"
        visibility="private"
        actions={
          <>
            <WorkButton kind="primary" disabled>Marking done…</WorkButton>
            <WorkButton kind="muted" disabled>Dismiss</WorkButton>
          </>
        }
      />,
    );
    expect(screen.getByRole("button", { name: /marking done/i })).toBeDisabled();
  });
});

describe("SuggestionCard", () => {
  it("fires onAccept / onDismiss with loading state copy", () => {
    const onAccept = vi.fn();
    const onDismiss = vi.fn();

    const { rerender } = render(
      <SuggestionCard
        headline="Propose an EBR"
        rationale="Renewal is 60 days out."
        onAccept={onAccept}
        onDismiss={onDismiss}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /accept/i }));
    expect(onAccept).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: /dismiss/i }));
    expect(onDismiss).toHaveBeenCalledTimes(1);

    rerender(
      <SuggestionCard
        headline="Propose an EBR"
        rationale="Renewal is 60 days out."
        onAccept={onAccept}
        onDismiss={onDismiss}
        accepting
      />,
    );
    expect(screen.getByRole("button", { name: /accepting/i })).toBeDisabled();
  });
});

/**
 * Wave 0g Finding 1 (Option B): "Leave as-is" is rendered as italic
 * editorial prose, NOT as an interactive button. Doing nothing IS leaving
 * it as-is — a no-op button would imply action is required and violate
 * zero-guilt discipline. This test locks in the non-interactive rendering.
 */
describe("NudgeLeaveAsIs", () => {
  it("renders as non-interactive prose, not a button", () => {
    render(
      <NudgeRow
        headline="A commitment has been kept private"
        body="Dismiss if it's no longer live."
        actions={
          <>
            <WorkButton onClick={() => {}}>Dismiss</WorkButton>
            <NudgeLeaveAsIs />
          </>
        }
      />,
    );

    // Dismiss button is a real button.
    expect(screen.getByRole("button", { name: /dismiss/i })).toBeInTheDocument();

    // "Leave as-is" prose is NOT a button.
    const leaveAsIs = screen.getByText(/or leave as-is\./i);
    expect(leaveAsIs.tagName.toLowerCase()).toBe("span");
    expect(leaveAsIs).not.toHaveAttribute("role", "button");

    // No unnamed / empty-accessible-name buttons (i.e., no stray no-op CTAs).
    const buttons = screen.getAllByRole("button");
    expect(buttons).toHaveLength(1);
    expect(buttons[0]).toHaveAccessibleName(/dismiss/i);
  });

  it("accepts custom children while remaining non-interactive", () => {
    render(<NudgeLeaveAsIs>Or do nothing.</NudgeLeaveAsIs>);
    const prose = screen.getByText(/or do nothing/i);
    expect(prose.tagName.toLowerCase()).toBe("span");
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });
});

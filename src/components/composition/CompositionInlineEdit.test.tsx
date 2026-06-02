/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CompositionInlineEdit } from "@/components/composition/CompositionInlineEdit";
import type { EditRoute } from "@/services/composition/contracts";

const mockSubmit = vi.fn();

vi.mock("@/hooks/useIntelligenceCorrection", () => ({
  useIntelligenceCorrection: () => ({
    submitting: false,
    success: false,
    error: null,
    submit: mockSubmit,
    reset: vi.fn(),
  }),
}));

function route(overrides: Partial<EditRoute> = {}): EditRoute {
  return {
    field_path: "/sections/0/blocks/0/payload/text",
    role: "feedback_target",
    claim_refs: [{ claim_id: "claim-inline", claim_version: 2, field_path: "/text" }],
    feedback_allowed: true,
    refusal_reason: null,
    ...overrides,
  };
}

describe("CompositionInlineEdit", () => {
  beforeEach(() => {
    mockSubmit.mockReset();
    mockSubmit.mockResolvedValue(true);
  });

  it("submits claim-backed text corrections through the feedback path", async () => {
    render(
      <CompositionInlineEdit
        accountId="acct-fixture"
        route={route()}
        value="Original claim-backed summary"
        fallback={<p>Original claim-backed summary</p>}
      />,
    );

    fireEvent.click(screen.getByText("Original claim-backed summary"));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Corrected claim-backed summary" },
    });
    fireEvent.blur(screen.getByRole("textbox"));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith({
        entityId: "acct-fixture",
        entityType: "account",
        field: "composition:sections.0.blocks.0.payload.text",
        action: "corrected",
        itemKey: "claim-inline",
        currentValue: "Original claim-backed summary",
        correctedValue: "Corrected claim-backed summary",
        source: "composition_inline_edit",
      });
    });
  });

  it("does not expose inline editing for display-only or claimless routes", () => {
    const { rerender } = render(
      <CompositionInlineEdit
        accountId="acct-fixture"
        route={route({ role: "display_only", feedback_allowed: false })}
        value="Display-only summary"
        fallback={<p>Display-only summary</p>}
      />,
    );

    fireEvent.click(screen.getByText("Display-only summary"));
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(mockSubmit).not.toHaveBeenCalled();

    rerender(
      <CompositionInlineEdit
        accountId="acct-fixture"
        route={route({ claim_refs: [] })}
        value="Claimless summary"
        fallback={<p>Claimless summary</p>}
      />,
    );

    fireEvent.click(screen.getByText("Claimless summary"));
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    expect(mockSubmit).not.toHaveBeenCalled();
  });

  it("reverts the displayed text when correction submission fails", async () => {
    mockSubmit.mockResolvedValue(false);
    render(
      <CompositionInlineEdit
        accountId="acct-fixture"
        route={route()}
        value="Original claim-backed summary"
        fallback={<p>Original claim-backed summary</p>}
      />,
    );

    fireEvent.click(screen.getByText("Original claim-backed summary"));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Incorrect optimistic summary" },
    });
    fireEvent.blur(screen.getByRole("textbox"));

    await waitFor(() => expect(mockSubmit).toHaveBeenCalled());
    await waitFor(() => {
      expect(screen.getByText("Original claim-backed summary")).toBeInTheDocument();
    });
  });
});

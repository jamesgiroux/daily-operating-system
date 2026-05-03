/** @vitest-environment jsdom */

/**
 * IntelligenceCorrection tests —.
 *
 * Covers both variants:
 *   dismiss (default) — binary Yes / No → confirmed / dismissed
 *   correct           — three-state Yes / Partially / No → confirmed / annotated / corrected
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { IntelligenceCorrection } from "./IntelligenceCorrection";

// ── Mock the correction hook so tests run without a Tauri backend ──────────────

const mockSubmit = vi.fn();
const mockReset = vi.fn();

vi.mock("@/hooks/useIntelligenceCorrection", () => ({
  useIntelligenceCorrection: () => ({
    submit: mockSubmit,
    submitting: false,
    success: false,
    error: null,
    reset: mockReset,
  }),
}));

const DEFAULT_PROPS = {
  entityId: "acct-test",
  entityType: "account" as const,
  field: "state_of_play",
};

beforeEach(() => {
  mockSubmit.mockReset();
  mockReset.mockReset();
  mockSubmit.mockResolvedValue(true);
});

// ── dismiss variant (default) ─────────────────────────────────────────────────

describe("IntelligenceCorrection — dismiss variant (default)", () => {
  it('renders "Is this accurate?" with Yes and No buttons', () => {
    render(<IntelligenceCorrection {...DEFAULT_PROPS} />);
    expect(screen.getByText("Is this accurate?")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "No" })).toBeInTheDocument();
    // No "Partially" on dismiss variant
    expect(screen.queryByRole("button", { name: "Partially" })).not.toBeInTheDocument();
  });

  it("Yes submits confirmed action and moves to done state", async () => {
    render(<IntelligenceCorrection {...DEFAULT_PROPS} />);
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith(
        expect.objectContaining({ action: "confirmed" }),
      );
    });
    await waitFor(() => {
      expect(screen.getByText("Recorded.")).toBeInTheDocument();
    });
  });

  it("No submits dismissed action and moves to done state", async () => {
    render(<IntelligenceCorrection {...DEFAULT_PROPS} itemKey="some-key" />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith(
        expect.objectContaining({ action: "dismissed", itemKey: "some-key" }),
      );
    });
    await waitFor(() => {
      expect(screen.getByText("Recorded.")).toBeInTheDocument();
    });
  });

  it("Yes calls onConfirmed callback after success", async () => {
    const onConfirmed = vi.fn();
    render(<IntelligenceCorrection {...DEFAULT_PROPS} onConfirmed={onConfirmed} />);
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));
    await waitFor(() => expect(onConfirmed).toHaveBeenCalledTimes(1));
  });

  it("No calls onDismissed callback after success", async () => {
    const onDismissed = vi.fn();
    render(<IntelligenceCorrection {...DEFAULT_PROPS} onDismissed={onDismissed} />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    await waitFor(() => expect(onDismissed).toHaveBeenCalledTimes(1));
  });

  it("done state shows Undo button that resets to idle", async () => {
    render(<IntelligenceCorrection {...DEFAULT_PROPS} />);
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));
    await waitFor(() => screen.getByText("Recorded."));

    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(mockReset).toHaveBeenCalledTimes(1);
  });

  it("does not move to done when submit returns false", async () => {
    mockSubmit.mockResolvedValue(false);
    render(<IntelligenceCorrection {...DEFAULT_PROPS} />);
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));
    await waitFor(() => expect(mockSubmit).toHaveBeenCalled());
    // Still in idle state — "Is this accurate?" prompt still visible
    expect(screen.getByText("Is this accurate?")).toBeInTheDocument();
  });

  it("custom prompt label is rendered", () => {
    render(<IntelligenceCorrection {...DEFAULT_PROPS} prompt="Does this look right?" />);
    expect(screen.getByText("Does this look right?")).toBeInTheDocument();
  });
});

// ── correct variant ───────────────────────────────────────────────────────────

describe("IntelligenceCorrection — correct variant", () => {
  const correctProps = {
    ...DEFAULT_PROPS,
    variant: "correct" as const,
    currentValue: "Renewal confidence is high with expansion signals.",
  };

  it('renders "Is this accurate?" with Yes, Partially, and No buttons', () => {
    render(<IntelligenceCorrection {...correctProps} />);
    expect(screen.getByText("Is this accurate?")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Partially" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "No" })).toBeInTheDocument();
  });

  it("Yes submits confirmed action and shows done state", async () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith(
        expect.objectContaining({ action: "confirmed" }),
      );
    });
    await waitFor(() => screen.getByText("Recorded."));
  });

  it("Partially opens annotation textarea", () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "Partially" }));
    expect(screen.getByText("Add context — what did the AI miss?")).toBeInTheDocument();
    expect(screen.getByRole("textbox")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save note" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  it("Partially → annotation submit sends annotated action", async () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "Partially" }));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Renewal is actually at risk — champion went silent." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith(
        expect.objectContaining({
          action: "annotated",
          annotation: "Renewal is actually at risk — champion went silent.",
        }),
      );
    });
    await waitFor(() => screen.getByText("Recorded."));
  });

  it("Partially → Cancel returns to idle", () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "Partially" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Partially" })).toBeInTheDocument();
  });

  it("No opens inline editor prefilled with currentValue", () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    expect(screen.getByText("What should it say instead?")).toBeInTheDocument();
    const textarea = screen.getByRole("textbox") as HTMLTextAreaElement;
    expect(textarea.value).toBe("Renewal confidence is high with expansion signals.");
    expect(screen.getByRole("button", { name: "Save correction" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  it("No → editor submit sends corrected action with corrected value", async () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Renewal confidence is low — champion has disengaged." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save correction" }));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith(
        expect.objectContaining({
          action: "corrected",
          correctedValue: "Renewal confidence is low — champion has disengaged.",
        }),
      );
    });
    await waitFor(() => screen.getByText("Recorded."));
  });

  it("No → editor calls onCorrected callback with the new value", async () => {
    const onCorrected = vi.fn();
    render(<IntelligenceCorrection {...correctProps} onCorrected={onCorrected} />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "New corrected assessment text." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save correction" }));

    await waitFor(() =>
      expect(onCorrected).toHaveBeenCalledWith("New corrected assessment text."),
    );
  });

  it("No → Cancel returns to idle", () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "No" })).toBeInTheDocument();
  });

  it("Save note disabled when annotation is empty", () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "Partially" }));
    expect(screen.getByRole("button", { name: "Save note" })).toBeDisabled();
  });

  it("Save correction disabled when editor is empty", () => {
    render(<IntelligenceCorrection {...correctProps} currentValue="" />);
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    expect(screen.getByRole("button", { name: "Save correction" })).toBeDisabled();
  });

  it("opens editor with empty textarea when currentValue is null", () => {
    render(
      <IntelligenceCorrection {...DEFAULT_PROPS} variant="correct" currentValue={null} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    const textarea = screen.getByRole("textbox") as HTMLTextAreaElement;
    expect(textarea.value).toBe("");
  });

  it("done state Undo resets to idle in correct variant", async () => {
    render(<IntelligenceCorrection {...correctProps} />);
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));
    await waitFor(() => screen.getByText("Recorded."));

    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    expect(screen.getByRole("button", { name: "Partially" })).toBeInTheDocument();
    expect(mockReset).toHaveBeenCalledTimes(1);
  });
});

// ── Intelligence Loop wiring ───────────────────────────────────────────────────

describe("IntelligenceCorrection — Intelligence Loop signal verification", () => {
  it("confirmed action carries entityId, entityType, and field", async () => {
    render(
      <IntelligenceCorrection
        entityId="acct-loop-test"
        entityType="account"
        field="agreement_outlook"
        variant="correct"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Yes" }));
    await waitFor(() =>
      expect(mockSubmit).toHaveBeenCalledWith({
        entityId: "acct-loop-test",
        entityType: "account",
        field: "agreement_outlook",
        action: "confirmed",
      }),
    );
  });

  it("corrected action carries all required fields for Bayesian weight update", async () => {
    render(
      <IntelligenceCorrection
        entityId="acct-loop-test"
        entityType="account"
        field="health"
        variant="correct"
        currentValue="green"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "No" }));
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "yellow" } });
    fireEvent.click(screen.getByRole("button", { name: "Save correction" }));

    await waitFor(() =>
      expect(mockSubmit).toHaveBeenCalledWith({
        entityId: "acct-loop-test",
        entityType: "account",
        field: "health",
        action: "corrected",
        correctedValue: "yellow",
      }),
    );
  });

  it("annotated action carries annotation text for intel prompt threading", async () => {
    render(
      <IntelligenceCorrection
        entityId="acct-loop-test"
        entityType="account"
        field="state_of_play"
        variant="correct"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Partially" }));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "The risk is understated — procurement has paused." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));

    await waitFor(() =>
      expect(mockSubmit).toHaveBeenCalledWith({
        entityId: "acct-loop-test",
        entityType: "account",
        field: "state_of_play",
        action: "annotated",
        annotation: "The risk is understated — procurement has paused.",
      }),
    );
  });
});

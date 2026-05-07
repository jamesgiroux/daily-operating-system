/** @vitest-environment jsdom */

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { clearShowAllEvidenceStateForTests } from "@/lib/trust-band";
import { ContextEntryList } from "./ContextEntryList";

const handlers = {
  onUpdate: vi.fn(),
  onDelete: vi.fn(),
  onCreate: vi.fn(),
};

beforeEach(() => {
  clearShowAllEvidenceStateForTests();
  vi.clearAllMocks();
});

describe("ContextEntryList trust partitioning", () => {
  it("ContextEntryList_renders_caution_entries_in_collapsed_background", () => {
    render(
      <ContextEntryList
        entries={[
          {
            id: "entry-current",
            title: "Current Example note",
            content: "Current context",
            createdAt: "2026-05-01T12:00:00Z",
            trustBand: "likely_current",
          },
          {
            id: "entry-caution",
            title: "Older Example note",
            content: "Older context",
            createdAt: "2026-04-01T12:00:00Z",
            trustBand: "use_with_caution",
          },
        ]}
        surfaceId="context-test"
        {...handlers}
      />,
    );

    expect(screen.getByText("Current Example note")).toBeVisible();
    const background = screen.getByText("Background").closest("details");
    expect(background).not.toBeNull();
    expect(within(background!).getByText("Older Example note")).toBeInTheDocument();
    expect(within(background!).getByText("Use with caution")).toBeInTheDocument();
  });

  it("ContextEntryList_show_all_evidence_announces_low_confidence_entries", () => {
    render(
      <ContextEntryList
        entries={[
          {
            id: "entry-needs-verification",
            title: "Verify Example note",
            content: "Needs another source",
            createdAt: "2026-03-01T12:00:00Z",
            trustBand: "needs_verification",
          },
        ]}
        surfaceId="context-test"
        {...handlers}
      />,
    );

    expect(screen.getByText("No high-confidence current-state evidence since Mar 1.")).toBeVisible();
    expect(screen.getByText("Hiding low-confidence evidence")).toBeVisible();
    const button = screen.getByRole("button", { name: /show all evidence/i });
    expect(button).toHaveAttribute("aria-pressed", "false");
    expect(screen.queryByText("Verify Example note")).not.toBeInTheDocument();

    fireEvent.click(button);

    expect(button).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByText("Showing low-confidence evidence")).toBeVisible();
    expect(screen.getByText("Verify Example note")).toBeVisible();
    expect(screen.getByText("Needs verification")).toBeVisible();
  });

  it("ContextEntryList_unscored_legacy_entries_remain_visible", () => {
    render(
      <ContextEntryList
        entries={[
          {
            id: "entry-legacy",
            title: "Legacy Example note",
            content: "Legacy context remains visible",
            createdAt: "2026-05-01T12:00:00Z",
          },
        ]}
        surfaceId="context-test"
        {...handlers}
      />,
    );

    expect(screen.getByText("Legacy Example note")).toBeVisible();
    expect(screen.getByText("Unscored")).toBeVisible();
    expect(screen.queryByRole("button", { name: /show all evidence/i })).not.toBeInTheDocument();
  });

  it("ContextEntryList_routes_confidential_carriers_to_click_to_reveal_renderer", () => {
    render(
      <ContextEntryList
        entries={[
          {
            id: "entry-confidential",
            title: "Confidential Example note",
            content: {
              text: "Confidential claim hidden",
              policy: {
                kind: "redacted",
                sensitivity: "confidential",
                surface: "tauri_entity_detail",
                claimId: "claim-confidential-context",
                affordance: {
                  kind: "confidential_click_to_reveal",
                  claim_id: "claim-confidential-context",
                  label: "Confidential claim hidden",
                  audit_required: true,
                },
              },
            },
            createdAt: "2026-05-01T12:00:00Z",
            trustBand: "likely_current",
          },
        ]}
        surfaceId="context-test"
        {...handlers}
      />,
    );

    const affordance = screen.getByText("Confidential claim hidden").closest("[data-render-policy]");
    expect(affordance).toHaveAttribute("data-render-policy", "redacted");
    expect(affordance).toHaveAttribute("data-sensitivity", "confidential");
    expect(screen.getByRole("button", { name: "Reveal confidential claim" })).toBeVisible();
    expect(screen.queryByText("Confidential source text example.com")).not.toBeInTheDocument();
  });
});

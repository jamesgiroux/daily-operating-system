/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { UnifiedTimeline } from "./UnifiedTimeline";
import type { TimelineSource } from "@/lib/entity-types";
import type { RenderableClaimText } from "@/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);
const randomUUIDMock = vi.fn();

function confidentialClaim(
  claimId: string,
  label: string,
): RenderableClaimText {
  return {
    text: label,
    policy: {
      kind: "redacted",
      sensitivity: "confidential",
      surface: "tauri_entity_detail",
      claimId,
      affordance: {
        kind: "confidential_click_to_reveal",
        claim_id: claimId,
        label,
        audit_required: true,
      },
    },
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  randomUUIDMock.mockReset();
  randomUUIDMock.mockReturnValue("88888888-8888-4888-8888-888888888888");
  Object.defineProperty(globalThis, "crypto", {
    value: { randomUUID: randomUUIDMock },
    configurable: true,
  });
});

describe("UnifiedTimeline", () => {
  it("routes confidential context entry carriers through reveal affordances", async () => {
    const confidentialTitle = confidentialClaim(
      "claim-timeline-title",
      "Confidential timeline title hidden",
    );
    const confidentialContent = confidentialClaim(
      "claim-timeline-content",
      "Confidential timeline content hidden",
    );
    const sourceClaimText = "Private security blocker from DPO.";
    const data: TimelineSource = {
      recentMeetings: [],
      contextEntries: [
        {
          id: "context-confidential",
          entityType: "account",
          entityId: "account-1",
          title: confidentialTitle,
          content: confidentialContent,
          createdAt: "2026-05-01T12:00:00Z",
          updatedAt: "2026-05-01T12:00:00Z",
        },
      ],
    };
    invokeMock.mockResolvedValueOnce({
      text: sourceClaimText,
      policy: {
        kind: "render",
        sensitivity: "confidential",
        surface: "tauri_entity_detail",
        claimId: "claim-timeline-content",
      },
    } satisfies RenderableClaimText);

    render(<UnifiedTimeline data={data} />);

    expect(screen.queryByText(sourceClaimText)).not.toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Reveal confidential claim" })).toHaveLength(2);

    const contentAffordance = screen
      .getByText("Confidential timeline content hidden")
      .closest("[data-render-policy]");
    expect(contentAffordance).toHaveAttribute("data-render-policy", "redacted");

    fireEvent.click(
      within(contentAffordance as HTMLElement).getByRole("button", {
        name: "Reveal confidential claim",
      }),
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });
    expect(invokeMock).toHaveBeenCalledWith("reveal_sensitive_claim_text", {
      claimId: "claim-timeline-content",
      revealActionId: expect.any(String),
      surface: "tauri_entity_detail",
    });
    expect(await screen.findByText(sourceClaimText)).toBeInTheDocument();
  });
});

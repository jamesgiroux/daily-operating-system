/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ClaimTextRenderer } from "./ClaimTextRenderer";
import type { RenderableClaimText } from "@/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const invokeMock = vi.mocked(invoke);

const renderedClaim: RenderableClaimText = {
  text: "Expansion readiness is strong.",
  policy: {
    kind: "render",
    sensitivity: "internal",
    surface: "tauri_entity_detail",
    claimId: "claim-internal",
  },
};

const redactedClaim: RenderableClaimText = {
  text: "Confidential claim hidden",
  policy: {
    kind: "redacted",
    sensitivity: "confidential",
    surface: "tauri_entity_detail",
    claimId: "claim-confidential",
    affordance: {
      kind: "confidential_click_to_reveal",
      claim_id: "claim-confidential",
      label: "Confidential claim hidden",
      audit_required: true,
    },
  },
};

const droppedClaim: RenderableClaimText = {
  text: "",
  policy: {
    kind: "drop",
    sensitivity: "user_only",
    surface: "mcp_tool",
  },
};

beforeEach(() => {
  invokeMock.mockReset();
});

describe("ClaimTextRenderer", () => {
  it("renders unwrapped strings for legacy callers", () => {
    render(<ClaimTextRenderer value="Plain context text" />);
    expect(screen.getByText("Plain context text")).toBeInTheDocument();
  });

  it("renders claim text when policy allows render", () => {
    render(<ClaimTextRenderer value={renderedClaim} />);
    expect(screen.getByText("Expansion readiness is strong.")).toBeInTheDocument();
  });

  it("renders nothing for dropped claim text", () => {
    const { container } = render(<ClaimTextRenderer value={droppedClaim} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("hides confidential text until audited reveal succeeds", async () => {
    invokeMock.mockResolvedValueOnce({
      text: "Confidential renewal blocker.",
      policy: {
        kind: "render",
        sensitivity: "confidential",
        surface: "tauri_entity_detail",
        claimId: "claim-confidential",
      },
    } satisfies RenderableClaimText);

    render(<ClaimTextRenderer value={redactedClaim} surface="tauri_entity_detail" />);

    expect(screen.getByText("Confidential claim hidden")).toBeInTheDocument();
    expect(screen.queryByText("Confidential renewal blocker.")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Reveal confidential claim" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("reveal_sensitive_claim_text", {
        claimId: "claim-confidential",
        surface: "tauri_entity_detail",
      });
    });
    expect(await screen.findByText("Confidential renewal blocker.")).toBeInTheDocument();
  });
});

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
const randomUUIDMock = vi.fn();
const uuidV4Pattern = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const revealActionId = "11111111-1111-4111-8111-111111111111";

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
  randomUUIDMock.mockReset();
  randomUUIDMock.mockReturnValue(revealActionId);
  Object.defineProperty(globalThis, "crypto", {
    value: { randomUUID: randomUUIDMock },
    configurable: true,
  });
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
        revealActionId: expect.stringMatching(uuidV4Pattern),
        surface: "tauri_entity_detail",
      });
    });
    expect(await screen.findByText("Confidential renewal blocker.")).toBeInTheDocument();
  });

  it("guards rapid synchronous reveal clicks with one invoke", () => {
    invokeMock.mockReturnValue(
      new Promise<RenderableClaimText>(() => {
        // Keep the reveal in flight so the second click hits the same task tick.
      }),
    );

    render(
      <ClaimTextRenderer
        value={redactedClaim}
        surface="tauri_entity_detail"
      />,
    );

    const button = screen.getByRole("button", { name: "Reveal confidential claim" });
    fireEvent.click(button);
    fireEvent.click(button);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(randomUUIDMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("reveal_sensitive_claim_text", {
      claimId: "claim-confidential",
      revealActionId,
      surface: "tauri_entity_detail",
    });
  });

  it("clears revealed text when the incoming carrier identity changes", async () => {
    const reveal = vi.fn().mockResolvedValueOnce({
      text: "Previously revealed confidential payload.",
      policy: {
        kind: "render",
        sensitivity: "confidential",
        surface: "tauri_entity_detail",
        claimId: "claim-confidential",
      },
    } satisfies RenderableClaimText);
    const nextRedactedClaim: RenderableClaimText = {
      text: "New confidential claim hidden",
      policy: {
        ...redactedClaim.policy,
        claimId: "claim-confidential-next",
        affordance: {
          kind: "confidential_click_to_reveal",
          claim_id: "claim-confidential-next",
          label: "New confidential claim hidden",
          audit_required: true,
        },
      },
    };

    const { rerender } = render(
      <ClaimTextRenderer
        value={redactedClaim}
        surface="tauri_entity_detail"
        reveal={reveal}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Reveal confidential claim" }));

    expect(
      await screen.findByText("Previously revealed confidential payload."),
    ).toBeInTheDocument();

    rerender(
      <ClaimTextRenderer
        value={nextRedactedClaim}
        surface="tauri_entity_detail"
        reveal={reveal}
      />,
    );

    expect(screen.queryByText("Previously revealed confidential payload.")).not.toBeInTheDocument();
    expect(screen.getByText("New confidential claim hidden")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reveal confidential claim" })).toBeInTheDocument();
  });

  it("clears revealed text when the render surface changes", async () => {
    const reveal = vi.fn().mockResolvedValueOnce({
      text: "Previously revealed confidential payload.",
      policy: {
        kind: "render",
        sensitivity: "confidential",
        surface: "tauri_entity_detail",
        claimId: "claim-confidential",
      },
    } satisfies RenderableClaimText);

    const { rerender } = render(
      <ClaimTextRenderer
        value={redactedClaim}
        surface="tauri_entity_detail"
        reveal={reveal}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Reveal confidential claim" }));

    expect(
      await screen.findByText("Previously revealed confidential payload."),
    ).toBeInTheDocument();

    rerender(
      <ClaimTextRenderer
        value={redactedClaim}
        surface="tauri_report"
        reveal={reveal}
      />,
    );

    expect(screen.queryByText("Previously revealed confidential payload.")).not.toBeInTheDocument();
    expect(screen.getByText("Confidential claim hidden")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reveal confidential claim" })).toBeInTheDocument();
  });

  it("clears revealed text when equal carrier fields arrive on a new object reference", async () => {
    const reveal = vi.fn().mockResolvedValueOnce({
      text: "Previously revealed confidential payload.",
      policy: {
        kind: "render",
        sensitivity: "confidential",
        surface: "tauri_entity_detail",
        claimId: "claim-confidential",
      },
    } satisfies RenderableClaimText);
    const sameFieldCarrier: RenderableClaimText = {
      text: redactedClaim.text,
      policy: {
        ...redactedClaim.policy,
        affordance: {
          kind: "confidential_click_to_reveal",
          claim_id: "claim-confidential",
          label: "Confidential claim hidden",
          audit_required: true,
        },
      },
    };

    const { rerender } = render(
      <ClaimTextRenderer
        value={redactedClaim}
        surface="tauri_entity_detail"
        reveal={reveal}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Reveal confidential claim" }));

    expect(
      await screen.findByText("Previously revealed confidential payload."),
    ).toBeInTheDocument();

    rerender(
      <ClaimTextRenderer
        value={sameFieldCarrier}
        surface="tauri_entity_detail"
        reveal={reveal}
      />,
    );

    expect(screen.queryByText("Previously revealed confidential payload.")).not.toBeInTheDocument();
    expect(screen.getByText("Confidential claim hidden")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Reveal confidential claim" })).toBeInTheDocument();
  });
});

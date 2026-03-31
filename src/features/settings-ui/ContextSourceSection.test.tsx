/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ContextSourceSection from "@/features/settings-ui/ContextSourceSection";

const {
  invokeMock,
  listenMock,
  toastSuccessMock,
  toastErrorMock,
  toastWarningMock,
} = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  toastSuccessMock: vi.fn(),
  toastErrorMock: vi.fn(),
  toastWarningMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("sonner", () => ({
  toast: {
    success: toastSuccessMock,
    error: toastErrorMock,
    warning: toastWarningMock,
  },
}));

describe("ContextSourceSection", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastWarningMock.mockReset();
    listenMock.mockResolvedValue(() => {});
  });

  it("clears the reconnect warning after a successful Glean reconnect", async () => {
    let tokenHealthCalls = 0;

    invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      switch (command) {
        case "get_context_mode":
          return { mode: "Glean", endpoint: "https://acme.glean.com/mcp/default" };
        case "get_glean_auth_status":
          return { status: "authenticated", email: "user@acme.com" };
        case "get_glean_token_health":
          tokenHealthCalls += 1;
          return tokenHealthCalls === 1
            ? {
                connected: true,
                status: "expired",
                expiresAt: "2026-03-29T10:00:00Z",
                expiresInHours: -2,
              }
            : {
                connected: true,
                status: "healthy",
                expiresAt: "2026-03-30T10:00:00Z",
                expiresInHours: 22,
              };
        case "start_glean_auth":
          expect(args).toEqual({ endpoint: "https://acme.glean.com/mcp/default" });
          return { status: "authenticated", email: "user@acme.com" };
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<ContextSourceSection />);

    expect(
      await screen.findByText("Your Glean token has expired. Reconnect now to resume enrichment."),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Reconnect" }));

    await waitFor(() => {
      expect(screen.queryByText("Your Glean token has expired. Reconnect now to resume enrichment.")).not.toBeInTheDocument();
    });

    expect(toastSuccessMock).toHaveBeenCalledWith("Glean account connected");
    expect(tokenHealthCalls).toBe(2);
  });
});

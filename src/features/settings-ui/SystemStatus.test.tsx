/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AiBackgroundWorkSection } from "@/features/settings-ui/SystemStatus";

const { invokeMock, listenMock, getVersionMock, checkMock, relaunchMock, toastMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  getVersionMock: vi.fn(),
  checkMock: vi.fn(),
  relaunchMock: vi.fn(),
  toastMock: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: getVersionMock,
}));

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: checkMock,
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: relaunchMock,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: toastMock,
}));

vi.mock("@/hooks/useConnectivity", () => ({
  useConnectivity: () => ({ isOnline: true }),
}));

describe("AiBackgroundWorkSection", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    getVersionMock.mockReset();
    checkMock.mockReset();
    relaunchMock.mockReset();
    toastMock.success.mockReset();
    toastMock.error.mockReset();
    toastMock.warning.mockReset();
    listenMock.mockResolvedValue(() => {});
  });

  it("does not show the Google disconnected warning when Google auth is authenticated", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_config":
          return {
            google: {
              enabled: false,
              calendarPollIntervalMinutes: 5,
              emailPollIntervalMinutes: 15,
            },
            hygienePreMeetingHours: 12,
            aiModels: {
              synthesis: "sonnet",
              extraction: "sonnet",
              background: "haiku",
              mechanical: "haiku",
            },
          };
        case "get_ai_usage_diagnostics":
          return {
            backgroundPause: {
              paused: true,
              reason: "Paused background AI after 100% timeout rate across the last 20 background calls",
              rolling4hTokens: 0,
              timeoutRateLast20: 1,
            },
          };
        case "get_google_auth_status":
          return {
            status: "authenticated",
            email: "user@example.com",
          };
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<AiBackgroundWorkSection />);

    expect(await screen.findByText("Background AI Guard")).toBeInTheDocument();
    expect(screen.queryByText("Google is currently disconnected, so poll cadence settings are dormant.")).not.toBeInTheDocument();
  });
});

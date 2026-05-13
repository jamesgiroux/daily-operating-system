/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  AiBackgroundWorkSection,
  SurfaceRuntimeSection,
} from "@/features/settings-ui/SystemStatus";

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

describe("SurfaceRuntimeSection", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    getVersionMock.mockReset();
    checkMock.mockReset();
    relaunchMock.mockReset();
    toastMock.success.mockReset();
    toastMock.error.mockReset();
    toastMock.warning.mockReset();
  });

  it("shows runtime endpoint details and pairings", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          return {
            availability: "available",
            boundPort: 4387,
            endpointVersion: "v1",
            pairingString: "dos-pairing://stale-status-value",
          };
        case "list_surface_client_pairings":
          return [
            {
              surfaceClientId: "pairing-active",
              surfaceClientDisplayId: "Local client",
              siteBindingDigest: "site-digest",
              scopeDigest: "scope-digest",
              lifecycleState: "active",
              createdAt: "2026-05-12T10:00:00Z",
              lastUsedAt: "2026-05-12T10:30:00Z",
              expiresAt: null,
              revokedAt: null,
            },
          ];
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    expect(await screen.findByText("Available")).toBeInTheDocument();
    expect(screen.getByText("Port 4387 · v1")).toBeInTheDocument();
    expect(screen.getByText("Local client")).toBeInTheDocument();
    expect(screen.getByText(/Site site-digest · Scope scope-digest/)).toBeInTheDocument();
    expect(screen.queryByText("dos-pairing://stale-status-value")).not.toBeInTheDocument();
  });

  it("creates and displays a pairing string with its expiry after refresh succeeds", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          return {
            availability: "available",
            boundPort: 4387,
            endpointVersion: "v1",
          };
        case "list_surface_client_pairings":
          return [];
        case "create_surface_runtime_pairing_string":
          return {
            pairingString: "dos-pairing://example",
            expiresAt: "2026-05-12T11:00:00Z",
          };
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    const createButton = await screen.findByRole("button", { name: "Create Pairing String" });
    await waitFor(() => expect(createButton).toBeEnabled());
    fireEvent.click(createButton);

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("create_surface_runtime_pairing_string"));
    expect(await screen.findByText("dos-pairing://example")).toBeInTheDocument();
    expect(screen.getByText(/Expires/)).toHaveTextContent("May 12");
    expect(toastMock.success).toHaveBeenCalledWith("Pairing string created");
  });

  it("clears the created pairing string and skips the create success toast when refresh fails", async () => {
    let statusLoads = 0;

    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          statusLoads += 1;
          if (statusLoads === 1) {
            return {
              availability: "available",
              boundPort: 4387,
              endpointVersion: "v1",
            };
          }
          throw new Error("Refresh failed");
        case "list_surface_client_pairings":
          return [];
        case "create_surface_runtime_pairing_string":
          return {
            pairingString: "dos-pairing://example",
            expiresAt: "2026-05-12T11:00:00Z",
          };
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    const createButton = await screen.findByRole("button", { name: "Create Pairing String" });
    await waitFor(() => expect(createButton).toBeEnabled());
    fireEvent.click(createButton);

    await waitFor(() => expect(toastMock.error).toHaveBeenCalledWith("Failed to load surface runtime"));
    expect(toastMock.success).not.toHaveBeenCalledWith("Pairing string created");
    expect(screen.queryByText("dos-pairing://example")).not.toBeInTheDocument();
  });

  it("only enables revoke for active or suspended pairings", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          return {
            availability: "available",
            boundPort: 4387,
            endpointVersion: "v1",
          };
        case "list_surface_client_pairings":
          return [
            {
              surfaceClientId: "pairing-active",
              surfaceClientDisplayId: "Active client",
              siteBindingDigest: "active-site",
              scopeDigest: "active-scope",
              lifecycleState: "active",
              createdAt: "2026-05-12T10:00:00Z",
              lastUsedAt: null,
              expiresAt: null,
              revokedAt: null,
            },
            {
              surfaceClientId: "pairing-revoked",
              surfaceClientDisplayId: "Revoked client",
              siteBindingDigest: "revoked-site",
              scopeDigest: "revoked-scope",
              lifecycleState: "revoked",
              createdAt: "2026-05-12T09:00:00Z",
              lastUsedAt: null,
              expiresAt: null,
              revokedAt: "2026-05-12T11:00:00Z",
            },
          ];
        case "revoke_surface_client_pairing":
          return undefined;
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    expect(await screen.findByText("Active client")).toBeInTheDocument();
    const revokeButtons = screen.getAllByRole("button", { name: "Revoke" });
    expect(revokeButtons).toHaveLength(2);
    expect(revokeButtons[0]).toBeEnabled();
    expect(revokeButtons[1]).toBeDisabled();

    fireEvent.click(revokeButtons[0]);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("revoke_surface_client_pairing", {
        surfaceClientId: "pairing-active",
      });
    });
    expect(toastMock.success).toHaveBeenCalledWith("Pairing revoked");
  });

  it("clears a visible pairing string when revoke succeeds after refresh", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          return {
            availability: "available",
            boundPort: 4387,
            endpointVersion: "v1",
          };
        case "list_surface_client_pairings":
          return [
            {
              surfaceClientId: "pairing-active",
              surfaceClientDisplayId: "Active client",
              siteBindingDigest: "active-site",
              scopeDigest: "active-scope",
              lifecycleState: "active",
              createdAt: "2026-05-12T10:00:00Z",
              lastUsedAt: null,
              expiresAt: null,
              revokedAt: null,
            },
          ];
        case "create_surface_runtime_pairing_string":
          return {
            pairingString: "dos-pairing://example",
            expiresAt: "2026-05-12T11:00:00Z",
          };
        case "revoke_surface_client_pairing":
          return undefined;
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    const createButton = await screen.findByRole("button", { name: "Create Pairing String" });
    await waitFor(() => expect(createButton).toBeEnabled());
    fireEvent.click(createButton);
    expect(await screen.findByText("dos-pairing://example")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Revoke" }));

    await waitFor(() => expect(toastMock.success).toHaveBeenCalledWith("Pairing revoked"));
    expect(screen.queryByText("dos-pairing://example")).not.toBeInTheDocument();
  });

  it("skips the revoke success toast when post-revoke refresh fails", async () => {
    let statusLoads = 0;

    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          statusLoads += 1;
          if (statusLoads === 1) {
            return {
              availability: "available",
              boundPort: 4387,
              endpointVersion: "v1",
            };
          }
          throw new Error("Refresh failed");
        case "list_surface_client_pairings":
          return [
            {
              surfaceClientId: "pairing-active",
              surfaceClientDisplayId: "Active client",
              siteBindingDigest: "active-site",
              scopeDigest: "active-scope",
              lifecycleState: "active",
              createdAt: "2026-05-12T10:00:00Z",
              lastUsedAt: null,
              expiresAt: null,
              revokedAt: null,
            },
          ];
        case "revoke_surface_client_pairing":
          return undefined;
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    expect(await screen.findByText("Active client")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Revoke" }));

    await waitFor(() => expect(toastMock.error).toHaveBeenCalledWith("Failed to load surface runtime"));
    expect(toastMock.success).not.toHaveBeenCalledWith("Pairing revoked");
  });

  it("clears a visible pairing string when revoke fails before refresh", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case "get_surface_runtime_pairing_status":
          return {
            availability: "available",
            boundPort: 4387,
            endpointVersion: "v1",
          };
        case "list_surface_client_pairings":
          return [
            {
              surfaceClientId: "pairing-active",
              surfaceClientDisplayId: "Active client",
              siteBindingDigest: "active-site",
              scopeDigest: "active-scope",
              lifecycleState: "active",
              createdAt: "2026-05-12T10:00:00Z",
              lastUsedAt: null,
              expiresAt: null,
              revokedAt: null,
            },
          ];
        case "create_surface_runtime_pairing_string":
          return {
            pairingString: "dos-pairing://example",
            expiresAt: "2026-05-12T11:00:00Z",
          };
        case "revoke_surface_client_pairing":
          throw new Error("Revoke failed");
        default:
          throw new Error(`Unexpected invoke: ${command}`);
      }
    });

    render(<SurfaceRuntimeSection />);

    const createButton = await screen.findByRole("button", { name: "Create Pairing String" });
    await waitFor(() => expect(createButton).toBeEnabled());
    fireEvent.click(createButton);
    expect(await screen.findByText("dos-pairing://example")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Revoke" }));

    await waitFor(() => expect(toastMock.error).toHaveBeenCalledWith("Failed to revoke pairing"));
    expect(screen.queryByText("dos-pairing://example")).not.toBeInTheDocument();
  });
});

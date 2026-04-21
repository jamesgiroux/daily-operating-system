/** @vitest-environment jsdom */

/**
 * DOS-240 Wave 0f: UI-click integration test.
 *
 * Wave 0e-C wired `MeetingEntityChips` chip X to `dismiss_meeting_entity`,
 * but only backend-side unit tests existed. This test mounts the real
 * component, simulates a click on the chip's X button, and asserts the
 * expected Tauri command + args are invoked (and that the legacy
 * `remove_meeting_entity` command is NOT).
 */

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { LinkedEntity } from "@/types";

// ── Mocks ──────────────────────────────────────────────────────────────────────

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, to }: { children: React.ReactNode; to?: string }) => (
    <a href={String(to ?? "#")}>{children}</a>
  ),
}));

const toastErrorMock = vi.fn();
const toastSuccessMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

// Stub EntityPicker — it pulls in Tauri + router-heavy deps and is not
// what's under test here.
vi.mock("./entity-picker", () => ({
  EntityPicker: () => <div data-testid="entity-picker-stub" />,
}));

// ── Test helpers ───────────────────────────────────────────────────────────────

function makeLinkedEntity(overrides: Partial<LinkedEntity> = {}): LinkedEntity {
  return {
    id: "acct-42",
    name: "Acme Corp",
    entityType: "account",
    isPrimary: true,
    confidence: 0.95,
    ...overrides,
  };
}

// Dynamic import after vi.mock calls so the module picks up mocked deps.
async function loadComponent() {
  const mod = await import("./meeting-entity-chips");
  return mod.MeetingEntityChips;
}

const baseMeetingProps = {
  meetingId: "mtg-1",
  meetingTitle: "Quarterly review",
  meetingStartTime: "2026-04-18T15:00:00Z",
  meetingType: "external",
};

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("MeetingEntityChips — chip X dismissal (DOS-240)", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
  });

  it("clicking the chip X invokes dismiss_meeting_entity with camelCase args", async () => {
    const MeetingEntityChips = await loadComponent();
    invokeMock.mockResolvedValue(undefined);

    const onEntitiesChanged = vi.fn();
    const entity = makeLinkedEntity();

    render(
      <MeetingEntityChips
        {...baseMeetingProps}
        linkedEntities={[entity]}
        onEntitiesChanged={onEntitiesChanged}
      />,
    );

    // Chip is rendered
    expect(screen.getByText("Acme Corp")).toBeInTheDocument();

    // The X button is the only <button> inside the chip (EntityPicker is
    // stubbed to a div), so it is uniquely identifiable.
    const removeButton = screen
      .getByText("Acme Corp")
      .closest("span")
      ?.querySelector("button");
    expect(removeButton).toBeTruthy();

    fireEvent.click(removeButton!);

    // Wait for the async invoke() call to settle.
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(1);
    });

    // Correct command + camelCase Tauri arg contract.
    expect(invokeMock).toHaveBeenCalledWith("dismiss_meeting_entity", {
      meetingId: "mtg-1",
      entityId: "acct-42",
      entityType: "account",
    });

    // Legacy command MUST NOT be invoked.
    const legacyCalls = invokeMock.mock.calls.filter(
      (call) => call[0] === "remove_meeting_entity",
    );
    expect(legacyCalls).toHaveLength(0);

    // User-visible feedback: optimistic removal — chip disappears.
    await waitFor(() => {
      expect(screen.queryByText("Acme Corp")).not.toBeInTheDocument();
    });

    // Parent is notified so it can refetch.
    await waitFor(() => {
      expect(onEntitiesChanged).toHaveBeenCalled();
    });

    // No error toast on happy path.
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("surfaces an error toast and restores the chip when dismiss invoke rejects", async () => {
    const MeetingEntityChips = await loadComponent();
    invokeMock.mockRejectedValue(new Error("backend boom"));

    const entity = makeLinkedEntity({ id: "acct-err", name: "Error Co" });

    render(
      <MeetingEntityChips
        {...baseMeetingProps}
        linkedEntities={[entity]}
      />,
    );

    const removeButton = screen
      .getByText("Error Co")
      .closest("span")
      ?.querySelector("button");
    expect(removeButton).toBeTruthy();

    fireEvent.click(removeButton!);

    // Error toast is surfaced — failure is NOT silent.
    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledTimes(1);
    });
    expect(toastErrorMock.mock.calls[0][0]).toMatch(/unlink/i);

    // Optimistic rollback: the chip reappears so the user sees the
    // entity is still linked.
    await waitFor(() => {
      expect(screen.getByText("Error Co")).toBeInTheDocument();
    });

    // Still only the dismiss command was attempted — no fallback to the
    // legacy command.
    expect(invokeMock).toHaveBeenCalledWith(
      "dismiss_meeting_entity",
      expect.objectContaining({
        meetingId: "mtg-1",
        entityId: "acct-err",
        entityType: "account",
      }),
    );
    const legacyCalls = invokeMock.mock.calls.filter(
      (call) => call[0] === "remove_meeting_entity",
    );
    expect(legacyCalls).toHaveLength(0);
  });
});

/** @vitest-environment jsdom */

import { act, cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import EmailsPage from "./EmailsPage";
import type { EmailBriefingData, EmailSyncStats } from "@/types";

const { invokeMock, eventHandlers, navigateMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  eventHandlers: new Map<string, () => void>(),
  navigateMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigateMock,
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn(), warning: vi.fn() },
}));

vi.mock("@/hooks/useMagazineShell", () => ({
  useRegisterMagazineShell: vi.fn(),
}));

vi.mock("@/hooks/usePersonality", () => ({
  usePersonality: () => ({ personality: "default" }),
}));

vi.mock("@/hooks/useGoogleAuth", () => ({
  useGoogleAuth: () => ({
    status: { status: "authenticated", email: "user@example.com" },
  }),
}));

vi.mock("@/hooks/useTauriEvent", () => ({
  useTauriEvent: (event: string, handler: () => void) => {
    eventHandlers.set(event, handler);
  },
}));

const emptyBriefing: EmailBriefingData = {
  highPriority: [],
  mediumPriority: [],
  lowPriority: [],
  entityThreads: [],
  stats: {
    total: 0,
    highCount: 0,
    mediumCount: 0,
    lowCount: 0,
    needsAction: 0,
  },
  hasEnrichment: true,
};

const syncStats: EmailSyncStats = {
  lastFetchAt: null,
  lastSuccessfulFetchAt: null,
  total: 0,
  enriched: 0,
  pending: 0,
  failed: 0,
  permanentlyFailed: 0,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

function installResolvedInvoke() {
  invokeMock.mockImplementation((command: string) => {
    switch (command) {
      case "get_emails_enriched":
        return Promise.resolve(emptyBriefing);
      case "list_dismissed_email_items":
        return Promise.resolve([]);
      case "get_email_sync_status":
        return Promise.resolve(syncStats);
      case "sync_email_inbox_presence":
        return Promise.resolve(true);
      default:
        return Promise.resolve(null);
    }
  });
}

function getEmailReadCount() {
  return invokeMock.mock.calls.filter(([command]) => command === "get_emails_enriched").length;
}

async function flushMicrotasks() {
  await act(async () => {
    await Promise.resolve();
  });
}

describe("EmailsPage refresh behavior", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    invokeMock.mockReset();
    navigateMock.mockReset();
    eventHandlers.clear();
    installResolvedInvoke();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("coalesces backend email event bursts into one silent foreground read", async () => {
    render(<EmailsPage />);
    await flushMicrotasks();
    expect(getEmailReadCount()).toBe(1);

    act(() => {
      eventHandlers.get("emails-updated")?.();
      eventHandlers.get("workflow-completed")?.();
      eventHandlers.get("email-enrichment-progress")?.();
      vi.advanceTimersByTime(749);
    });
    expect(getEmailReadCount()).toBe(1);

    await act(async () => {
      vi.advanceTimersByTime(1);
      await Promise.resolve();
    });

    await flushMicrotasks();
    expect(getEmailReadCount()).toBe(2);
  });

  it("queues one silent refresh when a backend event fires during an active load", async () => {
    const firstRead = deferred<EmailBriefingData>();
    let emailReadCalls = 0;
    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case "get_emails_enriched":
          emailReadCalls += 1;
          return emailReadCalls === 1 ? firstRead.promise : Promise.resolve(emptyBriefing);
        case "list_dismissed_email_items":
          return Promise.resolve([]);
        case "get_email_sync_status":
          return Promise.resolve(syncStats);
        case "sync_email_inbox_presence":
          return Promise.resolve(true);
        default:
          return Promise.resolve(null);
      }
    });

    render(<EmailsPage />);
    expect(getEmailReadCount()).toBe(1);

    act(() => {
      eventHandlers.get("emails-updated")?.();
      vi.advanceTimersByTime(750);
    });
    expect(getEmailReadCount()).toBe(1);

    await act(async () => {
      firstRead.resolve(emptyBriefing);
      await firstRead.promise;
      await Promise.resolve();
    });
    await flushMicrotasks();

    await act(async () => {
      vi.runOnlyPendingTimers();
      await Promise.resolve();
    });

    await flushMicrotasks();
    expect(getEmailReadCount()).toBe(2);
  });
});

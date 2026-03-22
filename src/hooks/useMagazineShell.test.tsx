/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  MagazineShellContext,
  useFolioVolatile,
  useMagazineShellProvider,
  useUpdateFolioVolatile,
} from "@/hooks/useMagazineShell";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@/hooks/useTauriEvent", () => ({
  useTauriEvent: () => {},
}));

vi.mock("@/hooks/useAppState", () => ({
  useAppState: () => ({
    appState: { demoModeActive: false },
    clearDemo: vi.fn(),
  }),
}));

function ShellHarness({
  accountId,
  onReport,
}: {
  accountId: string;
  onReport: (accountId: string) => void;
}) {
  const shell = useMagazineShellProvider();

  return (
    <MagazineShellContext.Provider value={shell}>
      <VolatilePage accountId={accountId} onReport={onReport} />
      <FolioConsumer />
    </MagazineShellContext.Provider>
  );
}

function FolioConsumer() {
  const volatile = useFolioVolatile();
  return <div data-testid="folio-bar-actions">{volatile.folioActions}</div>;
}

function VolatilePage({
  accountId,
  onReport,
}: {
  accountId: string;
  onReport: (accountId: string) => void;
}) {
  useUpdateFolioVolatile(
    {
      folioActions: (
        <button type="button" onClick={() => onReport(accountId)}>
          Open report for {accountId}
        </button>
      ),
    },
    accountId,
  );

  return null;
}

describe("useMagazineShell volatile folio state", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ entityMode: "account" });
  });

  it("updates folio actions for same-route entity navigation", () => {
    const onReport = vi.fn();
    const { rerender } = render(
      <ShellHarness accountId="acct-001" onReport={onReport} />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Open report for acct-001" }),
    );
    expect(onReport).toHaveBeenLastCalledWith("acct-001");

    rerender(<ShellHarness accountId="acct-002" onReport={onReport} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Open report for acct-002" }),
    );
    expect(onReport).toHaveBeenLastCalledWith("acct-002");
  });
});

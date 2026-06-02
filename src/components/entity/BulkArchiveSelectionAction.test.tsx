/** @vitest-environment jsdom */

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  BulkArchiveSelectionAction,
  type BulkArchivePreview,
  type BulkArchiveResult,
} from "./BulkArchiveSelectionAction";

const { invokeMock, toastMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  toastMock: {
    error: vi.fn(),
    success: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("sonner", () => ({
  toast: toastMock,
}));

const preview: BulkArchivePreview = {
  entityType: "project",
  requestedIds: ["parent-project", "child-project"],
  rootIds: ["parent-project"],
  changedIds: ["parent-project", "child-project"],
  selectedIds: ["parent-project", "child-project"],
  cascadedChildIds: ["child-project"],
  coveredChildIds: ["child-project"],
  notFoundIds: [],
  alreadyArchivedIds: [],
  totalChangedCount: 2,
  directCascadeCount: 1,
  planId: "bulk-archive-plan",
  planFingerprint: "bulk-archive:v1:fingerprint",
  expiresAt: "2026-06-02T12:00:00Z",
};

function renderAction({
  onArchived = vi.fn(),
  onClearSelection = vi.fn(),
}: {
  onArchived?: (result: BulkArchiveResult) => void | Promise<void>;
  onClearSelection?: () => void;
} = {}) {
  render(
    <BulkArchiveSelectionAction
      selectedIds={["parent-project", "child-project"]}
      entityLabel="project"
      entityPluralLabel="projects"
      previewCommand="preview_bulk_archive_projects"
      executeCommand="bulk_archive_projects"
      onArchived={onArchived}
      onClearSelection={onClearSelection}
    />,
  );
  return { onArchived, onClearSelection };
}

describe("BulkArchiveSelectionAction", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    toastMock.error.mockReset();
    toastMock.success.mockReset();
    toastMock.warning.mockReset();
  });

  it("executes archive against the preview plan binding", async () => {
    const result: BulkArchiveResult = {
      status: "succeeded",
      preview,
      changedIds: preview.changedIds,
      alreadyArchivedIds: [],
      notFoundIds: [],
      itemResults: [],
    };
    invokeMock.mockImplementation((command: string) => {
      if (command === "preview_bulk_archive_projects") return Promise.resolve(preview);
      if (command === "bulk_archive_projects") return Promise.resolve(result);
      return Promise.resolve(null);
    });
    const { onArchived, onClearSelection } = renderAction();

    fireEvent.click(screen.getByRole("button", { name: "Archive 2 selected projects" }));
    expect(await screen.findByText(/Archive 2 projects, including 1 descendant row/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^Archive$/ }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("bulk_archive_projects", {
        ids: preview.requestedIds,
        planId: preview.planId,
        planFingerprint: preview.planFingerprint,
      });
    });
    expect(onArchived).toHaveBeenCalledWith(result);
    expect(onClearSelection).toHaveBeenCalledTimes(1);
    expect(toastMock.success).toHaveBeenCalledWith("Archived 2 projects");
  });

  it("keeps selection when the preview is stale", async () => {
    const staleResult: BulkArchiveResult = {
      status: "preview_stale",
      preview,
      changedIds: [],
      alreadyArchivedIds: [],
      notFoundIds: [],
      itemResults: [],
    };
    invokeMock.mockImplementation((command: string) => {
      if (command === "preview_bulk_archive_projects") return Promise.resolve(preview);
      if (command === "bulk_archive_projects") return Promise.resolve(staleResult);
      return Promise.resolve(null);
    });
    const { onArchived, onClearSelection } = renderAction();

    fireEvent.click(screen.getByRole("button", { name: "Archive 2 selected projects" }));
    await screen.findByText(/Archive 2 projects/);
    fireEvent.click(screen.getByRole("button", { name: /^Archive$/ }));

    await waitFor(() => {
      expect(toastMock.warning).toHaveBeenCalledWith("Archive preview changed. Review again before archiving.");
    });
    expect(onArchived).not.toHaveBeenCalled();
    expect(onClearSelection).not.toHaveBeenCalled();
  });
});

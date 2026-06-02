/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ProjectsPage from "./ProjectsPage";
import type { ProjectListItem } from "@/types";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, to }: { children: ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
}));

const parentProject: ProjectListItem = {
  id: "parent-project",
  name: "Parent Project",
  status: "active",
  openActionCount: 0,
  archived: false,
  childCount: 1,
  isParent: true,
};

const childProject: ProjectListItem = {
  id: "child-project",
  name: "Child Project",
  status: "on_hold",
  openActionCount: 0,
  archived: false,
  parentId: "parent-project",
  parentName: "Parent Project",
  childCount: 0,
  isParent: false,
};

const peerProject: ProjectListItem = {
  id: "peer-project",
  name: "Peer Project",
  status: "completed",
  openActionCount: 2,
  archived: false,
  childCount: 0,
  isParent: false,
};

describe("ProjectsPage selection", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === "get_projects_list") {
        return Promise.resolve([parentProject, peerProject]);
      }
      if (command === "get_child_projects_list" && args?.parentId === "parent-project") {
        return Promise.resolve([childProject]);
      }
      if (command === "get_archived_projects") {
        return Promise.resolve([
          {
            id: "archived-project",
            name: "Archived Project",
            status: "completed",
            openActionCount: 0,
            archived: true,
            childCount: 0,
            isParent: false,
          },
        ]);
      }
      return Promise.resolve(null);
    });
  });

  it("selects active tree rows and clears selection in archived mode", async () => {
    render(<ProjectsPage />);

    const parentCheckbox = await screen.findByRole("checkbox", { name: "Select Parent Project" });
    expect(await screen.findByRole("checkbox", { name: "Select Child Project" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Select Peer Project" })).toBeInTheDocument();

    fireEvent.click(parentCheckbox);
    expect(screen.getByRole("status")).toHaveTextContent("1 selected");

    fireEvent.click(screen.getByRole("button", { name: /archived/i }));
    expect(await screen.findByText("Archived Project")).toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Select Archived Project" })).not.toBeInTheDocument();
  });
});

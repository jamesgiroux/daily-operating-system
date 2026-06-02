/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PeoplePage from "./PeoplePage";
import type { PersonListItem } from "@/types";

const invokeMock = vi.fn();
const navigateMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, to }: { children: ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
  useNavigate: () => navigateMock,
  useSearch: () => ({}),
}));

const externalPerson: PersonListItem = {
  id: "external-person",
  email: "external@example.com",
  name: "External Person",
  organization: "Example Account",
  role: "Champion",
  relationship: "external",
  meetingCount: 3,
  updatedAt: "2026-06-01T00:00:00Z",
  archived: false,
  temperature: "hot",
  trend: "steady",
};

const internalPerson: PersonListItem = {
  id: "internal-person",
  email: "internal@example.com",
  name: "Internal Person",
  organization: "DailyOS",
  role: "CSM",
  relationship: "internal",
  meetingCount: 4,
  updatedAt: "2026-06-01T00:00:00Z",
  archived: false,
  temperature: "warm",
  trend: "steady",
};

describe("PeoplePage selection", () => {
  beforeEach(() => {
    navigateMock.mockReset();
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_people") return Promise.resolve([externalPerson, internalPerson]);
      if (command === "get_archived_people") return Promise.resolve([]);
      if (command === "get_duplicate_people") return Promise.resolve([]);
      return Promise.resolve(null);
    });
  });

  it("preserves selected active people when relationship tabs hide them", async () => {
    render(<PeoplePage />);

    const internalCheckbox = await screen.findByRole("checkbox", { name: "Select Internal Person" });
    expect(screen.getByRole("checkbox", { name: "Select External Person" })).toBeInTheDocument();

    fireEvent.click(internalCheckbox);
    expect(screen.getByRole("status")).toHaveTextContent("1 selected");

    fireEvent.click(screen.getByRole("button", { name: "external" }));

    expect(screen.queryByRole("checkbox", { name: "Select Internal Person" })).not.toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Select External Person" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("1 selected");
  });
});

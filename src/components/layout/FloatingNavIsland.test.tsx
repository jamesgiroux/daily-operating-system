/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { FloatingNavIsland } from "./FloatingNavIsland";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

describe("FloatingNavIsland", () => {
  it("omits the deprecated week navigation affordance", () => {
    const deprecatedWeekLabel = ["This", "Week"].join(" ");

    render(
      <FloatingNavIsland
        activePage="today"
        onHome={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "Today" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Mail" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: deprecatedWeekLabel })).toBeNull();
  });
});

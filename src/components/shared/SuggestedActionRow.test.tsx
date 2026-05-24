/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SuggestedActionRow } from "./SuggestedActionRow";

describe("SuggestedActionRow", () => {
  it("renders trust and source count for claim-backed suggestions", () => {
    render(
      <SuggestedActionRow
        action={{
          id: "suggestion-1",
          title: "Follow up on renewal plan",
          priority: 2,
          sourceLabel: "Meeting",
          accountName: "Example Account",
          trustBand: "use_with_caution",
          commitmentSourceCount: 2,
        }}
        onAccept={vi.fn()}
        onReject={vi.fn()}
      />,
    );

    expect(screen.getByRole("img", { name: /trust band: use with caution/i })).toBeInTheDocument();
    expect(screen.getByText(/2 sources/)).toBeInTheDocument();
  });
});

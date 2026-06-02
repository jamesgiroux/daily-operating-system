/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EntitySelectionBar } from "./EntityListShell";

describe("EntitySelectionBar", () => {
  it("stays hidden until rows are selected", () => {
    const { container } = render(
      <EntitySelectionBar
        selectedCount={0}
        visibleCount={2}
        onSelectVisible={vi.fn()}
        onClear={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("renders accessible selection actions", () => {
    const onSelectVisible = vi.fn();
    const onClear = vi.fn();

    render(
      <EntitySelectionBar
        selectedCount={2}
        visibleCount={4}
        onSelectVisible={onSelectVisible}
        onClear={onClear}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("2 selected");

    fireEvent.click(screen.getByRole("button", { name: "Select 4 visible rows" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear selected rows" }));

    expect(onSelectVisible).toHaveBeenCalledTimes(1);
    expect(onClear).toHaveBeenCalledTimes(1);
  });
});

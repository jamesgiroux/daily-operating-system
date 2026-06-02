/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  flattenEntityTreeIds,
  useEntityListSelection,
} from "./useEntityListSelection";

function SelectionHarness({
  visibleIds,
  activeIds = visibleIds,
}: {
  visibleIds: string[];
  activeIds?: string[];
}) {
  const selection = useEntityListSelection({ visibleIds, activeIds });

  return (
    <div>
      <output data-testid="selected">{selection.selectedIds.join(",")}</output>
      {activeIds.map((id) => (
        <button
          key={id}
          type="button"
          onClick={(event) => selection.toggle(id, { shiftKey: event.shiftKey })}
        >
          Toggle {id}
        </button>
      ))}
      <button type="button" onClick={selection.selectVisible}>
        Select visible
      </button>
      <button type="button" onClick={selection.clear}>
        Clear
      </button>
      <button type="button" onClick={() => selection.pruneTo(["a", "b"])}>
        Prune to a-b
      </button>
    </div>
  );
}

describe("useEntityListSelection", () => {
  it("toggles individual ids, selects visible ids, and clears", () => {
    render(<SelectionHarness visibleIds={["a", "b"]} activeIds={["a", "b", "c"]} />);

    fireEvent.click(screen.getByRole("button", { name: "Toggle c" }));
    expect(screen.getByTestId("selected")).toHaveTextContent("c");

    fireEvent.click(screen.getByRole("button", { name: "Select visible" }));
    expect(screen.getByTestId("selected")).toHaveTextContent("a,b,c");

    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(screen.getByTestId("selected")).toHaveTextContent("");
  });

  it("uses visible row order for shift range selection", () => {
    render(<SelectionHarness visibleIds={["a", "b", "c", "d"]} />);

    fireEvent.click(screen.getByRole("button", { name: "Toggle b" }));
    fireEvent.click(screen.getByRole("button", { name: "Toggle d" }), { shiftKey: true });

    expect(screen.getByTestId("selected")).toHaveTextContent("b,c,d");
  });

  it("prunes stale ids against the active list without clearing hidden active selections", () => {
    const { rerender } = render(
      <SelectionHarness visibleIds={["a", "b"]} activeIds={["a", "b", "c"]} />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Toggle c" }));
    expect(screen.getByTestId("selected")).toHaveTextContent("c");

    rerender(<SelectionHarness visibleIds={["a"]} activeIds={["a", "b", "c"]} />);
    expect(screen.getByTestId("selected")).toHaveTextContent("c");

    rerender(<SelectionHarness visibleIds={["a"]} activeIds={["a", "b"]} />);
    expect(screen.getByTestId("selected")).toHaveTextContent("");
  });

  it("flattens entity trees in rendered visual order", () => {
    const roots = [
      { id: "parent", isParent: true },
      { id: "peer" },
    ];
    const children = {
      parent: [{ id: "child" }],
    };

    expect(flattenEntityTreeIds(roots, children, {
      expandedOnly: true,
      expandedParents: new Set(["parent"]),
    })).toEqual(["parent", "child", "peer"]);

    expect(flattenEntityTreeIds(roots, children, {
      expandedOnly: true,
      expandedParents: new Set(),
    })).toEqual(["parent", "peer"]);
  });
});

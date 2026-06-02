/** @vitest-environment jsdom */

import { fireEvent, render, screen } from "@testing-library/react";
import { type ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { EntityRow } from "./EntityRow";

vi.mock("@tanstack/react-router", () => ({
  Link: ({ children, to }: { children: ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
}));

describe("EntityRow", () => {
  it("renders selection, navigation, and row controls as separate focus targets", () => {
    const onSelect = vi.fn();
    const onExpand = vi.fn();

    render(
      <EntityRow
        to="/accounts/$accountId"
        params={{ accountId: "acct-1" }}
        name="Example Account"
        showBorder
        selection={{
          selected: false,
          label: "Select Example Account",
          onChange: onSelect,
        }}
        controls={(
          <button type="button" onClick={onExpand}>
            Expand
          </button>
        )}
      />,
    );

    const checkbox = screen.getByRole("checkbox", { name: "Select Example Account" });
    const link = screen.getByRole("link", { name: "Example Account" });
    const expand = screen.getByRole("button", { name: "Expand" });

    expect(link).not.toContainElement(checkbox);
    expect(link).not.toContainElement(expand);

    fireEvent.click(checkbox);
    expect(onSelect).toHaveBeenCalledWith({ shiftKey: false });
    expect(onExpand).not.toHaveBeenCalled();

    fireEvent.click(expand);
    expect(onExpand).toHaveBeenCalledTimes(1);
  });

  it("passes shift-key checkbox interaction to the selection handler", () => {
    const onSelect = vi.fn();

    render(
      <EntityRow
        to="/people/$personId"
        params={{ personId: "person-1" }}
        name="Example Person"
        showBorder={false}
        selection={{
          selected: true,
          label: "Select Example Person",
          onChange: onSelect,
        }}
      />,
    );

    const checkbox = screen.getByRole("checkbox", { name: "Select Example Person" });
    expect(checkbox).toBeChecked();

    fireEvent.click(checkbox, { shiftKey: true });
    expect(onSelect).toHaveBeenCalledWith({ shiftKey: true });
  });
});

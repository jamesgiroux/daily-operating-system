/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TrustBandIndicator } from "./TrustBandIndicator";
import type { TrustBandWire } from "@/lib/trust-band";

const visibleBands: Array<{ band: TrustBandWire; label: string }> = [
  { band: "use_with_caution", label: "Use with caution" },
  { band: "needs_verification", label: "Needs verification" },
  { band: "unscored", label: "Unscored" },
];

describe("TrustBandIndicator", () => {
  it("trustBandIndicator_renders_nothing_for_likely_current", () => {
    const { container } = render(<TrustBandIndicator band="likely_current" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("trustBandIndicator_renders_open_circle_with_accessible_label", () => {
    render(<TrustBandIndicator band="use_with_caution" />);

    const indicator = screen.getByRole("img", { name: /Use with caution/ });
    expect(indicator).toBeVisible();
    expect(indicator).toHaveAttribute("data-band", "use_with_caution");
  });

  it("trustBandIndicator_carries_distinct_aria_label_per_band", () => {
    for (const { band, label } of visibleBands) {
      const { unmount } = render(<TrustBandIndicator band={band} />);
      expect(
        screen.getByRole("img", { name: new RegExp(label) }),
      ).toBeInTheDocument();
      unmount();
    }
  });

  it("trustBandIndicator_does_not_render_visible_label_text", () => {
    render(<TrustBandIndicator band="needs_verification" />);
    // Label is in tooltip (Radix Portal, hidden until hover/focus); not in default DOM.
    expect(screen.queryByText("Needs verification")).not.toBeInTheDocument();
  });
});

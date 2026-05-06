/** @vitest-environment jsdom */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TrustBandIndicator } from "./TrustBandIndicator";
import type { TrustBandWire } from "@/lib/trust-band";

const bands: Array<{ band: TrustBandWire; label: string }> = [
  { band: "likely_current", label: "Likely current" },
  { band: "use_with_caution", label: "Use with caution" },
  { band: "needs_verification", label: "Needs verification" },
  { band: "unscored", label: "Unscored" },
];

describe("TrustBandIndicator", () => {
  it("trustBandIndicator_has_visible_text_and_accessible_name", () => {
    render(<TrustBandIndicator band="use_with_caution" />);

    expect(screen.getByText("Use with caution")).toBeVisible();
    expect(
      screen.getByRole("img", {
        name: "Trust band: Use with caution. Shown in Background evidence.",
      }),
    ).toBeInTheDocument();
  });

  it("trustBandIndicator_uses_non_color_label_for_each_band", () => {
    render(
      <div>
        {bands.map(({ band }) => (
          <TrustBandIndicator key={band} band={band} />
        ))}
      </div>,
    );

    for (const { label } of bands) {
      expect(screen.getByText(label)).toBeVisible();
    }
  });
});

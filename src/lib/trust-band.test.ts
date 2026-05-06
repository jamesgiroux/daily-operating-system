import { describe, expect, it } from "vitest";
import {
  extractTrustBand,
  getNewestRenderedProvenanceSourceDate,
  partitionTrustEvidence,
} from "./trust-band";

const renderedProvenance = {
  value: {
    sources: [
      {
        source_asof: "2026-05-01T12:00:00Z",
        observed_at: "2026-05-02T12:00:00Z",
      },
      {
        observed_at: "2026-05-03T12:00:00Z",
      },
    ],
    field_attributions: {
      "/0/content": {
        trust_band: "use_with_caution",
      },
      "/1/content": {
        trust_band: "needs_verification",
      },
      "/2/content": {
        trust_band: "likely_current",
      },
    },
  },
};

describe("trust band helpers", () => {
  it("extractTrustBand_prefers_field_attribution_band", () => {
    expect(extractTrustBand(renderedProvenance, "/0/content")).toBe("use_with_caution");
  });

  it("extractTrustBand_defaults_unknown_or_missing_to_unscored", () => {
    expect(extractTrustBand(renderedProvenance, "/unknown")).toBe("unscored");
    expect(
      extractTrustBand(
        { value: { field_attributions: { "/0/content": { trust_band: "surprising" } } } },
        "/0/content",
      ),
    ).toBe("unscored");
  });

  it("partitionTrustEvidence_hides_needs_verification_by_default", () => {
    const partition = partitionTrustEvidence(
      [
        { text: "Current Example note", fieldPath: "/2/content" },
        { text: "Needs Example review", fieldPath: "/1/content" },
      ],
      {
        renderedProvenance,
        getFieldPaths: (item) => item.fieldPath,
      },
    );

    expect(partition.current.map((item) => item.text)).toEqual(["Current Example note"]);
    expect(partition.needsVerification.map((item) => item.text)).toEqual(["Needs Example review"]);
    expect(partition.revealedNeedsVerification).toEqual([]);
    expect(partition.hiddenNeedsVerificationCount).toBe(1);
  });

  it("partitionTrustEvidence_show_all_reveals_low_confidence", () => {
    const partition = partitionTrustEvidence(
      [{ text: "Needs Example review", fieldPath: "/1/content" }],
      {
        renderedProvenance,
        showAllEvidence: true,
        getFieldPaths: (item) => item.fieldPath,
      },
    );

    expect(partition.revealedNeedsVerification.map((item) => item.text)).toEqual([
      "Needs Example review",
    ]);
    expect(partition.hiddenNeedsVerificationCount).toBe(0);
  });

  it("uses_newest_source_asof_then_observed_at", () => {
    expect(getNewestRenderedProvenanceSourceDate(renderedProvenance)).toBe(
      "2026-05-03T12:00:00Z",
    );
  });
});

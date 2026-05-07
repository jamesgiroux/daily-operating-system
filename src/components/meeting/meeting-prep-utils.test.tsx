import { beforeEach, describe, expect, it } from "vitest";
import { parsePrepGridItem, partitionLegacyPrepGrid } from "./meeting-prep-utils";
import {
  clearShowAllEvidenceStateForTests,
  renderedProvenanceFrom,
} from "@/lib/trust-band";
import type { MeetingPrep } from "@/types";

beforeEach(() => {
  clearShowAllEvidenceStateForTests();
});

describe("prep grid helpers", () => {
  it("renders impact tails as badges instead of inline text", () => {
    expect(parsePrepGridItem("Support mitigated a malicious traffic spike — high")).toEqual({
      text: "Support mitigated a malicious traffic spike",
      impact: "high",
    });
  });

  it("does not duplicate the same item in Discuss and Wins", () => {
    const item =
      "Support quickly identified and mitigated a malicious traffic spike for the customer site — high";
    const prep = {
      actions: [item],
      wins: [item],
    };

    const partition = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), false);

    expect(partition.current.some((entry) => entry.section === "discuss")).toBe(false);
    expect(partition.current).toMatchObject([
      {
        section: "wins",
        text: "Support quickly identified and mitigated a malicious traffic spike for the customer site",
        impact: "high",
      },
    ]);
  });

  it("marks use-with-caution evidence as background", () => {
    const prep: MeetingPrep = {
      actions: ["Confirm rollout owner"],
      risks: ["Renewal signal is older than the current quarter"],
      renderedProvenance: {
        value: {
          field_attributions: {
            "/actions/0": { trust_band: "likely_current" as const },
            "/risks/0": { trust_band: "use_with_caution" as const },
          },
        },
      },
    };

    const partition = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), false);

    expect(partition.current).toMatchObject([{ text: "Confirm rollout owner" }]);
    expect(partition.caution).toMatchObject([
      {
        text: "Renewal signal is older than the current quarter",
        trustBand: "use_with_caution",
      },
    ]);
  });

  it("collapses needs-verification evidence until show-all", () => {
    const prep: MeetingPrep = {
      wins: ["Verify whether the expansion pilot is still active"],
      renderedProvenance: {
        value: {
          field_attributions: {
            "/wins/0": { trust_band: "needs_verification" as const },
          },
        },
      },
    };

    const hidden = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), false);
    const shown = partitionLegacyPrepGrid(prep, renderedProvenanceFrom(prep), true);

    expect(hidden.current).toHaveLength(0);
    expect(hidden.revealedNeedsVerification).toHaveLength(0);
    expect(shown.revealedNeedsVerification).toMatchObject([
      {
        text: "Verify whether the expansion pilot is still active",
        trustBand: "needs_verification",
      },
    ]);
  });
});

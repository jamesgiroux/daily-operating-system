/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import { renewalCallVerdict } from "./OutlookPanel";
import type { RenewalOutlook } from "@/types";

function ro(overrides: Partial<RenewalOutlook>): RenewalOutlook {
  return {
    confidence: undefined,
    riskFactors: [],
    expansionPotential: undefined,
    recommendedStart: undefined,
    negotiationLeverage: [],
    negotiationRisk: [],
    ...overrides,
  };
}

describe("renewalCallVerdict — chapter-title verdict from renewalOutlook", () => {
  it("returns 'Churn risk' when confidence is low", () => {
    expect(renewalCallVerdict(ro({ confidence: "low" }))).toBe("Churn risk");
  });

  it("returns 'Churn risk' even when expansion narrative is present, because confidence is low", () => {
    expect(
      renewalCallVerdict(
        ro({
          confidence: "low",
          expansionPotential:
            "Identified $80K expansion opportunity across Parse.ly and Signature tier upgrade — stalled on delivery blockers.",
        }),
      ),
    ).toBe("Churn risk");
  });

  it("returns 'Expansion' when confidence is high AND expansion narrative is substantive", () => {
    expect(
      renewalCallVerdict(
        ro({
          confidence: "high",
          expansionPotential:
            "Identified $80K expansion opportunity across Parse.ly analytics and Signature tier upgrade with FDE.",
        }),
      ),
    ).toBe("Expansion");
  });

  it("does NOT return 'Expansion' when confidence is high but expansion is empty or a negation", () => {
    expect(renewalCallVerdict(ro({ confidence: "high" }))).toBe("Renewal");
    expect(
      renewalCallVerdict(
        ro({ confidence: "high", expansionPotential: "None identified in the current cycle." }),
      ),
    ).toBe("Renewal");
    expect(
      renewalCallVerdict(
        ro({
          confidence: "high",
          expansionPotential: "No expansion signals surfaced in the last quarter.",
        }),
      ),
    ).toBe("Renewal");
  });

  it("returns 'Renewal' for moderate confidence even with substantive expansion narrative", () => {
    // Blackstone-shaped: moderate confidence + rich expansion narrative.
    // Expansion is real but we haven't earned 'Expansion' as the call yet.
    expect(
      renewalCallVerdict(
        ro({
          confidence: "moderate",
          expansionPotential:
            "Identified $73K–$85K expansion opportunity across three vectors. All currently stalled pending resolution of Safe Publisher and DR evaluation blockers.",
        }),
      ),
    ).toBe("Renewal");
  });

  it("returns 'Renewal' when outlook is null or confidence is missing", () => {
    expect(renewalCallVerdict(null)).toBe("Renewal");
    expect(renewalCallVerdict(undefined)).toBe("Renewal");
    expect(renewalCallVerdict(ro({}))).toBe("Renewal");
  });

  it("does NOT return 'Expansion' when the expansion narrative is too thin (<=80 chars)", () => {
    expect(
      renewalCallVerdict(
        ro({
          confidence: "high",
          expansionPotential: "Small add-on possible.",
        }),
      ),
    ).toBe("Renewal");
  });
});

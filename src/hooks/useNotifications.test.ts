import { describe, expect, it } from "vitest";
import { formatGleanDegradedMessage } from "./useNotifications";

describe("formatGleanDegradedMessage", () => {
  it("uses applicable dimensions instead of the legacy six-dimension total", () => {
    expect(
      formatGleanDegradedMessage({
        entity_id: "entity-1",
        entity_type: "person",
        succeeded: 2,
        failed: 1,
        failed_dimensions: ["engagement_signals"],
        required_failed_dimensions: ["engagement_signals"],
        optional_failed_dimensions: [],
        total_dimensions: 6,
        applicable_total: 3,
        skipped_dimensions: [
          "commercial_financial",
          "strategic_context",
          "value_success",
        ],
        wall_clock_ms: 1200,
        will_fall_back: false,
      }),
    ).toBe(
      "Glean refresh incomplete (2/3 applicable areas updated; 3 not applicable; 1 need attention) - showing partial results",
    );
  });

  it("keeps older payloads readable", () => {
    expect(
      formatGleanDegradedMessage({
        entity_id: "entity-1",
        entity_type: "account",
        succeeded: 2,
        failed: 4,
        failed_dimensions: [
          "commercial_financial",
          "strategic_context",
          "value_success",
          "engagement_signals",
        ],
        wall_clock_ms: 1200,
        will_fall_back: false,
      }),
    ).toBe(
      "Glean refresh incomplete (2/6 applicable areas updated; 4 need attention) - showing partial results",
    );
  });
});

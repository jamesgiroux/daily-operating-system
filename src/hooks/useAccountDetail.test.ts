import { describe, expect, it } from "vitest";
import {
  buildPresetSentimentLabels,
  DEFAULT_SENTIMENT_LABELS,
} from "./useAccountDetail";
import type { RolePreset } from "@/types/preset";

function makePreset(healthOptions: string[]): RolePreset {
  return {
    id: "test",
    name: "Test",
    description: "Test preset",
    defaultEntityMode: "account",
    vocabulary: {
      entityNoun: "account",
      entityNounPlural: "accounts",
      primaryMetric: "ARR",
      healthLabel: "Health",
      riskLabel: "Risk",
      successVerb: "renew",
      cadenceNoun: "touch",
    },
    vitals: {
      account: [
        {
          key: "health",
          label: "Health",
          fieldType: "select",
          source: "column",
          columnMapping: "health",
          options: healthOptions,
        },
      ],
      project: [],
      person: [],
    },
    metadata: {
      account: [],
      project: [],
      person: [],
    },
    lifecycleEvents: [],
    prioritization: {
      primarySignal: "health",
      secondarySignal: "renewal",
      urgencyDrivers: [],
    },
    intelligence: {
      systemRole: "",
      dimensionWeights: {},
      signalKeywords: [],
      emailSignalTypes: [],
      dimensionLabels: {},
      closeConcept: "",
      keyAdvocateLabel: "",
      dimensionGuidance: {},
    },
    briefingEmphasis: "",
  };
}

describe("buildPresetSentimentLabels", () => {
  it("keeps Your Assessment labels independent from preset health bands", () => {
    expect(buildPresetSentimentLabels(makePreset(["red", "yellow", "green"]))).toEqual(
      DEFAULT_SENTIMENT_LABELS,
    );
  });
});

import { invoke } from "@tauri-apps/api/core";

import type { AbilityResponseJson } from "@/types";
import {
  BRIEFING_SCHEMA_VERSION,
  type BriefingSection,
  type DailyBriefingOutput,
} from "./contracts";

export interface DailyBriefingAbilityRequest {
  date: string;
  workspaceId: string;
  sections?: BriefingSection[];
}

export type DailyBriefingAbilityResponse =
  AbilityResponseJson<DailyBriefingOutput>;

export function invokeDailyBriefingAbility({
  date,
  workspaceId,
  sections,
}: DailyBriefingAbilityRequest): Promise<DailyBriefingAbilityResponse> {
  return invoke<DailyBriefingAbilityResponse>("invoke_ability", {
    abilityName: "get_daily_briefing",
    inputJson: {
      schemaVersion: BRIEFING_SCHEMA_VERSION,
      date,
      workspaceId,
      sections,
    },
    renderSurface: "tauri_briefing_prep",
    dryRun: false,
    confirmation: null,
  });
}

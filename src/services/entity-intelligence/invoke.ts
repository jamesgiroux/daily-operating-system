import { invoke } from "@tauri-apps/api/core";

import type { AbilityResponseJson } from "@/types";
import {
  ENVELOPE_SCHEMA_VERSION,
  type ContextDepth,
  type EntityIntelligenceEnvelope,
  type EntityKind,
  type EnvelopeSection,
  type RenderSurface,
} from "./contracts";

export interface EntityIntelligenceAbilityRequest {
  entityType: EntityKind;
  entityId: string;
  depth?: ContextDepth;
  sections?: EnvelopeSection[];
  renderSurface: RenderSurface;
}

export type EntityIntelligenceAbilityResponse =
  AbilityResponseJson<EntityIntelligenceEnvelope>;

export function invokeEntityIntelligenceAbility({
  entityType,
  entityId,
  depth = "standard",
  sections,
  renderSurface,
}: EntityIntelligenceAbilityRequest): Promise<EntityIntelligenceAbilityResponse> {
  return invoke<EntityIntelligenceAbilityResponse>("invoke_ability", {
    abilityName: "get_entity_intelligence",
    inputJson: {
      schemaVersion: ENVELOPE_SCHEMA_VERSION,
      entityType,
      entityId,
      depth,
      sections,
    },
    renderSurface,
    dryRun: false,
    confirmation: null,
  });
}

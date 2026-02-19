/**
 * Frontend types for role presets (I312).
 * Mirrors the Rust RolePreset schema from src-tauri/src/presets/schema.rs.
 */

export interface PresetVitalField {
  key: string;
  label: string;
  fieldType: "currency" | "number" | "text" | "select" | "date";
  source: "column" | "signal" | "metadata";
  columnMapping?: string;
  options?: string[];
}

export interface PresetMetadataField {
  key: string;
  label: string;
  fieldType: "text" | "number" | "select" | "date";
  options?: string[];
  required: boolean;
}

export interface PresetVitalsConfig {
  account: PresetVitalField[];
  project: PresetVitalField[];
  person: PresetVitalField[];
}

export interface PresetMetadataConfig {
  account: PresetMetadataField[];
  project: PresetMetadataField[];
  person: PresetMetadataField[];
}

export interface PresetVocabulary {
  entityNoun: string;
  entityNounPlural: string;
  primaryMetric: string;
  healthLabel: string;
  riskLabel: string;
  successVerb: string;
  cadenceNoun: string;
}

export interface PresetPrioritization {
  primarySignal: string;
  secondarySignal: string;
  urgencyDrivers: string[];
}

export interface PresetRoleDefinition {
  id: string;
  label: string;
  description?: string;
}

export interface RolePreset {
  id: string;
  name: string;
  description: string;
  defaultEntityMode: string;
  vocabulary: PresetVocabulary;
  vitals: PresetVitalsConfig;
  metadata: PresetMetadataConfig;
  stakeholderRoles?: PresetRoleDefinition[];
  internalTeamRoles?: PresetRoleDefinition[];
  lifecycleEvents: string[];
  prioritization: PresetPrioritization;
  briefingEmphasis: string;
}

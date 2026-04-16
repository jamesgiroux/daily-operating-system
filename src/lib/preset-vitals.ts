/**
 * preset-vitals.ts — Build VitalDisplay[] from a RolePreset config (I312).
 *
 * Maps preset vital field definitions to actual entity data. Each field
 * has a `source` that tells us where to read: "column" for direct DB
 * fields, "signal" for computed signals, "metadata" for JSON metadata.
 */
import type { VitalDisplay } from "@/lib/entity-types";
import type { PresetVitalField } from "@/types/preset";
import { formatArr, formatShortDate } from "@/lib/utils";

/** Loose data shape covering all entity detail types. */
interface EntityData {
  // Account columns
  arr?: number | null;
  health?: string;
  lifecycle?: string;
  renewalDate?: string;
  contractStart?: string;
  nps?: number | null;
  // Project columns
  status?: string;
  owner?: string;
  targetDate?: string;
  // Person columns
  relationship?: string;
  meetingCount?: number;
  // Computed signals (all entity types)
  signals?: Record<string, unknown>;
  // Metadata JSON blob
  metadata?: Record<string, unknown>;
}

/** Colour hints per field type to maintain editorial tone. */
const HIGHLIGHT_MAP: Record<string, VitalDisplay["highlight"]> = {
  arr: "turmeric",
  health: undefined, // health uses healthColorMap below
  status: "olive",
  relationship: "larkspur",
};

const healthColorMap: Record<string, VitalDisplay["highlight"]> = {
  yellow: "saffron",
};

/**
 * Build a VitalDisplay[] from preset vital fields and entity data.
 *
 * Falls through gracefully: if a field's data is missing/null the vital is
 * skipped. This means the strip always reflects what the user has filled in.
 */
export function buildVitalsFromPreset(
  fields: PresetVitalField[],
  data: EntityData,
): VitalDisplay[] {
  const vitals: VitalDisplay[] = [];

  for (const field of fields) {
    const raw = resolveValue(field, data);
    if (raw == null || raw === "") continue;

    const display = formatVital(field, raw);
    if (!display) continue;

    vitals.push(display);
  }

  // Append signal-derived vitals that aren't in the preset fields
  // (e.g. meeting frequency — always useful)
  const sig = data.signals;
  const hasMeetingField = fields.some(
    (f) => f.key === "meeting_frequency_30d",
  );
  if (!hasMeetingField && sig) {
    const mf30 = sig.meetingFrequency30d as number | undefined;
    if (mf30 != null) {
      vitals.push({ text: `${mf30} meetings / 30d` });
    }
  }

  return vitals;
}

/** Resolve a field value from the entity data based on its source. */
function resolveValue(
  field: PresetVitalField,
  data: EntityData,
): unknown {
  if (field.source === "column") {
    return resolveColumn(field.columnMapping ?? field.key, data);
  }
  if (field.source === "signal") {
    return data.signals?.[field.key];
  }
  if (field.source === "metadata") {
    return data.metadata?.[field.key];
  }
  return undefined;
}

/** Map a column name to the corresponding field in the entity data. */
function resolveColumn(col: string, data: EntityData): unknown {
  switch (col) {
    case "arr":
      return data.arr;
    case "health":
      return data.health;
    case "lifecycle":
      return data.lifecycle;
    case "contract_end":
      return data.renewalDate;
    case "contract_start":
      return data.contractStart;
    case "nps":
      return data.nps;
    case "status":
      return data.status;
    case "owner":
      return data.owner;
    case "target_date":
      return data.targetDate;
    case "relationship":
      return data.relationship;
    default:
      return undefined;
  }
}

/** Format a resolved value into a VitalDisplay. */
function formatVital(
  field: PresetVitalField,
  value: unknown,
): VitalDisplay | null {
  const key = field.key;
  const label = field.label;

  switch (field.fieldType) {
    case "currency": {
      const num = typeof value === "number" ? value : parseFloat(String(value));
      if (isNaN(num)) return null;
      return {
        text: `$${formatArr(num)} ${label}`,
        highlight: HIGHLIGHT_MAP[key],
      };
    }
    case "number": {
      const num = typeof value === "number" ? value : parseFloat(String(value));
      if (isNaN(num)) return null;
      return { text: `${label} ${num}` };
    }
    case "date": {
      const str = String(value);
      if (key === "contract_end" || key === "renewal") {
        return { text: formatRenewalCountdown(str) };
      }
      return { text: `${label}: ${formatShortDate(str)}` };
    }
    case "select":
    case "text": {
      const str = String(value);
      if (key === "health") {
        const capitalized =
          str.charAt(0).toUpperCase() + str.slice(1);
        return {
          text: `${capitalized} ${label}`,
          highlight: healthColorMap[str],
        };
      }
      if (key === "status") {
        return {
          text: str
            .replace(/_/g, " ")
            .replace(/\b\w/g, (c) => c.toUpperCase()),
          highlight: HIGHLIGHT_MAP[key],
        };
      }
      if (key === "relationship") {
        return {
          text: str,
          highlight: HIGHLIGHT_MAP[key],
        };
      }
      return { text: `${str}` };
    }
    default:
      return null;
  }
}

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round(
      (renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24),
    );
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    return `Renewal in ${diffDays}d`;
  } catch {
    return dateStr;
  }
}

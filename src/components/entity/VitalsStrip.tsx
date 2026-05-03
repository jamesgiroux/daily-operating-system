/**
 * VitalsStrip — inline horizontal strip of key metrics with dot separators.
 * Generalized: accepts a pre-built `vitals` array instead of a specific entity detail.
 * Callers (account, project, person pages) assemble their own vitals array.
 */
import type { VitalDisplay } from "@/lib/entity-types";
import type { AccountSourceRef } from "@/types";
import {
  formatProvenanceSource,
} from "@/components/ui/ProvenanceLabel";
import css from "./VitalsStrip.module.css";

interface VitalsStripProps {
  vitals: VitalDisplay[];
  /** Per-field source attribution refs, keyed by field name on each VitalDisplay. */
  sourceRefs?: AccountSourceRef[];
}

const highlightColor: Record<string, string> = {
  turmeric: "var(--color-spice-turmeric)",
  saffron: "var(--color-spice-saffron)",
  olive: "var(--color-garden-olive)",
  larkspur: "var(--color-garden-larkspur)",
};

/** Map from VitalDisplay text prefix patterns to source ref field names. */
function matchVitalToSourceRef(
  vitalText: string,
  refsByField: Map<string, AccountSourceRef>,
): AccountSourceRef | undefined {
  const lower = vitalText.toLowerCase();
  if (lower.includes("arr")) return refsByField.get("arr");
  if (lower.includes("health")) return refsByField.get("health");
  if (lower.includes("nps")) return refsByField.get("nps");
  if (lower.includes("renewal")) return refsByField.get("renewal_date");
  if (lower.includes("contract")) return refsByField.get("contract_start");
  // lifecycle is rendered as plain text (e.g. "Onboarding")
  if (refsByField.has("lifecycle")) {
    const lifecycleRef = refsByField.get("lifecycle")!;
    if (lifecycleRef.sourceValue && lower.includes(lifecycleRef.sourceValue.toLowerCase())) {
      return lifecycleRef;
    }
  }
  return undefined;
}

export function VitalsStrip({ vitals, sourceRefs }: VitalsStripProps) {
  if (vitals.length === 0) return null;

  // Build a lookup from field name to the most recent source ref
  const refsByField = new Map<string, AccountSourceRef>();
  if (sourceRefs) {
    for (const ref of sourceRefs) {
      if (!refsByField.has(ref.field)) {
        refsByField.set(ref.field, ref);
      }
    }
  }

  return (
    <div className={css.strip}>
      <div className={css.items}>
        {vitals.map((v, i) => {
          const ref = sourceRefs ? matchVitalToSourceRef(v.text, refsByField) : undefined;
          const attribution = ref ? formatProvenanceSource(ref.sourceSystem) : null;
          return (
            <span key={i} className={css.item}>
              {i > 0 && (
                <span className={css.separatorDot} />
              )}
              <span className={css.fieldStack}>
                {/* Data-driven color comes from each vital's highlight value. */}
                <span
                  className={css.vitalText}
                  // Data-driven color comes from each vital's highlight value.
                  style={{ color: v.highlight ? highlightColor[v.highlight] : "var(--color-text-secondary)" }}
                >
                  {v.text}
                </span>
                {attribution && (
                  <span className={css.attribution}>
                    {attribution}
                  </span>
                )}
              </span>
            </span>
          );
        })}
      </div>
    </div>
  );
}

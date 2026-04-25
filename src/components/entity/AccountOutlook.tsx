/**
 * AccountOutlook — Three-section editorial outlook chapter.
 *
 * Surfaces agreementOutlook, expansionSignals, and contractContext
 * as three visually distinct editorial sections with generous whitespace.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
 * I650: Editorial redesign — three breathing sections, prose risk factors.
 */
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { ProvenanceTag } from "@/components/ui/ProvenanceTag";
import { formatArr } from "@/lib/utils";
import css from "./AccountOutlook.module.css";

interface AccountOutlookProps {
  intelligence: EntityIntelligence;
  onUpdateField?: (fieldPath: string, value: string) => void;
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return "";
  try {
    return new Date(dateStr).toLocaleDateString("en-US", {
      month: "long",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}

function getConfidenceClass(confidence?: string): string {
  switch (confidence?.toLowerCase()) {
    case "high":
      return css.confidenceHigh;
    case "moderate":
      return css.confidenceModerate;
    case "low":
      return css.confidenceLow;
    default:
      return "";
  }
}

function getStageBadge(stage?: string): string {
  switch (stage?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "committed":
    case "evaluating":
      return css.badgeSage;
    case "exploring":
      return css.badgeTurmeric;
    case "blocked":
      return css.badgeTerracotta;
    default:
      return css.badgeNeutral;
  }
}

function getStageLabel(stage?: string): string {
  switch (stage?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "committed":
      return "Committed";
    case "evaluating":
      return "Evaluating";
    case "exploring":
      return "Exploring";
    case "blocked":
      return "Blocked";
    default:
      return stage ?? "";
  }
}

/** Join risk factors into flowing editorial prose. */
function riskFactorsAsProse(factors: string[]): string {
  if (factors.length === 0) return "";
  if (factors.length === 1) return `Watch for ${factors[0].toLowerCase()}.`;
  const last = factors[factors.length - 1].toLowerCase();
  const rest = factors.slice(0, -1).map((f) => f.toLowerCase()).join(", ");
  return `Watch for ${rest} and ${last}.`;
}

export function AccountOutlook({
  intelligence,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: AccountOutlookProps) {
  const renewal = intelligence.agreementOutlook ?? null;
  const signals = (intelligence.expansionSignals ?? []).filter((s) => s.opportunity?.trim());
  const contract = intelligence.contractContext ?? null;

  const hasRenewal =
    renewal != null &&
    (renewal.confidence ||
      renewal.riskFactors?.length ||
      renewal.expansionPotential);
  const hasSignals = signals.length > 0;
  const hasContract =
    contract != null &&
    (contract.contractType ||
      contract.renewalDate ||
      contract.currentArr != null);

  if (!hasRenewal && !hasSignals && !hasContract) return null;

  return (
    <section className={css.section}>

      {/* ═══════════════════════════════════════════════════════════════════
          SECTION 1 — Renewal
          ═══════════════════════════════════════════════════════════════════ */}
      {hasRenewal && renewal && (
        <div className={css.renewalSection}>
          <h2 className={css.statement}>
            Renewal confidence is{" "}
            <span className={getConfidenceClass(renewal.confidence)}>
              {renewal.confidence?.toLowerCase() ?? "unknown"}
            </span>
          </h2>

          {/* Risk factors as flowing prose, not bullet list */}
          {renewal.riskFactors && renewal.riskFactors.length > 0 && (
            <p className={css.renewalProse}>
              {onUpdateField ? (
                <EditableText
                  value={riskFactorsAsProse(renewal.riskFactors)}
                  onChange={(v) => onUpdateField("agreementOutlook.riskFactors[0]", v)}
                  as="span"
                  multiline
                />
              ) : (
                riskFactorsAsProse(renewal.riskFactors)
              )}
              {onItemFeedback && (
                <span className={css.itemActions}>
                  <IntelligenceFeedback
                    value={getItemFeedback?.("agreementOutlook.riskFactors") ?? null}
                    onFeedback={(type) =>
                      onItemFeedback("agreementOutlook.riskFactors", type)
                    }
                  />
                </span>
              )}
            </p>
          )}

          {renewal.recommendedStart && (
            <p className={css.renewalStart}>
              Start the conversation by {formatDate(renewal.recommendedStart)}.
            </p>
          )}
        </div>
      )}

      {/* ═══════════════════════════════════════════════════════════════════
          SECTION 2 — Growth Opportunities
          ═══════════════════════════════════════════════════════════════════ */}
      {hasSignals && (
        <div className={css.growthSection}>
          <div className={css.sectionRule} />
          <h3 className={css.sectionHeading}>Growth Opportunities</h3>

          <div className={css.signalList}>
            {signals.map((signal, i) => (
              <div key={i} className={css.signalItem}>
                <div className={css.signalBody}>
                  {onUpdateField ? (
                    <EditableText
                      value={signal.opportunity}
                      onChange={(v) =>
                        onUpdateField(`expansionSignals[${i}].opportunity`, v)
                      }
                      as="p"
                      multiline
                      className={css.signalText}
                    />
                  ) : (
                    <p className={css.signalText}>{signal.opportunity}</p>
                  )}
                  <div className={css.signalMeta}>
                    {signal.stage && (
                      <span className={`${css.badge} ${getStageBadge(signal.stage)}`}>
                        {getStageLabel(signal.stage)}
                      </span>
                    )}
                    {signal.arrImpact != null && signal.arrImpact > 0 && (
                      <span className={css.arrImpact}>
                        +${formatArr(signal.arrImpact)} ARR
                      </span>
                    )}
                    <ProvenanceTag
                      itemSource={signal.itemSource}
                      discrepancy={signal.discrepancy}
                    />
                  </div>
                </div>
                {(onUpdateField || onItemFeedback) && (
                  <span className={css.itemActions}>
                    {onItemFeedback && (
                      <IntelligenceFeedback
                        value={
                          getItemFeedback?.(
                            `expansionSignals[${i}].opportunity`,
                          ) ?? null
                        }
                        onFeedback={(type) =>
                          onItemFeedback(
                            `expansionSignals[${i}].opportunity`,
                            type,
                          )
                        }
                      />
                    )}
                    {onUpdateField && (
                      <button
                        type="button"
                        className={css.dismissButton}
                        onClick={() =>
                          onUpdateField(
                            `expansionSignals[${i}].opportunity`,
                            "",
                          )
                        }
                        title="Dismiss"
                      >
                        <X size={13} />
                      </button>
                    )}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ═══════════════════════════════════════════════════════════════════
          SECTION 3 — Commercial Reality
          ═══════════════════════════════════════════════════════════════════ */}
      {hasContract && contract && (
        <div className={css.commercialSection}>
          <div className={css.sectionRule} />
          <h3 className={css.sectionHeading}>Commercial Reality</h3>

          <div className={css.contractGrid}>
            {contract.contractType && (
              <div className={css.contractCell}>
                <div className={css.contractLabel}>Type</div>
                <div className={css.contractValue}>{contract.contractType}</div>
              </div>
            )}
            {contract.autoRenew != null && (
              <div className={css.contractCell}>
                <div className={css.contractLabel}>Auto-Renew</div>
                <div className={css.contractValue}>
                  {contract.autoRenew ? "Yes" : "No"}
                </div>
              </div>
            )}
            {contract.renewalDate && (
              <div className={css.contractCell}>
                <div className={css.contractLabel}>Renewal</div>
                <div className={css.contractValue}>
                  {formatDate(contract.renewalDate)}
                </div>
              </div>
            )}
            {contract.currentArr != null && (
              <div className={css.contractCell}>
                <div className={css.contractLabel}>Current ARR</div>
                <div className={css.contractValueHighlight}>
                  ${formatArr(contract.currentArr)}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </section>
  );
}

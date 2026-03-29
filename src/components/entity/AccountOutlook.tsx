/**
 * AccountOutlook -- Narrative editorial outlook section.
 * Surfaces renewalOutlook, expansionSignals, and contractContext
 * from EntityIntelligence as flowing editorial prose.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
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
  /** When provided, items become editable. Called with (fieldPath, newValue). */
  onUpdateField?: (fieldPath: string, value: string) => void;
  /** Per-item feedback value getter. */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
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

export function AccountOutlook({
  intelligence,
  onUpdateField,
  getItemFeedback,
  onItemFeedback,
}: AccountOutlookProps) {
  const renewal = intelligence.renewalOutlook ?? null;
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
      {/* Editorial statement */}
      {hasRenewal && renewal && (
        <>
          <h2 className={css.statement}>
            Renewal confidence is{" "}
            <span className={getConfidenceClass(renewal.confidence)}>
              {renewal.confidence?.toLowerCase() ?? "unknown"}
            </span>
            {renewal.expansionPotential &&
              ` \u2014 ${renewal.expansionPotential}`}
          </h2>

          {/* Risk factors as supporting detail */}
          {renewal.riskFactors && renewal.riskFactors.length > 0 && (
            <ul className={css.riskFactorList}>
              {renewal.riskFactors.map((factor, i) => (
                <li key={i} className={css.riskFactorItem}>
                  <span className={css.riskFactorContent}>
                    {onUpdateField ? (
                      <EditableText
                        value={factor}
                        onChange={(v) =>
                          onUpdateField(
                            `renewalOutlook.riskFactors[${i}]`,
                            v,
                          )
                        }
                        as="span"
                        multiline
                      />
                    ) : (
                      factor
                    )}
                    {(onUpdateField || onItemFeedback) && (
                      <span className={css.itemActions}>
                        {onItemFeedback && (
                          <IntelligenceFeedback
                            value={
                              getItemFeedback?.(
                                `renewalOutlook.riskFactors[${i}]`,
                              ) ?? null
                            }
                            onFeedback={(type) =>
                              onItemFeedback(
                                `renewalOutlook.riskFactors[${i}]`,
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
                                `renewalOutlook.riskFactors[${i}]`,
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
                  </span>
                </li>
              ))}
            </ul>
          )}

          {/* Recommended start date */}
          {renewal.recommendedStart && (
            <p className={css.recommendedStart}>
              Start the conversation by {formatDate(renewal.recommendedStart)}.
            </p>
          )}
        </>
      )}

      {/* Expansion signals */}
      {hasSignals && (
        <div className={css.expansionSection}>
          <div className={css.expansionLabel}>Growth Opportunities</div>
          <div className={css.signalList}>
            {signals.map((signal, i) => (
              <div key={i} className={css.signalItem}>
                <div className={css.signalBody}>
                  {onUpdateField ? (
                    <EditableText
                      value={signal.opportunity}
                      onChange={(v) =>
                        onUpdateField(
                          `expansionSignals[${i}].opportunity`,
                          v,
                        )
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
                      <span
                        className={`${css.badge} ${getStageBadge(signal.stage)}`}
                      >
                        {getStageLabel(signal.stage)}
                      </span>
                    )}
                    {signal.arrImpact != null && signal.arrImpact > 0 && (
                      <span className={css.arrImpact}>
                        +${formatArr(signal.arrImpact)} ARR
                      </span>
                    )}
                    <ProvenanceTag itemSource={signal.itemSource} discrepancy={signal.discrepancy} />
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

      {/* Contract strip */}
      {hasContract && contract && (
        <div className={css.contractStrip}>
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
              <div className={css.contractLabel}>Renewal Date</div>
              <div className={css.contractValue}>
                {formatDate(contract.renewalDate)}
              </div>
            </div>
          )}
          {contract.currentArr != null && (
            <div className={css.contractCell}>
              <div className={css.contractLabel}>Current ARR</div>
              <div className={css.contractValueArr}>
                ${formatArr(contract.currentArr)}
              </div>
            </div>
          )}
        </div>
      )}
    </section>
  );
}

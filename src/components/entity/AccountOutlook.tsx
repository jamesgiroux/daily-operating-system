/**
 * AccountOutlook — Outlook chapter.
 * Surfaces renewalOutlook, expansionSignals, and contractContext
 * from EntityIntelligence. Collapses entirely when all are empty.
 *
 * I550: Per-item inline editing, dismiss, and feedback controls.
 */
import { X } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
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
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}

function getConfidenceColor(confidence?: string): string {
  switch (confidence?.toLowerCase()) {
    case "high":
      return css.badgeSage;
    case "moderate":
      return css.badgeTurmeric;
    case "low":
      return css.badgeTerracotta;
    default:
      return css.badgeNeutral;
  }
}

function getStageColor(stage?: string): string {
  switch (stage?.toLowerCase().replace(/[_\s-]/g, "")) {
    case "committed":
      return css.badgeSage;
    case "evaluating":
      return css.badgeTurmeric;
    case "exploring":
      return css.badgeLarkspur;
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
  const signals = intelligence.expansionSignals ?? [];
  const contract = intelligence.contractContext ?? null;

  const hasRenewal = renewal != null && (renewal.confidence || renewal.riskFactors?.length || renewal.expansionPotential);
  const hasSignals = signals.length > 0;
  const hasContract = contract != null && (contract.contractType || contract.renewalDate || contract.currentArr != null);

  if (!hasRenewal && !hasSignals && !hasContract) return null;

  return (
    <section className={css.section}>
      <ChapterHeading title="Outlook" />

      {/* Renewal Outlook */}
      {hasRenewal && renewal && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Renewal Outlook</h3>
          <div className={css.renewalCard}>
            {renewal.confidence && (
              <div className={css.renewalHeader}>
                <span className={css.renewalConfidenceLabel}>Confidence</span>
                <span className={`${css.badge} ${getConfidenceColor(renewal.confidence)}`}>
                  {renewal.confidence}
                </span>
              </div>
            )}
            {renewal.riskFactors && renewal.riskFactors.length > 0 && (
              <div className={css.renewalSection}>
                <span className={css.renewalFieldLabel}>Risk Factors</span>
                <ul className={css.riskFactorList}>
                  {renewal.riskFactors.map((factor, i) => (
                    <li key={i} className={css.riskFactorItem}>
                      <span className={css.riskFactorContent}>
                        {onUpdateField ? (
                          <EditableText
                            value={factor}
                            onChange={(v) => onUpdateField(`renewalOutlook.riskFactors[${i}]`, v)}
                            as="span"
                            multiline
                            style={{ font: "inherit", color: "inherit" }}
                          />
                        ) : (
                          factor
                        )}
                        {(onUpdateField || onItemFeedback) && (
                          <span className={css.itemActions}>
                            {onItemFeedback && (
                              <IntelligenceFeedback
                                value={getItemFeedback?.(`renewalOutlook.riskFactors[${i}]`) ?? null}
                                onFeedback={(type) => onItemFeedback(`renewalOutlook.riskFactors[${i}]`, type)}
                              />
                            )}
                            {onUpdateField && (
                              <button
                                type="button"
                                className={css.dismissButton}
                                onClick={() => onUpdateField(`renewalOutlook.riskFactors[${i}]`, "")}
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
              </div>
            )}
            {renewal.expansionPotential && (
              <div className={css.renewalSection}>
                <span className={css.renewalFieldLabel}>Expansion Potential</span>
                <p className={css.renewalText}>{renewal.expansionPotential}</p>
              </div>
            )}
            {renewal.recommendedStart && (
              <div className={css.renewalSection}>
                <span className={css.renewalFieldLabel}>Recommended Start</span>
                <span className={css.renewalDate}>{formatDate(renewal.recommendedStart)}</span>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Expansion Signals */}
      {hasSignals && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Expansion Signals</h3>
          <div className={css.signalList}>
            {signals.map((signal, i) => (
              <div key={i} className={css.signalItem}>
                <div className={css.signalHeader}>
                  {onUpdateField ? (
                    <EditableText
                      value={signal.opportunity}
                      onChange={(v) => onUpdateField(`expansionSignals[${i}].opportunity`, v)}
                      as="p"
                      multiline
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontSize: 16,
                        lineHeight: 1.5,
                        color: "var(--color-text-primary)",
                        margin: 0,
                        flex: 1,
                      }}
                    />
                  ) : (
                    <p className={css.signalText}>{signal.opportunity}</p>
                  )}
                  {signal.stage && (
                    <span className={`${css.badge} ${getStageColor(signal.stage)}`}>
                      {getStageLabel(signal.stage)}
                    </span>
                  )}
                  {(onUpdateField || onItemFeedback) && (
                    <span className={css.itemActions}>
                      {onItemFeedback && (
                        <IntelligenceFeedback
                          value={getItemFeedback?.(`expansionSignals[${i}].opportunity`) ?? null}
                          onFeedback={(type) => onItemFeedback(`expansionSignals[${i}].opportunity`, type)}
                        />
                      )}
                      {onUpdateField && (
                        <button
                          type="button"
                          className={css.dismissButton}
                          onClick={() => onUpdateField(`expansionSignals[${i}].opportunity`, "")}
                          title="Dismiss"
                        >
                          <X size={13} />
                        </button>
                      )}
                    </span>
                  )}
                </div>
                {signal.arrImpact != null && signal.arrImpact > 0 && (
                  <span className={css.arrImpact}>+${formatArr(signal.arrImpact)} ARR</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Contract Context */}
      {hasContract && contract && (
        <div className={css.subsection}>
          <h3 className={css.subsectionLabel}>Contract Context</h3>
          <div className={css.contractGrid}>
            {contract.contractType && (
              <div className={css.contractField}>
                <span className={css.contractFieldLabel}>Type</span>
                <span className={css.contractFieldValue}>{contract.contractType}</span>
              </div>
            )}
            {contract.autoRenew != null && (
              <div className={css.contractField}>
                <span className={css.contractFieldLabel}>Auto-Renew</span>
                <span className={css.contractFieldValue}>{contract.autoRenew ? "Yes" : "No"}</span>
              </div>
            )}
            {contract.renewalDate && (
              <div className={css.contractField}>
                <span className={css.contractFieldLabel}>Renewal Date</span>
                <span className={css.contractFieldValue}>{formatDate(contract.renewalDate)}</span>
              </div>
            )}
            {contract.currentArr != null && (
              <div className={css.contractField}>
                <span className={css.contractFieldLabel}>Current ARR</span>
                <span className={css.contractFieldValue}>${formatArr(contract.currentArr)}</span>
              </div>
            )}
            {contract.multiYearRemaining != null && contract.multiYearRemaining > 0 && (
              <div className={css.contractField}>
                <span className={css.contractFieldLabel}>Multi-Year Remaining</span>
                <span className={css.contractFieldValue}>{contract.multiYearRemaining} year{contract.multiYearRemaining !== 1 ? "s" : ""}</span>
              </div>
            )}
          </div>
        </div>
      )}
    </section>
  );
}

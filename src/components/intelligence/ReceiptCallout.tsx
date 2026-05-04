import clsx from "clsx";
import { useId, useState, type ComponentPropsWithoutRef, type ReactNode } from "react";
import { AsOfTimestamp } from "@/components/ui/AsOfTimestamp";
import { ConfidenceScoreChip } from "@/components/ui/ConfidenceScoreChip";
import { DataGapNotice } from "@/components/ui/DataGapNotice";
import { FreshnessIndicator } from "@/components/ui/FreshnessIndicator";
import { Pill } from "@/components/ui/Pill";
import { ProvenanceTag, type ProvenanceTagSource } from "@/components/ui/ProvenanceTag";
import type { TrustBand as TrustBandValue } from "@/components/ui/TrustBandBadge";
import { VerificationStatusFlag } from "@/components/ui/VerificationStatusFlag";
import { ClaimRow, type ClaimConsistencyState, type ClaimRowClaim } from "./ClaimRow";
import { TrustBand } from "./TrustBand";
import styles from "./ReceiptCallout.module.css";

export type ReceiptCalloutMode = "collapsed" | "expanded";
export type ReceiptCalloutPosition = "inline" | "drawer";

export interface ReceiptSourceDetail {
  id?: string;
  source?: ProvenanceTagSource | null;
  label?: ReactNode;
  capturedAt?: string | Date | null;
  rawValue?: ReactNode;
  confidence?: number | null;
  href?: string;
}

export interface ReceiptFreshnessStep {
  id?: string;
  label: ReactNode;
  at?: string | Date | null;
  detail?: ReactNode;
}

export interface ReceiptContradiction {
  id?: string;
  label: ReactNode;
  href?: string;
  severity?: "low" | "medium" | "high";
}

export interface ReceiptCorrectionEntry {
  id?: string;
  previousValue?: ReactNode;
  correctedValue?: ReactNode;
  correctedAt?: string | Date | null;
  correctedBy?: ReactNode;
  note?: ReactNode;
}

export interface ReceiptCalloutProps
  extends Omit<ComponentPropsWithoutRef<"section">, "children"> {
  claim: ClaimRowClaim;
  mode?: ReceiptCalloutMode;
  defaultMode?: ReceiptCalloutMode;
  position?: ReceiptCalloutPosition;
  loading?: boolean;
  error?: ReactNode;
  resolverConfidence?: number | null;
  resolverLabel?: ReactNode;
  consistencyState?: ClaimConsistencyState;
  consistencyMessage?: ReactNode;
  sources?: ReceiptSourceDetail[];
  freshnessChain?: ReceiptFreshnessStep[];
  contradictions?: ReceiptContradiction[];
  correctionHistory?: ReceiptCorrectionEntry[];
  onModeChange?: (mode: ReceiptCalloutMode, claim: ClaimRowClaim) => void;
  onConfirm?: (claim: ClaimRowClaim) => void;
  onCorrect?: (claim: ClaimRowClaim) => void;
  onDismiss?: (claim: ClaimRowClaim) => void;
}

function resolveMode(
  controlledMode: ReceiptCalloutMode | undefined,
  internalMode: ReceiptCalloutMode,
): ReceiptCalloutMode {
  return controlledMode ?? internalMode;
}

function resolveBand(claim: ClaimRowClaim): TrustBandValue | undefined {
  return claim.band ?? claim.trustBand;
}

function resolverBandLabel(band: TrustBandValue | undefined): string {
  switch (band) {
    case "likely_current":
      return "Likely current";
    case "use_with_caution":
      return "Use with caution";
    case "needs_verification":
      return "Needs verification";
    default:
      return "Unbanded";
  }
}

function resolvePrimarySource(claim: ClaimRowClaim): ProvenanceTagSource | null | undefined {
  return claim.itemSource ?? claim.source;
}

function hasRenderableContent(value: ReactNode): boolean {
  return value !== null && value !== undefined && value !== false && value !== "";
}

function defaultSources(claim: ClaimRowClaim): ReceiptSourceDetail[] {
  const source = resolvePrimarySource(claim);
  if (!source && !claim.asOf && !hasRenderableContent(claim.value)) return [];

  return [
    {
      id: "primary",
      source,
      label: "Primary source",
      capturedAt: claim.asOf ?? claim.sourceAsof ?? claim.sourcedAt,
      rawValue: claim.value,
      confidence: claim.confidence,
    },
  ];
}

function defaultFreshnessChain(claim: ClaimRowClaim): ReceiptFreshnessStep[] {
  const asOf = claim.asOf ?? claim.sourceAsof ?? claim.sourcedAt;
  if (!asOf) return [];

  return [
    {
      id: "as-of",
      label: "Claim as of",
      at: asOf,
    },
  ];
}

function verificationStatus(state: ClaimConsistencyState) {
  if (state === "flagged") return "flagged";
  if (state === "corrected") return "corrected";
  return "ok";
}

function consistencyLabel(state: ClaimConsistencyState): string {
  switch (state) {
    case "corrected":
      return "Corrected";
    case "flagged":
      return "Flagged";
    case "dismissed":
      return "Dismissed";
    case "ok":
    default:
      return "Consistent";
  }
}

function renderRawValue(value: ReactNode) {
  if (!hasRenderableContent(value)) return null;
  return <blockquote className={styles.rawValue}>{value}</blockquote>;
}

export function ReceiptCallout({
  claim,
  mode,
  defaultMode = "collapsed",
  position = "inline",
  loading = false,
  error,
  resolverConfidence,
  resolverLabel,
  consistencyState,
  consistencyMessage,
  sources,
  freshnessChain,
  contradictions = [],
  correctionHistory,
  onModeChange,
  onConfirm,
  onCorrect,
  onDismiss,
  className,
  ...sectionProps
}: ReceiptCalloutProps) {
  const receiptId = useId();
  const [internalMode, setInternalMode] = useState(defaultMode);
  const resolvedMode = resolveMode(mode, internalMode);
  const expanded = resolvedMode === "expanded";
  const band = resolveBand(claim);
  const resolvedConsistencyState = consistencyState ?? claim.consistencyState ?? "ok";
  const resolvedSources = sources ?? defaultSources(claim);
  const resolvedFreshnessChain = freshnessChain ?? defaultFreshnessChain(claim);
  const resolvedConfidence = resolverConfidence ?? claim.confidence;
  const resolvedCorrectionHistory = correctionHistory ?? (
    claim.correction || claim.previousValue || claim.correctedValue
      ? [
          {
            id: "current-correction",
            previousValue: claim.correction?.previousValue ?? claim.previousValue,
            correctedValue: claim.correction?.correctedValue ?? claim.correctedValue ?? claim.value,
            correctedAt: claim.correction?.correctedAt,
            correctedBy: claim.correction?.correctedBy,
            note: claim.correction?.note,
          },
        ]
      : []
  );

  function setMode(nextMode: ReceiptCalloutMode) {
    if (mode === undefined) setInternalMode(nextMode);
    onModeChange?.(nextMode, claim);
  }

  return (
    <section
      className={clsx(styles.root, className)}
      data-mode={resolvedMode}
      data-position={position}
      data-band={band}
      data-ds-name="ReceiptCallout"
      data-ds-spec="patterns/ReceiptCallout.md"
      {...sectionProps}
    >
      <ClaimRow
        claim={{ ...claim, consistencyState: resolvedConsistencyState, consistencyMessage }}
        density={position === "drawer" ? "rich" : "default"}
        expandable
        expanded={expanded}
        controlsId={receiptId}
        loading={loading}
        error={error ? "Unable to load receipt detail." : undefined}
        onExpandedChange={(nextExpanded) => setMode(nextExpanded ? "expanded" : "collapsed")}
        onCorrect={onCorrect}
        onDismiss={onDismiss}
      />

      {expanded && (
        <div
          id={receiptId}
          className={styles.receipt}
          role="region"
          aria-label="Claim receipt"
        >
          {loading ? (
            <div className={styles.loading} role="status">
              <span className={styles.skeletonLine} />
              <span className={styles.skeletonLine} />
              <span className={styles.skeletonShort} />
            </div>
          ) : error ? (
            <div className={styles.error} role="alert">
              {error}
            </div>
          ) : (
            <>
              <div className={styles.resolverBand}>
                <div className={styles.resolverCopy}>
                  <span className={styles.kicker}>Resolver band</span>
                  <span className={styles.resolverLabel}>
                    {resolverLabel ?? resolverBandLabel(band)}
                  </span>
                </div>
                <div className={styles.resolverSignals}>
                  <TrustBand claim={claim} density="default" />
                  <ConfidenceScoreChip score={resolvedConfidence} />
                </div>
              </div>

              <div className={styles.consistency}>
                {resolvedConsistencyState === "dismissed" ? (
                  <Pill tone="neutral" size="compact">Dismissed</Pill>
                ) : (
                  <VerificationStatusFlag
                    status={verificationStatus(resolvedConsistencyState)}
                    label={consistencyLabel(resolvedConsistencyState)}
                  />
                )}
                <span>
                  {consistencyMessage
                    ?? claim.consistencyMessage
                    ?? "No consistency conflicts are attached to this claim."}
                </span>
              </div>

              <div className={styles.section}>
                <h4 className={styles.sectionTitle}>Provenance chain</h4>
                {resolvedSources.length > 0 ? (
                  <ul className={styles.sourceList}>
                    {resolvedSources.map((source, index) => (
                      <li className={styles.sourceItem} key={source.id ?? index}>
                        <div className={styles.sourceHeader}>
                          <span className={styles.sourceLabel}>{source.label ?? "Source"}</span>
                          {source.source && (
                            <ProvenanceTag itemSource={source.source} showSynthesized />
                          )}
                          <ConfidenceScoreChip score={source.confidence} />
                        </div>
                        <div className={styles.sourceMeta}>
                          <AsOfTimestamp at={source.capturedAt ?? null} format="both" prefix="Captured" />
                          {source.href && (
                            <a className={styles.sourceLink} href={source.href}>
                              Open source
                            </a>
                          )}
                        </div>
                        {renderRawValue(source.rawValue)}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <DataGapNotice message="No provenance chain is available for this claim." />
                )}
              </div>

              <div className={styles.section}>
                <h4 className={styles.sectionTitle}>Freshness chain</h4>
                {resolvedFreshnessChain.length > 0 ? (
                  <ul className={styles.freshnessList}>
                    {resolvedFreshnessChain.map((step, index) => (
                      <li className={styles.freshnessItem} key={step.id ?? index}>
                        <span className={styles.freshnessLabel}>{step.label}</span>
                        <FreshnessIndicator at={step.at} format="both" />
                        {step.detail && <span className={styles.freshnessDetail}>{step.detail}</span>}
                      </li>
                    ))}
                  </ul>
                ) : (
                  <DataGapNotice message="No freshness checkpoints are available for this claim." />
                )}
              </div>

              {(contradictions.length > 0 || resolvedCorrectionHistory.length > 0) && (
                <div className={styles.section}>
                  <h4 className={styles.sectionTitle}>Findings</h4>
                  {contradictions.length > 0 && (
                    <ul className={styles.findingList}>
                      {contradictions.map((contradiction, index) => (
                        <li
                          className={styles.findingItem}
                          data-severity={contradiction.severity}
                          key={contradiction.id ?? index}
                        >
                          {contradiction.href ? (
                            <a className={styles.sourceLink} href={contradiction.href}>
                              {contradiction.label}
                            </a>
                          ) : (
                            contradiction.label
                          )}
                        </li>
                      ))}
                    </ul>
                  )}
                  {resolvedCorrectionHistory.length > 0 && (
                    <ul className={styles.correctionList}>
                      {resolvedCorrectionHistory.map((entry, index) => (
                        <li className={styles.correctionItem} key={entry.id ?? index}>
                          <span className={styles.correctionValues}>
                            {hasRenderableContent(entry.previousValue) && (
                              <span className={styles.previousValue}>{entry.previousValue}</span>
                            )}
                            {hasRenderableContent(entry.correctedValue) && (
                              <span className={styles.correctedValue}>{entry.correctedValue}</span>
                            )}
                          </span>
                          <span className={styles.sourceMeta}>
                            {entry.correctedBy && <span>by {entry.correctedBy}</span>}
                            <AsOfTimestamp at={entry.correctedAt ?? null} format="absolute" prefix="Corrected" />
                            {entry.note && <span>{entry.note}</span>}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}

              <div className={styles.actions} aria-label="Receipt actions">
                {onConfirm && (
                  <Pill as="button" tone="sage" size="compact" interactive onClick={() => onConfirm(claim)}>
                    Confirm
                  </Pill>
                )}
                {onCorrect && (
                  <Pill as="button" tone="neutral" size="compact" interactive onClick={() => onCorrect(claim)}>
                    Correct
                  </Pill>
                )}
                {onDismiss && (
                  <Pill as="button" tone="neutral" size="compact" interactive onClick={() => onDismiss(claim)}>
                    Dismiss
                  </Pill>
                )}
              </div>
            </>
          )}
        </div>
      )}
    </section>
  );
}

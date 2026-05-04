import clsx from "clsx";
import { ChevronDown } from "lucide-react";
import {
  useId,
  useState,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from "react";
import { Pill } from "@/components/ui/Pill";
import type { ProvenanceTagSource } from "@/components/ui/ProvenanceTag";
import type { TrustBand as TrustBandValue } from "@/components/ui/TrustBandBadge";
import { VerificationStatusFlag } from "@/components/ui/VerificationStatusFlag";
import { TrustBand, type TrustBandClaim } from "./TrustBand";
import styles from "./ClaimRow.module.css";

export type ClaimRowDensity = "compact" | "default" | "rich";
export type ClaimConsistencyState = "ok" | "corrected" | "flagged" | "dismissed";

export interface ClaimCorrection {
  previousValue?: ReactNode;
  correctedValue?: ReactNode;
  correctedBy?: ReactNode;
  correctedAt?: string | Date | null;
  note?: ReactNode;
}

export interface ClaimRowClaim extends TrustBandClaim {
  id?: string | number;
  subject?: ReactNode;
  field?: ReactNode;
  value?: ReactNode;
  previousValue?: ReactNode;
  correctedValue?: ReactNode;
  band?: TrustBandValue;
  trustBand?: TrustBandValue;
  source?: ProvenanceTagSource | null;
  itemSource?: ProvenanceTagSource | null;
  asOf?: string | Date | null;
  confidence?: number | null;
  consistencyState?: ClaimConsistencyState;
  consistencyMessage?: ReactNode;
  correction?: ClaimCorrection;
}

export interface ClaimRowAction {
  label: ReactNode;
  onClick: (claim: ClaimRowClaim) => void;
  disabled?: boolean;
  tone?: "neutral" | "sage" | "turmeric" | "terracotta";
}

export interface ClaimRowProps
  extends Omit<ComponentPropsWithoutRef<"article">, "children"> {
  claim: ClaimRowClaim;
  density?: ClaimRowDensity;
  expandable?: boolean;
  expanded?: boolean;
  defaultExpanded?: boolean;
  controlsId?: string;
  expandButtonLabel?: string;
  expandedContent?: ReactNode;
  loading?: boolean;
  error?: ReactNode;
  actions?: ClaimRowAction[];
  onExpandedChange?: (expanded: boolean, claim: ClaimRowClaim) => void;
  onCorrect?: (claim: ClaimRowClaim) => void;
  onDismiss?: (claim: ClaimRowClaim) => void;
  onInspect?: (claim: ClaimRowClaim) => void;
}

function hasRenderableContent(value: ReactNode): boolean {
  return value !== null && value !== undefined && value !== false && value !== "";
}

function fieldLabel(field: ReactNode): ReactNode {
  if (typeof field !== "string") return field;

  return field
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function normalizeConsistencyState(claim: ClaimRowClaim): ClaimConsistencyState {
  if (claim.consistencyState) return claim.consistencyState;
  if (hasRenderableContent(claim.correctedValue) || hasRenderableContent(claim.previousValue)) {
    return "corrected";
  }
  return "ok";
}

function formatDate(value: string | Date | null | undefined): string | null {
  if (!value) return null;
  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function renderValue(claim: ClaimRowClaim, state: ClaimConsistencyState) {
  const correction = claim.correction;
  const previousValue = correction?.previousValue ?? claim.previousValue;
  const correctedValue = correction?.correctedValue ?? claim.correctedValue ?? claim.value;
  const value = hasRenderableContent(correctedValue) ? correctedValue : "Unavailable";

  if (state === "corrected" && hasRenderableContent(previousValue)) {
    return (
      <span className={styles.correctionPair}>
        <span className={styles.previousValue}>{previousValue}</span>
        <span className={styles.correctedValue}>{value}</span>
      </span>
    );
  }

  return value;
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
      return "OK";
  }
}

function renderStatusFlag(state: ClaimConsistencyState) {
  if (state === "dismissed") {
    return (
      <Pill tone="neutral" size="compact" className={styles.dismissedFlag}>
        Dismissed
      </Pill>
    );
  }

  return (
    <VerificationStatusFlag
      status={state === "flagged" ? "flagged" : state === "corrected" ? "corrected" : "ok"}
      label={consistencyLabel(state)}
    />
  );
}

export function ClaimRow({
  claim,
  density = "default",
  expandable = false,
  expanded,
  defaultExpanded = false,
  controlsId,
  expandButtonLabel = "Inspect claim receipt",
  expandedContent,
  loading = false,
  error,
  actions,
  onExpandedChange,
  onCorrect,
  onDismiss,
  onInspect,
  className,
  ...articleProps
}: ClaimRowProps) {
  const generatedDetailsId = useId();
  const [internalExpanded, setInternalExpanded] = useState(defaultExpanded);
  const isExpanded = expanded ?? internalExpanded;
  const detailsId = controlsId ?? generatedDetailsId;
  const state = normalizeConsistencyState(claim);
  const trustDensity = density === "rich" ? "expanded" : density === "compact" ? "compact" : "default";
  const renderedActions: ClaimRowAction[] = [
    ...(actions ?? []),
    ...(onCorrect ? [{ label: "Correct", onClick: onCorrect, tone: "neutral" as const }] : []),
    ...(onDismiss ? [{ label: "Dismiss", onClick: onDismiss, tone: "neutral" as const }] : []),
  ];
  const showActions = density === "rich" && renderedActions.length > 0;
  const field = fieldLabel(claim.field);

  function toggleExpanded() {
    if (!expandable) return;
    const next = !isExpanded;
    if (expanded === undefined) setInternalExpanded(next);
    onExpandedChange?.(next, claim);
    if (next) onInspect?.(claim);
  }

  return (
    <article
      className={clsx(styles.row, className)}
      data-density={density}
      data-state={state}
      data-loading={loading ? "true" : undefined}
      data-ds-name="ClaimRow"
      data-ds-spec="patterns/ClaimRow.md"
      aria-busy={loading ? "true" : undefined}
      {...articleProps}
    >
      {state === "flagged" && (
        <div className={styles.stateBanner} role="status">
          {renderStatusFlag(state)}
          <span>{claim.consistencyMessage ?? "This claim needs review before reuse."}</span>
        </div>
      )}

      <div className={styles.shell}>
        <div className={styles.body}>
          <div className={styles.claimLine}>
            {hasRenderableContent(claim.subject) && (
              <Pill tone="neutral" size="compact" className={styles.subject}>
                {claim.subject}
              </Pill>
            )}
            {hasRenderableContent(field) && <span className={styles.field}>{field}</span>}
            {hasRenderableContent(field) && <span className={styles.dash} aria-hidden="true">—</span>}
            <span className={styles.value}>{loading ? "Loading claim..." : renderValue(claim, state)}</span>
          </div>

          {error && (
            <div className={styles.error} role="alert">
              {error}
            </div>
          )}

          {state === "corrected" && (
            <div className={styles.correctionMeta}>
              {renderStatusFlag(state)}
              {claim.correction?.correctedBy && <span>by {claim.correction.correctedBy}</span>}
              {formatDate(claim.correction?.correctedAt) && (
                <span>{formatDate(claim.correction?.correctedAt)}</span>
              )}
              {claim.correction?.note && <span>{claim.correction.note}</span>}
            </div>
          )}

          {density === "rich" && (
            <TrustBand claim={claim} density={trustDensity} align="row" className={styles.richTrust} />
          )}

          {showActions && (
            <div className={styles.actions}>
              {renderedActions.map((action, index) => (
                <Pill
                  as="button"
                  tone={action.tone ?? "neutral"}
                  size="compact"
                  interactive
                  disabled={action.disabled}
                  onClick={() => action.onClick(claim)}
                  key={index}
                >
                  {action.label}
                </Pill>
              ))}
            </div>
          )}
        </div>

        {density !== "rich" && (
          <TrustBand claim={claim} density={trustDensity} className={styles.trust} />
        )}

        {expandable && (
          <button
            type="button"
            className={styles.expandButton}
            aria-label={expandButtonLabel}
            aria-expanded={isExpanded}
            aria-controls={detailsId}
            onClick={toggleExpanded}
          >
            <ChevronDown className={styles.chevron} aria-hidden="true" />
          </button>
        )}
      </div>

      {expandable && isExpanded && expandedContent && (
        <div id={detailsId} className={styles.expanded}>
          {expandedContent}
        </div>
      )}
    </article>
  );
}

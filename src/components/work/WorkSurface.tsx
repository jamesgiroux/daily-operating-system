/**
 * WorkSurface — component vocabulary for the Work tab (DOS-13).
 *
 * Workbench, not todo list. Zero-guilt execution surface.
 *
 * Components exported here are rendered from AccountDetailPage.renderWorkView.
 * Copy is load-bearing — see .docs/mockups/account-work-globex.html.
 *
 * Zero-guilt principles:
 *   - "Still active?" replaces "OVERDUE" everywhere; no red counters, no streak breakers.
 *   - Private vs Shared visibility is orthogonal to draft/done status.
 *   - Dismiss is equal-valid with Accept / Push / Mark done / Leave-as-is.
 *   - Suggestions carry "Dismiss (teaches system)" — they never "delete".
 *   - Nudges always offer "Leave as-is" as a first-class exit.
 *   - Nudge chapter hides entirely when the list is empty.
 */
import type { ReactNode } from "react";
import s from "./WorkSurface.module.css";

/* ─────────────────────────────────────────────────────────────────────────
 * VisibilityPill — Private vs Shared (renamed from "pencil vs pen").
 * Orthogonal to draft/done. Shared form is a link (mockup affords clicking
 * through to the external tracker).
 * ──────────────────────────────────────────────────────────────────────── */
export interface VisibilityPillProps {
  variant: "private" | "shared";
  label?: string;
  href?: string;
}

export function VisibilityPill({ variant, label, href }: VisibilityPillProps) {
  const className =
    variant === "private"
      ? `${s.visibilityPill} ${s.visibilityPrivate}`
      : `${s.visibilityPill} ${s.visibilityShared}`;
  const text = label ?? (variant === "private" ? "Private" : "Shared");
  if (variant === "shared" && href) {
    return (
      <a className={className} href={href} target="_blank" rel="noreferrer">
        {text}
      </a>
    );
  }
  return <span className={className}>{text}</span>;
}

/* ─────────────────────────────────────────────────────────────────────────
 * AudiencePill — Customer-facing vs Internal (also orthogonal).
 * ──────────────────────────────────────────────────────────────────────── */
export interface AudiencePillProps {
  variant: "customer" | "internal";
}

export function AudiencePill({ variant }: AudiencePillProps) {
  const className =
    variant === "customer"
      ? `${s.audiencePill} ${s.audienceCustomer}`
      : `${s.audiencePill} ${s.audienceInternal}`;
  return (
    <span className={className}>
      {variant === "customer" ? "Customer-facing" : "Internal"}
    </span>
  );
}

/* ─────────────────────────────────────────────────────────────────────────
 * WorkButton — the shared pill button vocabulary.
 * ──────────────────────────────────────────────────────────────────────── */
export interface WorkButtonProps {
  kind?: "default" | "primary" | "accent" | "muted";
  onClick?: () => void;
  disabled?: boolean;
  children: ReactNode;
  title?: string;
  type?: "button" | "submit";
}

export function WorkButton({
  kind = "default",
  onClick,
  disabled,
  children,
  title,
  type = "button",
}: WorkButtonProps) {
  const cls =
    kind === "primary"
      ? `${s.workBtn} ${s.workBtnPrimary}`
      : kind === "accent"
        ? `${s.workBtn} ${s.workBtnAccent}`
        : kind === "muted"
          ? `${s.workBtn} ${s.workBtnMuted}`
          : s.workBtn;
  return (
    <button type={type} className={cls} onClick={onClick} disabled={disabled} title={title}>
      {children}
    </button>
  );
}

/* ─────────────────────────────────────────────────────────────────────────
 * CiteChip — dotted-underline reference used in focus provenance rows.
 * ──────────────────────────────────────────────────────────────────────── */
export function CiteChip({
  children,
  onClick,
  href,
}: {
  children: ReactNode;
  onClick?: () => void;
  href?: string;
}) {
  if (href) {
    return (
      <a className={s.citeChip} href={href}>
        {children}
      </a>
    );
  }
  return (
    <button
      type="button"
      className={s.citeChip}
      style={{ background: "none", border: "none", padding: 0, cursor: onClick ? "pointer" : "default" }}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

/* ─────────────────────────────────────────────────────────────────────────
 * NumberedFocusList — Chapter 1 "90-day focus".
 * Large serif numerals, editorial paragraph, provenance chips.
 * Not a card grid — a reading sequence. Cites the commitments / suggestions /
 * programs the AI synthesized the item from.
 * ──────────────────────────────────────────────────────────────────────── */
export interface FocusItem {
  headline: string;
  paragraph: string;
  /** Where this focus item was synthesized from — one chip per source. */
  citations?: { label: string; href?: string }[];
}

export function NumberedFocusList({ items }: { items: FocusItem[] }) {
  if (items.length === 0) return null;
  return (
    <div className={s.focusList}>
      {items.map((item, idx) => (
        <article key={idx} className={s.focusItem}>
          <div className={s.focusNumeral}>{idx + 1}</div>
          <div className={s.focusBody}>
            <h3 className={s.focusHeadline}>{item.headline}</h3>
            <p className={s.focusPara}>{item.paragraph}</p>
            {item.citations && item.citations.length > 0 && (
              <div className={s.focusProvenance}>
                <span className={s.focusProvenanceLabel}>Synthesized from</span>
                {item.citations.map((c, i) => (
                  <CiteChip key={i} href={c.href}>
                    {c.label}
                  </CiteChip>
                ))}
              </div>
            )}
          </div>
        </article>
      ))}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────────────
 * ProgramPill — Chapter 2 "Programs & motions".
 * Standing state — orientation, not a to-do.
 * ──────────────────────────────────────────────────────────────────────── */
export interface ProgramPillProps {
  state: string;
  description: string;
}

export function ProgramPill({ state, description }: ProgramPillProps) {
  return (
    <article className={s.programPill}>
      <div className={s.programState}>{state}</div>
      <div className={s.programDescription}>{description}</div>
    </article>
  );
}

export function ProgramPillRow({ children }: { children: ReactNode }) {
  return <div className={s.programsRow}>{children}</div>;
}

/* ─────────────────────────────────────────────────────────────────────────
 * CommitmentCard — Chapter 3 "Commitments".
 * Provenance + Owner + Due (neutral) + Audience pill + Visibility pill,
 * optional soft "Still active?" nudge, and equal-valid exits.
 * ──────────────────────────────────────────────────────────────────────── */
export interface CommitmentCardProps {
  headline: string;
  /** e.g. "meeting · Feb 17". Rendered as cite chip after "From:". */
  provenance?: { label: string; href?: string }[];
  owner?: string | null;
  /** Human-readable due string or null. Never flagged red — neutral copy. */
  due?: string | null;
  audience: "customer" | "internal";
  visibility: "private" | "shared";
  /** When visibility === "shared", the external tracker reference (e.g. Linear ID). */
  sharedRef?: { label: string; href?: string };
  /** Optional inline status text from the external tracker. */
  linearStatus?: string;
  /** Soft, non-guilt flag. Copy must open with "Still active?" — never "OVERDUE". */
  stillActiveNote?: string;
  /** Equal-valid exits: Mark done / Push to tracker / Dismiss / Leave-as-is. */
  actions: ReactNode;
  /**
   * DOS Work-tab Phase 3: top-N visual weight. When true, the headline
   * renders at a heavier serif weight to give the first ~4 commitments a
   * "big three or four" reading order without demoting the rest.
   */
  emphasis?: boolean;
}

export function CommitmentCard({
  headline,
  provenance,
  owner,
  due,
  audience,
  visibility,
  sharedRef,
  linearStatus,
  stillActiveNote,
  actions,
  emphasis,
}: CommitmentCardProps) {
  const headlineClass = emphasis
    ? `${s.commitmentHeadline} ${s.commitmentHeadlineEmphasis}`
    : s.commitmentHeadline;
  return (
    <article className={s.commitmentCard}>
      <h3 className={headlineClass}>{headline}</h3>

      {provenance && provenance.length > 0 && (
        <div className={s.commitmentMetaRow}>
          <strong>From:</strong>
          {provenance.map((p, i) => (
            <span key={i} style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
              {i > 0 && <span aria-hidden>·</span>}
              <CiteChip href={p.href}>{p.label}</CiteChip>
            </span>
          ))}
        </div>
      )}

      <div className={s.commitmentOwnerRow}>
        <span>
          <strong>Owner:</strong> {owner ?? "Unassigned"}
        </span>
        <span aria-hidden>·</span>
        <span>
          <strong>Due:</strong> {due ?? "no date set"}
        </span>
      </div>

      <div className={s.pillRow}>
        <AudiencePill variant={audience} />
        {visibility === "shared" ? (
          <VisibilityPill
            variant="shared"
            label={sharedRef ? `Shared · ${sharedRef.label}` : "Shared"}
            href={sharedRef?.href}
          />
        ) : (
          <VisibilityPill variant="private" />
        )}
      </div>

      {linearStatus && <div className={s.linearStatusNote}>{linearStatus}</div>}

      {stillActiveNote && (
        <div className={s.softNudge}>
          <span className={s.softNudgeLabel}>Still active?</span>
          {stillActiveNote}
        </div>
      )}

      <div className={s.commitmentActions}>{actions}</div>
    </article>
  );
}

/* ─────────────────────────────────────────────────────────────────────────
 * SuggestionCard — Chapter 4 "Suggestions" (saffron background).
 * Accept promotes to a commitment. Dismiss teaches the system (Bayesian).
 * Dismissals are NEVER "delete" — they are feedback.
 * ──────────────────────────────────────────────────────────────────────── */
export interface SuggestionCardProps {
  label?: string;
  headline: string;
  rationale: string;
  provenance?: { label: string; href?: string }[];
  onAccept?: () => void;
  onDismiss?: () => void;
  accepting?: boolean;
  dismissing?: boolean;
}

export function SuggestionCard({
  label = "Suggestion · one-click accept",
  headline,
  rationale,
  provenance,
  onAccept,
  onDismiss,
  accepting,
  dismissing,
}: SuggestionCardProps) {
  return (
    <article className={s.recCard}>
      <div className={s.recLabel}>{label}</div>
      <h3 className={s.recHeadline}>{headline}</h3>
      <p className={s.recRationale}>{rationale}</p>
      {provenance && provenance.length > 0 && (
        <div className={s.recProvenance}>
          <span>Drafted from</span>
          {provenance.map((p, i) => (
            <CiteChip key={i} href={p.href}>
              {p.label}
            </CiteChip>
          ))}
        </div>
      )}
      <div className={s.recActions}>
        <WorkButton kind="accent" onClick={onAccept} disabled={accepting}>
          {accepting ? "Accepting…" : "Accept → create commitment"}
        </WorkButton>
        <WorkButton kind="muted" onClick={onDismiss} disabled={dismissing}>
          {dismissing ? "Dismissing…" : "Dismiss (teaches system)"}
        </WorkButton>
        <span className={s.recDismissHint}>Dismissals feed Bayesian weights</span>
      </div>
    </article>
  );
}

/* ─────────────────────────────────────────────────────────────────────────
 * SharedRefRow — Chapter 5 "Shared with the team".
 * Mirror of externally-visible state — Linear, Salesforce, Slack.
 * ──────────────────────────────────────────────────────────────────────── */
export interface SharedRefRowProps {
  id: string;
  href?: string;
  body: ReactNode;
  subline?: ReactNode;
  meta?: string;
}

export function SharedRefRow({ id, href, body, subline, meta }: SharedRefRowProps) {
  return (
    <div className={s.refRow}>
      {href ? (
        <a href={href} className={s.refRowId} target="_blank" rel="noreferrer">
          {id}
        </a>
      ) : (
        <span className={s.refRowId}>{id}</span>
      )}
      <div className={s.refRowBody}>
        {body}
        {subline && <div className={s.refRowSub}>{subline}</div>}
      </div>
      {meta && <span className={s.refRowMeta}>{meta}</span>}
    </div>
  );
}

export function SharedSubsectionLabel({ children }: { children: ReactNode }) {
  return <div className={s.sharedSubsectionLabel}>{children}</div>;
}

/* ─────────────────────────────────────────────────────────────────────────
 * RecentlyLandedRow — Chapter 6 "Recently landed" (30-day tail).
 * ──────────────────────────────────────────────────────────────────────── */
export interface RecentlyLandedRowProps {
  date: string;
  event: string;
  source?: ReactNode;
  status?: string;
}

export function RecentlyLandedRow({ date, event, source, status = "Delivered" }: RecentlyLandedRowProps) {
  return (
    <div className={s.timelineRow}>
      <div className={s.timelineDate}>{date}</div>
      <div>
        <div className={s.timelineEvent}>{event}</div>
        {source && <div className={s.timelineSource}>{source}</div>}
      </div>
      <span className={`${s.refRowMeta} ${s.timelineDelivered}`}>{status}</span>
    </div>
  );
}

export function RecentlyLandedList({ children }: { children: ReactNode }) {
  return <div className={s.timelineList}>{children}</div>;
}

/* ─────────────────────────────────────────────────────────────────────────
 * ReportCard — Chapter 7 "Outputs". Links out to the Report Engine.
 * ──────────────────────────────────────────────────────────────────────── */
export interface ReportCardProps {
  type: string;
  title: string;
  generatedAt?: string;
  trigger?: string;
  onOpen?: () => void;
  onRefresh?: () => void;
  onExport?: () => void;
}

export function ReportCard({
  type,
  title,
  generatedAt,
  trigger,
  onOpen,
  onRefresh,
  onExport,
}: ReportCardProps) {
  return (
    <article className={s.reportCard}>
      <div className={s.reportType}>{type}</div>
      <h3 className={s.reportTitle}>{title}</h3>
      <div className={s.reportGen}>
        {generatedAt && <>Generated {generatedAt}<br /></>}
        {trigger && <>Trigger: {trigger}</>}
      </div>
      <div className={s.reportActions}>
        {onOpen && (
          <WorkButton kind="primary" onClick={onOpen}>
            Open
          </WorkButton>
        )}
        {onRefresh && <WorkButton onClick={onRefresh}>Refresh</WorkButton>}
        {onExport && <WorkButton onClick={onExport}>Export to Google Doc</WorkButton>}
      </div>
    </article>
  );
}

export function ReportGrid({ children }: { children: ReactNode }) {
  return <div className={s.reportGrid}>{children}</div>;
}

export function ReportFooterNote({ children }: { children: ReactNode }) {
  return <p className={s.reportFooterNote}>{children}</p>;
}

/* ─────────────────────────────────────────────────────────────────────────
 * NudgeRow — Chapter 8 "Nudges". Soft meta, hidden when empty.
 * Every nudge must offer a "Leave as-is" exit as an equal-valid action.
 * ──────────────────────────────────────────────────────────────────────── */
export interface NudgeRowProps {
  headline: string;
  body: string;
  actions: ReactNode;
}

export function NudgeRow({ headline, body, actions }: NudgeRowProps) {
  return (
    <article className={s.nudgeRow}>
      <h3 className={s.nudgeHeadline}>{headline}</h3>
      <p className={s.nudgeBody}>{body}</p>
      <div className={s.nudgeActions}>{actions}</div>
    </article>
  );
}

export function NudgeList({ children }: { children: ReactNode }) {
  return <div className={s.nudgeList}>{children}</div>;
}

/**
 * NudgeLeaveAsIs — the zero-guilt exit rendered as italic editorial prose,
 * not a button. Doing nothing IS "leave as-is"; rendering it as an
 * interactive CTA would imply action is required. See DOS-13 Wave 0g
 * Finding 1 (Option B).
 */
export function NudgeLeaveAsIs({ children = "Or leave as-is." }: { children?: ReactNode }) {
  return <span className={s.nudgeLeaveAsIs}>{children}</span>;
}

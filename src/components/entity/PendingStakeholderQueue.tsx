/**
 * PendingStakeholderQueue — DOS-258 Lane F
 *
 * Renders a review queue for account_stakeholders rows with
 * status='pending_review'. Each card shows the person's name, email,
 * confidence score, and data source with Confirm / Dismiss actions.
 *
 * Hidden entirely when the queue is empty — no empty-state clutter.
 * Dismiss fires immediately (no confirmation modal); Confirm promotes
 * the row to status='active'. Both use optimistic removal from the hook.
 */
import type { PendingStakeholderSuggestion } from "@/types";
import type { UsePendingStakeholdersResult } from "@/hooks/usePendingStakeholders";
import css from "./PendingStakeholderQueue.module.css";

interface PendingStakeholderQueueProps {
  queue: UsePendingStakeholdersResult;
}

export function PendingStakeholderQueue({ queue }: PendingStakeholderQueueProps) {
  const { suggestions, confirm, dismiss, inFlight } = queue;

  if (suggestions.length === 0) return null;

  return (
    <section className={css.section}>
      <div className={css.label}>
        New from recent activity
        <span className={css.labelHint}>
          {suggestions.length} person{suggestions.length === 1 ? "" : "s"} spotted in meetings or email — confirm to add to The Room
        </span>
      </div>
      <div className={css.list}>
        {suggestions.map((s) => (
          <PendingCard
            key={s.personId}
            suggestion={s}
            busy={inFlight.has(s.personId)}
            onConfirm={() => void confirm(s.personId)}
            onDismiss={() => void dismiss(s.personId)}
          />
        ))}
      </div>
    </section>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

interface PendingCardProps {
  suggestion: PendingStakeholderSuggestion;
  busy: boolean;
  onConfirm: () => void;
  onDismiss: () => void;
}

function PendingCard({ suggestion, busy, onConfirm, onDismiss }: PendingCardProps) {
  const displayName = suggestion.name ?? "Unknown person";
  const sourceLabel = formatDataSource(suggestion.dataSource);
  const confidenceLabel =
    suggestion.confidence != null
      ? `${Math.round(suggestion.confidence * 100)}% confidence`
      : null;

  const metaParts: string[] = [];
  if (sourceLabel) metaParts.push(sourceLabel);
  if (suggestion.email) metaParts.push(suggestion.email);
  if (confidenceLabel) metaParts.push(confidenceLabel);

  const siblings = suggestion.siblingAccountHints ?? [];

  return (
    <div className={css.card}>
      <div className={css.cardBody}>
        <div className={css.name}>{displayName}</div>
        {metaParts.length > 0 && (
          <div className={css.meta}>
            {metaParts.map((part, i) => (
              <span key={i}>{part}</span>
            ))}
          </div>
        )}
        {siblings.length > 0 && (
          <div className={css.siblingHint}>
            Also relevant to:{" "}
            {siblings
              .map(([, name]) => name)
              .join(", ")}
          </div>
        )}
      </div>
      <div className={css.actions}>
        <button
          type="button"
          className={css.btnDismiss}
          disabled={busy}
          onClick={onDismiss}
        >
          Dismiss
        </button>
        <button
          type="button"
          className={css.btnConfirm}
          disabled={busy}
          onClick={onConfirm}
        >
          Confirm
        </button>
      </div>
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

/**
 * Maps data_source values from account_stakeholders to human-readable labels.
 * New source types can be added here as the auto-suggest engine expands.
 */
function formatDataSource(source: string | null | undefined): string | null {
  if (!source) return null;
  switch (source.toLowerCase()) {
    case "meeting_attendance":
    case "calendar":
    case "gong":
      return "from recent meeting";
    case "email":
    case "email_correspondence":
    case "gmail":
      return "from recent email";
    case "glean":
      return "from Glean";
    case "user":
      return null; // user-added rows won't be pending_review, but handle gracefully
    default:
      return `from ${source}`;
  }
}

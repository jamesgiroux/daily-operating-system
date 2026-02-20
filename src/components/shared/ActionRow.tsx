/**
 * Shared ActionRow — renders an action with variant-appropriate density.
 *
 * Consolidates duplicate implementations from:
 * - TheWork.tsx (compact: link-only, accent bar, due date)
 * - ActionsPage.tsx (full: checkbox, context line, priority badge)
 *
 * ADR-0084 C1.
 */
import { Link } from "@tanstack/react-router";

interface ActionRowCompactProps {
  variant: "compact";
  action: { id: string; title: string; dueDate?: string; source?: string };
  accentColor?: string;
  dateColor?: string;
  bold?: boolean;
  formatDate?: (d: string) => string;
}

interface ActionRowFullProps {
  variant: "full";
  action: {
    id: string;
    title: string;
    status: string;
    priority: string;
    dueDate?: string | null;
    accountName?: string | null;
    accountId?: string | null;
    sourceLabel?: string | null;
  };
  onToggle: () => void;
  showBorder?: boolean;
  formatDate?: (d: string) => string;
  stripMarkdown?: (s: string) => string;
}

interface ActionRowOutcomeProps {
  variant: "outcome";
  action: {
    id: string;
    title: string;
    status: string;
    priority: string;
    dueDate?: string | null;
  };
  onComplete: () => void;
  onAccept: () => void;
  onReject: () => void;
  onCyclePriority: () => void;
}

export type ActionRowProps = ActionRowCompactProps | ActionRowFullProps | ActionRowOutcomeProps;

function defaultFormatDate(d: string): string {
  try {
    const dt = new Date(d);
    return dt.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  } catch {
    return d;
  }
}

function defaultStripMarkdown(s: string): string {
  return s.replace(/\*\*/g, "").replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
}

/** Compact variant: link-only row with optional accent bar (TheWork style) */
function CompactActionRow({
  action,
  accentColor,
  dateColor = "var(--color-text-tertiary)",
  bold,
  formatDate = defaultFormatDate,
}: ActionRowCompactProps) {
  return (
    <Link
      to="/actions/$actionId"
      params={{ actionId: action.id }}
      style={{
        display: "block",
        position: "relative",
        padding: "14px 0 14px 20px",
        borderBottom: "1px solid var(--color-rule-light)",
        textDecoration: "none",
        color: "inherit",
      }}
    >
      {accentColor && (
        <div
          style={{
            position: "absolute",
            left: 0,
            top: 14,
            bottom: 14,
            width: 3,
            borderRadius: 2,
            background: accentColor,
          }}
        />
      )}
      <div
        style={{
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          lineHeight: 1.55,
          fontWeight: bold ? 500 : 400,
          color: "var(--color-text-primary)",
        }}
      >
        {action.title}
      </div>
      {(action.dueDate || action.source) && (
        <div style={{ display: "flex", gap: 16, marginTop: 4 }}>
          {action.dueDate && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                letterSpacing: "0.04em",
                color: dateColor,
              }}
            >
              {formatDate(action.dueDate)}
            </span>
          )}
          {action.source && (
            <span style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-text-tertiary)" }}>
              {action.source}
            </span>
          )}
        </div>
      )}
    </Link>
  );
}

/** Full variant: checkbox + context + priority badge (ActionsPage style) */
function FullActionRow({
  action,
  onToggle,
  showBorder = true,
  formatDate = defaultFormatDate,
  stripMarkdown = defaultStripMarkdown,
}: ActionRowFullProps) {
  const isCompleted = action.status === "completed";
  const isOverdue =
    action.dueDate &&
    action.status === "pending" &&
    new Date(action.dueDate) < new Date();

  const contextParts: string[] = [];
  if (isOverdue && action.dueDate) {
    const days = Math.floor(
      (new Date().getTime() - new Date(action.dueDate).getTime()) / (1000 * 60 * 60 * 24),
    );
    if (days > 0) contextParts.push(`${days} day${days !== 1 ? "s" : ""} overdue`);
  } else if (action.dueDate) {
    contextParts.push(formatDate(action.dueDate));
  }
  if (action.accountName || action.accountId) {
    contextParts.push((action.accountName || action.accountId)!);
  }
  if (action.sourceLabel) contextParts.push(action.sourceLabel);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "14px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        opacity: isCompleted ? 0.4 : 1,
        transition: "opacity 0.15s ease",
      }}
    >
      <button
        onClick={onToggle}
        style={{
          width: 20,
          height: 20,
          borderRadius: 10,
          border: `2px solid ${isOverdue ? "var(--color-spice-terracotta)" : "var(--color-rule-heavy)"}`,
          background: isCompleted ? "var(--color-garden-sage)" : "transparent",
          cursor: "pointer",
          flexShrink: 0,
          marginTop: 2,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          transition: "all 0.15s ease",
        }}
      >
        {isCompleted && (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </button>
      <div style={{ flex: 1, minWidth: 0 }}>
        <Link
          to="/actions/$actionId"
          params={{ actionId: action.id }}
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 17,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            textDecoration: isCompleted ? "line-through" : "none",
            lineHeight: 1.4,
          }}
        >
          {stripMarkdown(action.title)}
        </Link>
        {contextParts.length > 0 && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: isOverdue ? 500 : 300,
              color: isOverdue ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
              marginTop: 2,
            }}
          >
            {contextParts.join(" \u00B7 ")}
          </div>
        )}
      </div>
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.04em",
          color:
            action.priority === "P1"
              ? "var(--color-spice-terracotta)"
              : action.priority === "P2"
                ? "var(--color-spice-turmeric)"
                : "var(--color-text-tertiary)",
          flexShrink: 0,
          marginTop: 4,
        }}
      >
        {action.priority}
      </span>
    </div>
  );
}

/** Outcome variant: compact triage row with accept/reject for proposed, checkbox + priority cycling (MeetingDetailPage style) */
function OutcomeActionRow({
  action,
  onComplete,
  onAccept,
  onReject,
  onCyclePriority,
}: ActionRowOutcomeProps) {
  const isCompleted = action.status === "completed";
  const isProposed = action.status === "proposed";

  const priorityColor: Record<string, string> = {
    P1: "var(--color-spice-terracotta)",
    P3: "var(--color-text-tertiary)",
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "3px 4px",
        borderLeft: isProposed ? "2px dashed var(--color-spice-turmeric)" : "none",
        paddingLeft: isProposed ? 8 : 4,
      }}
    >
      {isProposed ? (
        <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
          <button
            onClick={onAccept}
            title="Accept"
            style={{
              width: 20, height: 20, borderRadius: 3,
              border: "1px solid var(--color-garden-sage)",
              background: "transparent", cursor: "pointer",
              display: "flex", alignItems: "center", justifyContent: "center", padding: 0,
            }}
          >
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
              <path d="M2.5 6L5 8.5L9.5 4" stroke="var(--color-garden-sage)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
          <button
            onClick={onReject}
            title="Reject"
            style={{
              width: 20, height: 20, borderRadius: 3,
              border: "1px solid var(--color-spice-terracotta)",
              background: "transparent", cursor: "pointer",
              display: "flex", alignItems: "center", justifyContent: "center", padding: 0,
            }}
          >
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
              <path d="M3 3L9 9M9 3L3 9" stroke="var(--color-spice-terracotta)" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      ) : (
        <button
          onClick={onComplete}
          style={{
            width: 16, height: 16, borderRadius: 3,
            border: isCompleted ? "1px solid var(--color-garden-sage)" : "1px solid var(--color-text-tertiary)",
            background: isCompleted ? "rgba(126, 170, 123, 0.2)" : "transparent",
            cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center",
            flexShrink: 0, padding: 0,
          }}
        >
          {isCompleted && (
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M2.5 6L5 8.5L9.5 4" stroke="var(--color-garden-sage)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          )}
        </button>
      )}

      {isProposed ? (
        <span style={{
          fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600,
          letterSpacing: "0.06em", textTransform: "uppercase",
          color: "var(--color-spice-turmeric)",
        }}>
          Suggested
        </span>
      ) : (
        <button
          onClick={onCyclePriority}
          style={{
            fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 500,
            letterSpacing: "0.04em", padding: "1px 6px",
            border: "1px solid var(--color-rule-light)", borderRadius: 3,
            background: "transparent",
            color: priorityColor[action.priority] ?? "var(--color-text-secondary)",
            cursor: "pointer",
          }}
        >
          {action.priority}
        </button>
      )}

      <span style={{
        flex: 1, fontSize: 13,
        color: isCompleted ? "var(--color-text-tertiary)" : "var(--color-text-primary)",
        textDecoration: isCompleted ? "line-through" : "none",
      }}>
        {action.title}
      </span>

      {action.dueDate && (
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--color-text-tertiary)" }}>
          {action.dueDate}
        </span>
      )}
    </div>
  );
}

export function ActionRow(props: ActionRowProps) {
  if (props.variant === "compact") return <CompactActionRow {...props} />;
  if (props.variant === "outcome") return <OutcomeActionRow {...props} />;
  return <FullActionRow {...props} />;
}

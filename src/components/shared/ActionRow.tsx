/**
 * Shared ActionRow — renders an action with variant-appropriate density.
 *
 * Consolidates duplicate implementations from:
 * - TheWork.tsx (compact: link-only, accent bar, due date)
 * - ActionsPage.tsx (full: checkbox, context line, priority badge)
 *
 * ADR-0084 C1.
 */
import { useState, useRef, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { toast } from "sonner";
import type { LinearPushResult } from "@/types";

function priorityLabel(p: number | string): string {
  const v = typeof p === "string" ? parseInt(p, 10) : p;
  switch (v) {
    case 0: return "—";
    case 1: return "Urgent";
    case 2: return "High";
    case 3: return "Medium";
    case 4: return "Low";
    default: return "Medium";
  }
}

/** Map internal status codes to user-facing labels (DOS-52). */
export function statusLabel(status: string): string {
  switch (status) {
    case "backlog": return "Suggested";
    case "unstarted": return "Active";
    case "started": return "In Progress";
    case "completed": return "Completed";
    case "cancelled": return "Cancelled";
    case "archived": return "Archived";
    default: return status;
  }
}

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
    priority: number;
    dueDate?: string | null;
    context?: string | null;
    accountName?: string | null;
    accountId?: string | null;
    sourceLabel?: string | null;
    needsDecision?: boolean;
    linearIdentifier?: string | null;
    linearUrl?: string | null;
  };
  onToggle: () => void;
  onLinearPush?: () => void;
  linearEnabled?: boolean;
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
    priority: number;
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
  onLinearPush,
  linearEnabled,
  showBorder = true,
  formatDate = defaultFormatDate,
  stripMarkdown = defaultStripMarkdown,
}: ActionRowFullProps) {
  const [hovered, setHovered] = useState(false);
  const [pushing, setPushing] = useState(false);
  const teamIdRef = useRef<string | null>(null);

  const isCompleted = action.status === "completed";
  const isOverdue =
    action.dueDate &&
    (action.status === "unstarted" || action.status === "started") &&
    new Date(action.dueDate) < new Date();

  // Eligible for push: not yet linked, active status, Linear enabled
  const canPush =
    linearEnabled &&
    !action.linearIdentifier &&
    (action.status === "backlog" || action.status === "unstarted");

  const handlePush = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (pushing) return;
    setPushing(true);
    try {
      // Cache team ID on first use
      if (!teamIdRef.current) {
        const teams = await invoke<Array<{ id: string; name: string }>>("get_linear_teams");
        if (!teams.length) {
          toast.error("No Linear teams found");
          return;
        }
        teamIdRef.current = teams[0].id;
      }
      const result = await invoke<LinearPushResult>("push_action_to_linear", {
        actionId: action.id,
        teamId: teamIdRef.current,
      });
      toast.success(
        <span>
          Pushed as{" "}
          <span
            style={{
              fontFamily: "var(--font-mono)",
              cursor: "pointer",
              textDecoration: "underline",
            }}
            onClick={() => open(result.url)}
          >
            {result.identifier}
          </span>
        </span>
      );
      onLinearPush?.();
    } catch (err) {
      toast.error(`Push failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setPushing(false);
    }
  }, [action.id, pushing, onLinearPush]);

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
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
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
        {action.needsDecision && (
          <span
            style={{
              display: "inline-block",
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 600,
              letterSpacing: "0.04em",
              textTransform: "uppercase",
              color: "var(--color-spice-turmeric)",
              background: "var(--color-spice-saffron-12)",
              borderRadius: 3,
              padding: "1px 6px",
              marginLeft: 8,
              verticalAlign: "middle",
            }}
          >
            Decision needed
          </span>
        )}
        {action.linearIdentifier && (
          <span
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (action.linearUrl) open(action.linearUrl);
            }}
            style={{
              display: "inline-block",
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 500,
              letterSpacing: "0.04em",
              color: "var(--color-text-tertiary)",
              borderRadius: 3,
              padding: "1px 6px",
              marginLeft: 8,
              verticalAlign: "middle",
              cursor: "pointer",
              transition: "color 0.1s ease",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.color = "var(--color-text-secondary)")}
            onMouseLeave={(e) => (e.currentTarget.style.color = "var(--color-text-tertiary)")}
            title={`Open ${action.linearIdentifier} in Linear`}
          >
            {action.linearIdentifier}
          </span>
        )}
        {action.context && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              fontWeight: 400,
              color: "var(--color-text-secondary)",
              marginTop: 4,
              lineHeight: 1.45,
            }}
          >
            {stripMarkdown(action.context)}
          </div>
        )}
        {contextParts.length > 0 && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: isOverdue ? 500 : 300,
              color: isOverdue ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
              marginTop: action.context ? 6 : 2,
            }}
          >
            {contextParts.join(" \u00B7 ")}
          </div>
        )}
      </div>
      {/* Push-to-Linear button: hover-reveal when eligible */}
      {canPush && hovered && (
        <button
          onClick={handlePush}
          disabled={pushing}
          title="Push to Linear"
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            letterSpacing: "0.04em",
            color: pushing ? "var(--color-text-tertiary)" : "var(--color-text-secondary)",
            background: "transparent",
            border: "1px solid var(--color-rule-light)",
            borderRadius: 3,
            padding: "2px 8px",
            cursor: pushing ? "wait" : "pointer",
            flexShrink: 0,
            marginTop: 4,
            transition: "all 0.1s ease",
            opacity: pushing ? 0.6 : 1,
          }}
        >
          {pushing ? "..." : "Linear"}
        </button>
      )}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.04em",
          color:
            action.priority <= 1
              ? "var(--color-spice-terracotta)"
              : action.priority <= 2
                ? "var(--color-spice-turmeric)"
                : "var(--color-text-tertiary)",
          flexShrink: 0,
          marginTop: 4,
        }}
      >
        {priorityLabel(action.priority)}
      </span>
    </div>
  );
}

/** Outcome variant: compact triage row with accept/reject for suggested, checkbox + priority cycling (MeetingDetailPage style) */
function OutcomeActionRow({
  action,
  onComplete,
  onAccept,
  onReject,
  onCyclePriority,
}: ActionRowOutcomeProps) {
  const isCompleted = action.status === "completed";
  const isSuggested = action.status === "backlog";

  const priorityColor: Record<string, string> = {
    1: "var(--color-spice-terracotta)",
    4: "var(--color-text-tertiary)",
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "3px 4px",
        borderLeft: isSuggested ? "2px dashed var(--color-spice-turmeric)" : "none",
        paddingLeft: isSuggested ? 8 : 4,
      }}
    >
      {isSuggested ? (
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
            title="Dismiss"
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
            background: isCompleted ? "var(--color-garden-sage-20)" : "transparent",
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

      {isSuggested ? (
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
          {priorityLabel(action.priority)}
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

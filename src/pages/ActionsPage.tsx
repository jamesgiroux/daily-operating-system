import { useState, useMemo } from "react";
import { useSearch, Link } from "@tanstack/react-router";
import { useActions } from "@/hooks/useActions";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { PriorityPicker } from "@/components/ui/priority-picker";
import { EntityPicker } from "@/components/ui/entity-picker";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import type { CreateActionParams } from "@/hooks/useActions";
import type { DbAction } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import { stripMarkdown } from "@/lib/utils";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";

type StatusTab = "pending" | "completed" | "waiting" | "all";
type PriorityTab = "all" | "P1" | "P2" | "P3";

const statusTabs: StatusTab[] = ["pending", "completed", "waiting", "all"];
const priorityTabs: PriorityTab[] = ["all", "P1", "P2", "P3"];

export default function ActionsPage() {
  const { personality } = usePersonality();
  const { search: initialSearch } = useSearch({ strict: false });
  const {
    actions,
    allActions,
    loading,
    error,
    refresh,
    createAction,
    toggleAction,
    statusFilter,
    setStatusFilter,
    priorityFilter,
    setPriorityFilter,
    searchQuery,
    setSearchQuery,
  } = useActions(initialSearch as string | undefined);

  const [showCreate, setShowCreate] = useState(false);

  // Computed stats
  const pendingCount = allActions.filter((a) => a.status === "pending").length;
  const overdueCount = allActions.filter(
    (a) => a.status === "pending" && a.dueDate && new Date(a.dueDate) < new Date()
  ).length;

  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // FolioBar readiness stats
  const folioStats = useMemo((): ReadinessStat[] => {
    const stats: ReadinessStat[] = [];
    if (pendingCount > 0) stats.push({ label: `${pendingCount} pending`, color: "sage" });
    if (overdueCount > 0) stats.push({ label: `${overdueCount} overdue`, color: "terracotta" });
    return stats;
  }, [pendingCount, overdueCount]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Actions",
      atmosphereColor: "terracotta" as const,
      activePage: "actions" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioStats,
      folioActions: (
        <button
          onClick={() => setShowCreate(true)}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase" as const,
            color: "var(--color-spice-terracotta)",
            background: "none",
            border: "1px solid var(--color-spice-terracotta)",
            borderRadius: 4,
            padding: "2px 10px",
            cursor: "pointer",
          }}
        >
          + Add
        </button>
      ),
    }),
    [formattedDate, folioStats],
  );
  useRegisterMagazineShell(shellConfig);

  // Loading state
  if (loading) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        {[1, 2, 3, 4].map((i) => (
          <div
            key={i}
            style={{
              height: 60,
              background: "var(--color-rule-light)",
              borderRadius: 8,
              marginBottom: 12,
              animation: "pulse 1.5s ease-in-out infinite",
            }}
          />
        ))}
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80, textAlign: "center" }}>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-spice-terracotta)" }}>
          {error}
        </p>
        <button
          onClick={refresh}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "1px solid var(--color-rule-heavy)",
            borderRadius: 4,
            padding: "4px 12px",
            cursor: "pointer",
            marginTop: 12,
          }}
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ PAGE HEADER ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 24 }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <h1
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 36,
              fontWeight: 400,
              letterSpacing: "-0.02em",
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Actions
          </h1>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
            }}
          >
            {actions.length} item{actions.length !== 1 ? "s" : ""}
          </span>
        </div>

        {/* Section rule */}
        <div style={{ height: 1, background: "var(--color-rule-heavy)", marginTop: 16, marginBottom: 16 }} />

        {/* Status filter toggles */}
        <div style={{ display: "flex", gap: 20, marginBottom: 12 }}>
          {statusTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => setStatusFilter(tab)}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 500,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: statusFilter === tab ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
                textDecoration: statusFilter === tab ? "underline" : "none",
                textUnderlineOffset: "4px",
                background: "none",
                border: "none",
                padding: 0,
                cursor: "pointer",
                transition: "color 0.15s ease",
              }}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Priority filter toggles */}
        <div style={{ display: "flex", gap: 20, marginBottom: 16 }}>
          {priorityTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => setPriorityFilter(tab)}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 500,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: priorityFilter === tab ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
                textDecoration: priorityFilter === tab ? "underline" : "none",
                textUnderlineOffset: "4px",
                background: "none",
                border: "none",
                padding: 0,
                cursor: "pointer",
                transition: "color 0.15s ease",
              }}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Search */}
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="⌘  Search actions..."
          style={{
            width: "100%",
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-primary)",
            background: "none",
            border: "none",
            borderBottom: "1px solid var(--color-rule-light)",
            padding: "8px 0",
            outline: "none",
          }}
        />
      </section>

      {/* ═══ CREATE FORM ═══ */}
      {showCreate && (
        <ActionCreateForm
          onSubmit={async (params) => {
            await createAction(params);
            setShowCreate(false);
          }}
          onCancel={() => setShowCreate(false)}
        />
      )}

      {/* ═══ ACTION ROWS ═══ */}
      <section>
        {actions.length === 0 ? (
          <EditorialEmpty
            {...getPersonalityCopy(
              statusFilter === "completed"
                ? "actions-completed-empty"
                : statusFilter === "waiting"
                  ? "actions-waiting-empty"
                  : "actions-empty",
              personality,
            )}
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {actions.map((action, i) => (
              <ActionRow
                key={action.id}
                action={action}
                onToggle={() => toggleAction(action.id)}
                showBorder={i < actions.length - 1}
              />
            ))}
          </div>
        )}
      </section>

      {/* ═══ END MARK ═══ */}
      {actions.length > 0 && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-heavy)",
            marginTop: 48,
            paddingTop: 32,
            paddingBottom: 120,
            textAlign: "center",
          }}
        >
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 14,
              fontStyle: "italic",
              color: "var(--color-text-tertiary)",
            }}
          >
            That's everything.
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Action Row ─────────────────────────────────────────────────────────────

function ActionRow({
  action,
  onToggle,
  showBorder,
}: {
  action: DbAction;
  onToggle: () => void;
  showBorder: boolean;
}) {
  const isCompleted = action.status === "completed";
  const isOverdue =
    action.dueDate &&
    action.status === "pending" &&
    new Date(action.dueDate) < new Date();

  // Context line parts
  const contextParts: string[] = [];
  if (isOverdue && action.dueDate) {
    const days = Math.floor(
      (new Date().getTime() - new Date(action.dueDate).getTime()) / (1000 * 60 * 60 * 24)
    );
    if (days > 0) contextParts.push(`${days} day${days !== 1 ? "s" : ""} overdue`);
  } else if (action.dueDate) {
    contextParts.push(formatDueDate(action.dueDate));
  }
  if (action.accountName || action.accountId) {
    contextParts.push(action.accountName || action.accountId!);
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
      {/* Checkbox */}
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

      {/* Content */}
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
              color: isOverdue
                ? "var(--color-spice-terracotta)"
                : "var(--color-text-tertiary)",
              marginTop: 2,
            }}
          >
            {contextParts.join(" \u00B7 ")}
          </div>
        )}
      </div>

      {/* Priority badge */}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.04em",
          color: action.priority === "P1"
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

// ─── Create Form (editorial style) ─────────────────────────────────────────

function ActionCreateForm({
  onSubmit,
  onCancel,
  defaultAccountId,
}: {
  onSubmit: (params: CreateActionParams) => Promise<void>;
  onCancel: () => void;
  defaultAccountId?: string;
}) {
  const [title, setTitle] = useState("");
  const [showDetails, setShowDetails] = useState(false);
  const [priority, setPriority] = useState("P2");
  const [dueDate, setDueDate] = useState("");
  const [accountId, setAccountId] = useState<string | null>(defaultAccountId ?? null);
  const [sourceLabel, setSourceLabel] = useState("");
  const [context, setContext] = useState("");
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit() {
    if (!title.trim() || submitting) return;
    setSubmitting(true);
    try {
      await onSubmit({
        title: title.trim(),
        priority,
        dueDate: dueDate || undefined,
        accountId: accountId ?? undefined,
        context: context.trim() || undefined,
        sourceLabel: sourceLabel.trim() || undefined,
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div
      style={{
        borderBottom: "1px solid var(--color-rule-heavy)",
        paddingBottom: 20,
        marginBottom: 8,
      }}
    >
      {/* Title input */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <div
          style={{
            width: 20,
            height: 20,
            borderRadius: 10,
            border: "2px solid var(--color-rule-heavy)",
            flexShrink: 0,
          }}
        />
        <input
          type="text"
          autoFocus
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !showDetails) handleSubmit();
            if (e.key === "Escape") onCancel();
          }}
          placeholder="What needs to be done?"
          style={{
            flex: 1,
            fontFamily: "var(--font-serif)",
            fontSize: 17,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            background: "none",
            border: "none",
            outline: "none",
          }}
        />
      </div>

      {/* Details toggle */}
      {!showDetails ? (
        <div style={{ display: "flex", alignItems: "center", gap: 12, paddingLeft: 32 }}>
          <button
            type="button"
            onClick={() => setShowDetails(true)}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
            }}
          >
            + details
          </button>
          <div style={{ flex: 1 }} />
          <button
            onClick={handleSubmit}
            disabled={!title.trim() || submitting}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              color: !title.trim() ? "var(--color-text-tertiary)" : "var(--color-spice-terracotta)",
              background: "none",
              border: "1px solid",
              borderColor: !title.trim() ? "var(--color-rule-heavy)" : "var(--color-spice-terracotta)",
              borderRadius: 4,
              padding: "3px 12px",
              cursor: !title.trim() ? "default" : "pointer",
            }}
          >
            Create
          </button>
          <button
            onClick={onCancel}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
            }}
          >
            Cancel
          </button>
        </div>
      ) : (
        <div style={{ paddingLeft: 32 }}>
          <button
            type="button"
            onClick={() => setShowDetails(false)}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
              marginBottom: 12,
            }}
          >
            - hide details
          </button>

          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12, marginBottom: 12 }}>
            <PriorityPicker value={priority} onChange={setPriority} />
            <input
              type="date"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-secondary)",
                background: "none",
                border: "1px solid var(--color-rule-heavy)",
                borderRadius: 4,
                padding: "3px 8px",
                outline: "none",
              }}
            />
            <EntityPicker
              value={accountId}
              onChange={(id) => setAccountId(id)}
              locked={!!defaultAccountId}
            />
          </div>

          <input
            type="text"
            value={sourceLabel}
            onChange={(e) => setSourceLabel(e.target.value)}
            placeholder="Source (e.g., Slack, call with Jane)"
            style={{
              width: "100%",
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-primary)",
              background: "none",
              border: "none",
              borderBottom: "1px solid var(--color-rule-light)",
              padding: "6px 0",
              outline: "none",
              marginBottom: 8,
            }}
          />

          <textarea
            value={context}
            onChange={(e) => setContext(e.target.value)}
            placeholder="Additional context..."
            rows={2}
            style={{
              width: "100%",
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-primary)",
              background: "none",
              border: "none",
              borderBottom: "1px solid var(--color-rule-light)",
              padding: "6px 0",
              outline: "none",
              resize: "none",
              marginBottom: 12,
            }}
          />

          <div style={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: 12 }}>
            <button
              onClick={handleSubmit}
              disabled={!title.trim() || submitting}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                color: !title.trim() ? "var(--color-text-tertiary)" : "var(--color-spice-terracotta)",
                background: "none",
                border: "1px solid",
                borderColor: !title.trim() ? "var(--color-rule-heavy)" : "var(--color-spice-terracotta)",
                borderRadius: 4,
                padding: "3px 12px",
                cursor: !title.trim() ? "default" : "pointer",
              }}
            >
              Create
            </button>
            <button
              onClick={onCancel}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Date Formatting ────────────────────────────────────────────────────────

function formatDueDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.floor(
      (date.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
    );
    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Tomorrow";
    if (diffDays === -1) return "Yesterday";
    if (diffDays < -1) return `${Math.abs(diffDays)} days ago`;
    if (diffDays <= 7) return `In ${diffDays} days`;
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  } catch {
    return dateStr;
  }
}

import { useState, useMemo, useCallback } from "react";
import { useSearch } from "@tanstack/react-router";
import { useActions } from "@/hooks/useActions";
import { useProposedActions } from "@/hooks/useProposedActions";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { ActionRow as SharedActionRow } from "@/components/shared/ActionRow";
import { ProposedActionRow as SharedProposedActionRow } from "@/components/shared/ProposedActionRow";
import { PriorityPicker } from "@/components/ui/priority-picker";
import { EntityPicker } from "@/components/ui/entity-picker";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import type { CreateActionParams } from "@/hooks/useActions";
import type { DbAction } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import { stripMarkdown } from "@/lib/utils";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { DatePicker } from "@/components/ui/date-picker";

// ─── Action Group Types ──────────────────────────────────────────────────────

interface ActionGroup {
  label: string;
  /** "meeting" groups sort by meeting start; "time-band" groups are the fallback */
  kind: "meeting" | "time-band";
  actions: DbAction[];
}

/** Priority sort weight: P1 first */
const PRIORITY_WEIGHT: Record<string, number> = { P1: 0, P2: 1, P3: 2 };

function sortByPriorityThenDue(a: DbAction, b: DbAction): number {
  const pw = (PRIORITY_WEIGHT[a.priority] ?? 9) - (PRIORITY_WEIGHT[b.priority] ?? 9);
  if (pw !== 0) return pw;
  if (!a.dueDate && !b.dueDate) return 0;
  if (!a.dueDate) return 1;
  if (!b.dueDate) return -1;
  return new Date(a.dueDate).getTime() - new Date(b.dueDate).getTime();
}

/**
 * Format a meeting-group label: "Meeting Title . Day"
 * Uses relative day names for this week, date for further out.
 */
function formatMeetingGroupLabel(title: string, startIso: string): string {
  try {
    const date = new Date(startIso);
    const now = new Date();
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const diffDays = Math.round(
      (new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime() - todayStart.getTime()) /
        (1000 * 60 * 60 * 24)
    );

    let dayLabel: string;
    if (diffDays === 0) dayLabel = "Today";
    else if (diffDays === 1) dayLabel = "Tomorrow";
    else if (diffDays === -1) dayLabel = "Yesterday";
    else if (diffDays > 1 && diffDays <= 6) {
      dayLabel = date.toLocaleDateString(undefined, { weekday: "long" });
    } else {
      dayLabel = date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
    }

    return `${title} \u00b7 ${dayLabel}`;
  } catch {
    return title;
  }
}

/**
 * Group actions by meeting context (meeting-centric), with an
 * "Everything Else" fallback for actions not linked to a meeting.
 * The Everything Else section uses the old time-band sub-grouping.
 */
function groupByMeeting(actions: DbAction[]): ActionGroup[] {
  const meetingMap = new Map<string, { title: string; start: string; actions: DbAction[] }>();
  const everythingElse: DbAction[] = [];

  for (const a of actions) {
    if (a.nextMeetingTitle && a.nextMeetingStart) {
      const key = `${a.nextMeetingTitle}::${a.nextMeetingStart}`;
      if (!meetingMap.has(key)) {
        meetingMap.set(key, { title: a.nextMeetingTitle, start: a.nextMeetingStart, actions: [] });
      }
      meetingMap.get(key)!.actions.push(a);
    } else {
      everythingElse.push(a);
    }
  }

  // Sort meeting groups by start time ascending (soonest first)
  const meetingEntries = [...meetingMap.values()].sort(
    (a, b) => new Date(a.start).getTime() - new Date(b.start).getTime()
  );

  const groups: ActionGroup[] = [];

  for (const entry of meetingEntries) {
    entry.actions.sort(sortByPriorityThenDue);
    groups.push({
      label: formatMeetingGroupLabel(entry.title, entry.start),
      kind: "meeting",
      actions: entry.actions,
    });
  }

  // "Everything Else" — sub-grouped by time-band
  if (everythingElse.length > 0) {
    const now = new Date();
    const sevenDaysOut = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000);

    const overdue: DbAction[] = [];
    const thisWeek: DbAction[] = [];
    const later: DbAction[] = [];

    for (const a of everythingElse) {
      if (a.dueDate && new Date(a.dueDate) < now) {
        overdue.push(a);
      } else if (a.dueDate && new Date(a.dueDate) <= sevenDaysOut) {
        thisWeek.push(a);
      } else {
        later.push(a);
      }
    }

    const sortByDue = (x: DbAction, y: DbAction) => {
      if (!x.dueDate && !y.dueDate) return 0;
      if (!x.dueDate) return 1;
      if (!y.dueDate) return -1;
      return new Date(x.dueDate).getTime() - new Date(y.dueDate).getTime();
    };

    overdue.sort(sortByDue);
    thisWeek.sort(sortByDue);
    later.sort(sortByDue);

    if (overdue.length > 0) groups.push({ label: "Overdue", kind: "time-band", actions: overdue });
    if (thisWeek.length > 0) groups.push({ label: "This Week", kind: "time-band", actions: thisWeek });
    if (later.length > 0) groups.push({ label: "Later", kind: "time-band", actions: later });
  }

  return groups;
}

type StatusTab = "proposed" | "pending" | "completed";
type PriorityTab = "all" | "P1" | "P2" | "P3";

const statusTabs: StatusTab[] = ["proposed", "pending", "completed"];
const statusTabLabels: Record<StatusTab, string> = { proposed: "Suggested", pending: "Pending", completed: "Completed" };
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
  const { proposedActions, acceptAction, rejectAction } = useProposedActions();

  const [showCreate, setShowCreate] = useState(false);

  // Computed stats
  const proposedCount = proposedActions.length;
  const pendingCount = allActions.filter((a) => a.status === "pending").length;
  const overdueCount = allActions.filter(
    (a) => a.status === "pending" && a.dueDate && new Date(a.dueDate) < new Date()
  ).length;

  const handleAccept = useCallback(async (id: string) => {
    await acceptAction(id);
    refresh();
  }, [acceptAction, refresh]);

  const handleReject = useCallback(async (id: string) => {
    await rejectAction(id);
  }, [rejectAction]);

  // Smart default: proposed tab when suggestions exist, else pending
  const [hasSetDefault, setHasSetDefault] = useState(false);
  if (!hasSetDefault && !loading && proposedCount > 0 && statusFilter !== "proposed") {
    setStatusFilter("proposed");
    setHasSetDefault(true);
  } else if (!hasSetDefault && !loading && proposedCount === 0 && statusFilter !== "pending") {
    setStatusFilter("pending");
    setHasSetDefault(true);
  } else if (!hasSetDefault && !loading) {
    setHasSetDefault(true);
  }

  // FolioBar readiness stats
  const folioStats = useMemo((): ReadinessStat[] => {
    const stats: ReadinessStat[] = [];
    if (proposedCount > 0) stats.push({ label: `${proposedCount} to review`, color: "terracotta" });
    if (pendingCount > 0) stats.push({ label: `${pendingCount} pending`, color: "sage" });
    if (overdueCount > 0) stats.push({ label: `${overdueCount} overdue`, color: "terracotta" });
    return stats;
  }, [proposedCount, pendingCount, overdueCount]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Actions",
      atmosphereColor: "terracotta" as const,
      activePage: "actions" as const,
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
    [folioStats],
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
              marginBottom: "var(--space-sm)",
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
        <div style={{ display: "flex", gap: "var(--space-lg)", marginBottom: "var(--space-sm)" }}>
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
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              {statusTabLabels[tab]}
              {tab === "proposed" && proposedCount > 0 && (
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    fontWeight: 600,
                    color: "var(--color-spice-turmeric)",
                    background: "var(--color-spice-saffron-12)",
                    borderRadius: 8,
                    padding: "1px 6px",
                    lineHeight: "16px",
                  }}
                >
                  {proposedCount}
                </span>
              )}
            </button>
          ))}
        </div>

        {/* Priority filter toggles */}
        <div style={{ display: "flex", gap: "var(--space-lg)", marginBottom: "var(--space-md)" }}>
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
        {statusFilter === "proposed" ? (
          proposedActions.length === 0 ? (
            <EditorialEmpty
              title="All clear"
              message="No AI suggestions waiting for review."
            />
          ) : (
            <div style={{ display: "flex", flexDirection: "column" }}>
              {proposedActions.map((action, i) => (
                <SharedProposedActionRow
                  key={action.id}
                  action={action}
                  onAccept={() => handleAccept(action.id)}
                  onReject={() => handleReject(action.id)}
                  showBorder={i < proposedActions.length - 1}
                  stripMarkdown={stripMarkdown}
                />
              ))}
            </div>
          )
        ) : actions.length === 0 ? (
          <EditorialEmpty
            {...getPersonalityCopy(
              statusFilter === "completed"
                ? "actions-completed-empty"
                : "actions-empty",
              personality,
            )}
          />
        ) : statusFilter === "pending" ? (
          // Grouped view for pending tab
          <PendingGroupedView actions={actions} onToggle={toggleAction} />
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {actions.map((action, i) => (
              <SharedActionRow
                key={action.id}
                variant="full"
                action={action}
                onToggle={() => toggleAction(action.id)}
                showBorder={i < actions.length - 1}
                stripMarkdown={stripMarkdown}
                formatDate={formatDueDate}
              />
            ))}
          </div>
        )}
      </section>

      {/* ═══ END MARK ═══ */}
      {actions.length > 0 && <FinisMarker />}
    </div>
  );
}

// ─── Pending Grouped View ───────────────────────────────────────────────────

function PendingGroupedView({
  actions,
  onToggle,
}: {
  actions: DbAction[];
  onToggle: (id: string) => void;
}) {
  const groups = useMemo(() => groupByMeeting(actions), [actions]);

  const timeBandColors: Record<string, string> = {
    Overdue: "var(--color-spice-terracotta)",
    "This Week": "var(--color-spice-turmeric)",
    Later: "var(--color-text-tertiary)",
  };

  // Track when we transition from meeting groups to time-band (Everything Else)
  const firstTimeBandIdx = groups.findIndex((g) => g.kind === "time-band");
  const hasTimeBands = firstTimeBandIdx !== -1;
  const hasMeetingGroups = groups.some((g) => g.kind === "meeting");

  return (
    <div>
      {/* "Everything Else" section header — only if we have both meeting and time-band groups */}
      {groups.map((group, idx) => (
        <div key={group.label}>
          {hasTimeBands && hasMeetingGroups && idx === firstTimeBandIdx && (
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                letterSpacing: "0.10em",
                textTransform: "uppercase",
                color: "var(--color-text-tertiary)",
                paddingBottom: 8,
                borderBottom: "2px solid var(--color-rule-heavy)",
                marginBottom: 16,
                marginTop: 32,
              }}
            >
              Everything Else
            </div>
          )}
          <div style={{ marginBottom: 32 }}>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.08em",
                textTransform: "uppercase",
                color: group.kind === "meeting"
                  ? "var(--color-garden-larkspur)"
                  : timeBandColors[group.label] ?? "var(--color-text-tertiary)",
                paddingBottom: 8,
                borderBottom: "1px solid var(--color-rule-light)",
                marginBottom: 0,
              }}
            >
              {group.label}
              <span style={{ fontWeight: 400, opacity: 0.7, marginLeft: 8 }}>{group.actions.length}</span>
            </div>
            <div style={{ display: "flex", flexDirection: "column" }}>
              {group.actions.map((action, i) => (
                <SharedActionRow
                  key={action.id}
                  variant="full"
                  action={action}
                  onToggle={() => onToggle(action.id)}
                  showBorder={i < group.actions.length - 1}
                  stripMarkdown={stripMarkdown}
                  formatDate={formatDueDate}
                />
              ))}
            </div>
          </div>
        </div>
      ))}
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
            <DatePicker
              value={dueDate}
              onChange={setDueDate}
              placeholder="Due date"
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

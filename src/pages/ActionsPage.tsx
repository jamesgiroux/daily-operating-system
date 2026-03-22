import { useState, useMemo, useCallback, useRef, useEffect } from "react";
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
import { toast } from "sonner";
import { EmptyState } from "@/components/editorial/EmptyState";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditorialPageHeader } from "@/components/editorial/EditorialPageHeader";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { DatePicker } from "@/components/ui/date-picker";
import s from "./ActionsPage.module.css";

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

  // "Everything Else" — single group, sorted by due date ascending.
  // Overdue items naturally surface to the top without a guilt-inducing header.
  // Proper overdue handling (aging, zero-guilt prompts) deferred to v1.0.2 I583.
  if (everythingElse.length > 0) {
    everythingElse.sort((x, y) => {
      if (!x.dueDate && !y.dueDate) return 0;
      if (!x.dueDate) return 1;
      if (!y.dueDate) return -1;
      return new Date(x.dueDate).getTime() - new Date(y.dueDate).getTime();
    });

    groups.push({ label: "Everything Else", kind: "time-band", actions: everythingElse });
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
    await rejectAction(id, "actions_page");
  }, [rejectAction]);

  // Smart default: proposed tab when suggestions exist, else pending
  const [hasSetDefault, setHasSetDefault] = useState(false);
  const prevProposedCountRef = useRef(0);
  const userManuallySelectedTab = useRef(false);

  if (!hasSetDefault && !loading && proposedCount > 0 && statusFilter !== "proposed") {
    setStatusFilter("proposed");
    setHasSetDefault(true);
  } else if (!hasSetDefault && !loading && proposedCount === 0 && statusFilter !== "pending") {
    setStatusFilter("pending");
    setHasSetDefault(true);
  } else if (!hasSetDefault && !loading) {
    setHasSetDefault(true);
  }

  // Auto-switch to proposed tab when new proposals arrive (0 -> >0 transition)
  useEffect(() => {
    if (
      hasSetDefault &&
      prevProposedCountRef.current === 0 &&
      proposedCount > 0 &&
      !userManuallySelectedTab.current
    ) {
      setStatusFilter("proposed");
      toast.info(`${proposedCount} new suggested action${proposedCount !== 1 ? "s" : ""} to review`);
    }
    prevProposedCountRef.current = proposedCount;
  }, [proposedCount, hasSetDefault, setStatusFilter]);

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
          className={s.folioAddButton}
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
    return <EditorialLoading count={4} />;
  }

  // Error state
  if (error) {
    return <EditorialError message={error} onRetry={refresh} />;
  }

  return (
    <div className={s.pageContainer}>
      <EditorialPageHeader
        title="Actions"
        scale="standard"
        width="standard"
        rule="subtle"
        meta={`${actions.length} item${actions.length !== 1 ? "s" : ""}`}
      >
        <div className={s.tabRow}>
          {statusTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => {
                userManuallySelectedTab.current = true;
                setStatusFilter(tab);
              }}
              className={`${s.tabButton} ${statusFilter === tab ? s.tabButtonActive : ""}`}
            >
              <span className={s.statusTabInner}>
                {statusTabLabels[tab]}
                {tab === "proposed" && proposedCount > 0 && (
                  <span className={s.proposedBadge}>{proposedCount}</span>
                )}
              </span>
            </button>
          ))}
        </div>

        <div className={s.tabRowPriority}>
          {priorityTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => setPriorityFilter(tab)}
              className={`${s.tabButton} ${priorityFilter === tab ? s.tabButtonActive : ""}`}
            >
              {tab}
            </button>
          ))}
        </div>

        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="⌘  Search actions..."
          className={s.searchInput}
        />
      </EditorialPageHeader>

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
            <EmptyState
              headline="All clear"
              explanation="No AI suggestions waiting for review. New proposals surface from meetings and emails."
              benefit="Action items, captured without lifting a finger."
            />
          ) : (
            <div className={s.actionColumn}>
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
          (() => {
            const copy = getPersonalityCopy(
              statusFilter === "completed" ? "actions-completed-empty" : "actions-empty",
              personality,
            );
            return (
              <EmptyState
                headline={copy.title}
                explanation={copy.explanation ?? copy.message ?? ""}
                benefit={copy.benefit}
                action={statusFilter !== "completed" ? { label: "Add an action", onClick: () => setShowCreate(true) } : undefined}
              />
            );
          })()
        ) : statusFilter === "pending" ? (
          // Grouped view for pending tab
          <PendingGroupedView actions={actions} onToggle={toggleAction} />
        ) : (
          <div className={s.actionColumn}>
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
      <FinisMarker />
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

  return (
    <div>
      {groups.map((group) => (
        <div key={group.label}>
          <div className={s.groupBlock}>
            <ChapterHeading title={group.label} epigraph={`${group.actions.length} action${group.actions.length !== 1 ? "s" : ""}`} />
            <div className={s.actionColumn}>
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

  const createBtnClass = `${s.createButton} ${!title.trim() ? s.createButtonDisabled : s.createButtonEnabled}`;

  return (
    <div className={s.createForm}>
      {/* Title input */}
      <div className={s.createTitleRow}>
        <div className={s.createCheckCircle} />
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
          className={s.createTitleInput}
        />
      </div>

      {/* Details toggle */}
      {!showDetails ? (
        <div className={s.createActionsRow}>
          <button
            type="button"
            onClick={() => setShowDetails(true)}
            className={s.detailsToggle}
          >
            + details
          </button>
          <div className={s.spacer} />
          <button
            onClick={handleSubmit}
            disabled={!title.trim() || submitting}
            className={createBtnClass}
          >
            Create
          </button>
          <button onClick={onCancel} className={s.cancelButton}>
            Cancel
          </button>
        </div>
      ) : (
        <div className={s.createDetailsPanel}>
          <button
            type="button"
            onClick={() => setShowDetails(false)}
            className={s.detailsToggleMargin}
          >
            - hide details
          </button>

          <div className={s.detailsFieldsRow}>
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
            className={s.formInput}
          />

          <textarea
            value={context}
            onChange={(e) => setContext(e.target.value)}
            placeholder="Additional context..."
            rows={2}
            className={s.formTextarea}
          />

          <div className={s.createActionsEnd}>
            <button
              onClick={handleSubmit}
              disabled={!title.trim() || submitting}
              className={createBtnClass}
            >
              Create
            </button>
            <button onClick={onCancel} className={s.cancelButton}>
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

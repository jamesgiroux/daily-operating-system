import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { useParams, Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { PriorityPicker } from "@/components/ui/priority-picker";
import { EntityPicker } from "@/components/ui/entity-picker";
import { EditableInline } from "@/components/ui/editable-inline";
import { EditableTextarea } from "@/components/ui/editable-textarea";
import { EditableDate } from "@/components/ui/editable-date";
import { EditableText } from "@/components/ui/EditableText";
import { formatFullDate } from "@/lib/utils";
import { classifyAction } from "@/lib/entity-utils";
import { Check, Circle, ExternalLink } from "lucide-react";
import type { ActionDetail, LinearPushResult } from "@/types";
import s from "./ActionDetailPage.module.css";

// =============================================================================
// Priority accent colors
// =============================================================================

const PRIORITY_CLASS: Record<string, string> = {
  1: s.priorityP1,
  2: s.priorityP2,
  3: s.priorityP2,
  4: s.priorityP3,
};

function priorityAccent(priority: number | string): string {
  const v = typeof priority === "string" ? parseInt(priority, 10) : priority;
  if (v <= 1) return "var(--color-spice-terracotta)";
  if (v <= 2) return "var(--color-spice-turmeric)";
  return "var(--color-garden-larkspur)";
}

// =============================================================================
// Main component
// =============================================================================

export default function ActionDetailPage() {
  const { actionId } = useParams({ strict: false }) as {
    actionId?: string;
  };
  const navigate = useNavigate();
  const [detail, setDetail] = useState<ActionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const saveTimerRef = useRef<ReturnType<typeof setTimeout>>();

  // Linear push state
  const [linearEnabled, setLinearEnabled] = useState(false);
  const [teams, setTeams] = useState<Array<{ id: string; name: string }>>([]);
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [pushing, setPushing] = useState(false);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: detail?.title ? (detail.title.length > 30 ? detail.title.slice(0, 30) + "…" : detail.title) : "Action",
      atmosphereColor: "terracotta" as const,
      activePage: "actions" as const,
      backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/actions", search: { search: undefined } }) },
      folioStatusText: saveStatus === "saving" ? "Saving…" : saveStatus === "saved" ? "✓ Saved" : undefined,
    }),
    [detail?.title, navigate, saveStatus],
  );
  useRegisterMagazineShell(shellConfig);

  const load = useCallback(async () => {
    if (!actionId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ActionDetail>("get_action_detail", {
        actionId,
      });
      setDetail(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [actionId]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    invoke<{ enabled: boolean; apiKeySet: boolean }>("get_linear_status")
      .then((s) => {
        const enabled = s.enabled && s.apiKeySet;
        setLinearEnabled(enabled);
        if (enabled) {
          invoke<Array<{ id: string; name: string }>>("get_linear_teams")
            .then((t) => { setTeams(t); })
            .catch(() => {});
        }
      })
      .catch(() => {});
  }, []);

  async function toggleStatus() {
    if (!detail) return;
    setToggling(true);
    try {
      if (detail.status === "completed") {
        await invoke("reopen_action", { id: detail.id });
      } else {
        await invoke("complete_action", { id: detail.id });
      }
      await load();
    } finally {
      setToggling(false);
    }
  }

  async function saveField(updates: Record<string, unknown>) {
    if (!detail) return;
    clearTimeout(saveTimerRef.current);
    setSaveStatus("saving");
    try {
      await invoke("update_action", {
        request: { id: detail.id, ...updates },
      });
      await load();
      setSaveStatus("saved");
      saveTimerRef.current = setTimeout(() => setSaveStatus("idle"), 1500);
    } catch (e) {
      console.error("Failed to save action:", e);
      toast.error("Failed to save");
      setSaveStatus("idle");
    }
  }

  async function handlePushToLinear() {
    if (!detail || !selectedTeamId || pushing) return;
    setPushing(true);
    try {
      const result = await invoke<LinearPushResult>("push_action_to_linear", {
        actionId: detail.id,
        teamId: selectedTeamId,
        title: detail.title,
      });
      toast.success(`Created ${result.identifier}`);
      await load();
    } catch (e) {
      toast.error(`Push failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setPushing(false);
    }
  }

  // ── Loading state ──

  if (loading) {
    return (
      <div className={`editorial-loading ${s.loadingSkeleton}`}>
        <div className={`${s.skeletonBar} ${s.skeletonTitle}`} />
        <div className={`${s.skeletonBar} ${s.skeletonHeadline}`} />
        <div className={`${s.skeletonBar} ${s.skeletonSubhead}`} />
        <div className={s.skeletonRule} />
        <div className={s.skeletonBody}>
          <div className={`${s.skeletonBlock} ${s.skeletonBlockSmall}`} />
          <div className={`${s.skeletonBlock} ${s.skeletonBlockLarge}`} />
        </div>
      </div>
    );
  }

  // ── Error state ──

  if (error || !detail) {
    return (
      <div className={s.errorState}>
        <p className={s.errorTitle}>
          Something went wrong
        </p>
        <p className={s.errorMessage}>
          {error ?? "Action not found"}
        </p>
        <button onClick={load} className={s.retryButton}>
          Try again
        </button>
      </div>
    );
  }

  const isCompleted = detail.status === "completed";
  const hasSource = detail.sourceId && detail.sourceMeetingTitle;
  const isAutoGenerated = detail.sourceType && detail.sourceType !== "manual";
  const accent = priorityAccent(detail.priority);
  const priorityCls = PRIORITY_CLASS[detail.priority];

  // Due date urgency
  const dueUrgency = classifyAction(detail, new Date());
  const dueColor =
    dueUrgency === "overdue"
      ? "var(--color-spice-terracotta)"
      : dueUrgency === "this-week"
        ? "var(--color-spice-turmeric)"
        : undefined;

  return (
    <div className={s.container}>

        {/* ── Title band ── */}
        <div className={s.titleBand}>
          {/* Status toggle circle */}
          <button
            onClick={toggleStatus}
            disabled={toggling}
            className={s.toggleButton}
            style={{
              cursor: toggling ? "wait" : "pointer",
              color: isCompleted ? "var(--color-text-tertiary)" : accent,
              opacity: toggling ? 0.5 : 1,
            }}
            title={isCompleted ? "Reopen action" : "Complete action"}
          >
            {isCompleted ? (
              <Check size={22} strokeWidth={2.5} />
            ) : (
              <Circle size={22} strokeWidth={1.5} />
            )}
          </button>

          {/* Editable title */}
          <div className={s.titleWrapper}>
            <EditableText
              value={detail.title}
              onChange={(title) => saveField({ title })}
              as="span"
              multiline={false}
              className={`${s.titleEditable} ${isCompleted ? s.titleCompleted : ""}`}
            />
          </div>
        </div>

        {/* Priority + Status strip */}
        <div className={s.statusStrip}>
          {/* Priority pill */}
          {priorityCls && (
            <span className={`${s.priorityPill} ${priorityCls}`}>
              {detail.priority <= 1 ? "Urgent" : detail.priority <= 2 ? "High" : detail.priority === 4 ? "Low" : "Medium"}
            </span>
          )}

          {/* Status text */}
          <span className={s.statusText}>
            {isCompleted ? "Completed" : "Open"}
          </span>

          {/* Waiting on */}
          {detail.waitingOn && (
            <span className={s.monoBadge}>
              Waiting: {detail.waitingOn}
            </span>
          )}

          {/* Source badge (meeting link or label) */}
          {hasSource && (
            <Link
              to="/meeting/$meetingId"
              params={{ meetingId: detail.sourceId! }}
              className={s.monoBadgeLink}
            >
              From meeting
            </Link>
          )}
        </div>

        {/* PriorityPicker (click row to change) */}
        <div className={s.priorityPickerRow}>
          <PriorityPicker
            value={detail.priority}
            onChange={(p) => saveField({ priority: p })}
          />
        </div>

        {/* Separator */}
        <div className={s.separator} />

        {/* ── 3. Context section ── */}
        <div className={s.section}>
          <div className={s.sectionLabel}>Context</div>
          <EditableTextarea
            value={detail.context ?? ""}
            onSave={(context) =>
              context
                ? saveField({ context })
                : saveField({ clearContext: true })
            }
            placeholder="Add context..."
          />
          {isAutoGenerated && detail.context && (
            <p className={s.autoNote}>
              Auto-generated — may be updated by next briefing
            </p>
          )}
        </div>

        {/* ── 4. Reference section ── */}
        <div className={s.section}>
          <div className={s.sectionLabelWide}>Reference</div>
          <div className={s.refSection}>

            {/* Account */}
            <div className={s.refRow}>
              <span className={s.refKey}>Account</span>
              <div className={s.refValue}>
                {detail.accountId && detail.accountName ? (
                  <span className={s.accountChip}>
                    <span className={s.accountDot} />
                    <Link
                      to="/accounts/$accountId"
                      params={{ accountId: detail.accountId }}
                      className={s.accountLink}
                    >
                      {detail.accountName}
                    </Link>
                    <button
                      onClick={() => saveField({ clearAccount: true })}
                      className={s.removeButton}
                      title="Remove account"
                    >
                      ×
                    </button>
                  </span>
                ) : (
                  <EntityPicker
                    value={null}
                    onChange={(id) => {
                      if (id) saveField({ accountId: id });
                    }}
                    entityType="account"
                    placeholder="Link account"
                  />
                )}
              </div>
            </div>

            {/* Due */}
            <div className={s.refRow}>
              <span className={s.refKey}>Due</span>
              <div className={s.refValue} style={dueColor ? { color: dueColor } : undefined}>
                <EditableDate
                  value={detail.dueDate ?? ""}
                  onSave={(v) =>
                    v
                      ? saveField({ dueDate: v })
                      : saveField({ clearDueDate: true })
                  }
                  urgencyColor={dueColor}
                />
              </div>
            </div>

            {/* Created (read-only) */}
            <div className={s.refRow}>
              <span className={s.refKey}>Created</span>
              <span className={s.refValue}>{formatFullDate(detail.createdAt)}</span>
            </div>

            {/* Completed (read-only, only when completed) */}
            {detail.completedAt && (
              <div className={s.refRow}>
                <span className={s.refKey}>Completed</span>
                <span className={s.refValue}>{formatFullDate(detail.completedAt)}</span>
              </div>
            )}

            {/* Source — meeting link (if not already shown in strip) or editable label */}
            {hasSource ? (
              <div className={s.refRow}>
                <span className={s.refKey}>Source</span>
                <div className={s.refValue}>
                  <Link
                    to="/meeting/$meetingId"
                    params={{ meetingId: detail.sourceId! }}
                    className={s.accountLink}
                  >
                    {detail.sourceMeetingTitle}
                  </Link>
                </div>
              </div>
            ) : (
              <div className={s.refRow}>
                <span className={s.refKey}>Source</span>
                <div className={s.refValue}>
                  <EditableInline
                    value={detail.sourceLabel ?? ""}
                    onSave={(v) =>
                      v
                        ? saveField({ sourceLabel: v })
                        : saveField({ clearSourceLabel: true })
                    }
                    placeholder="Add source"
                  />
                </div>
              </div>
            )}
          </div>
        </div>

        {/* ── 5. Linear section ── */}
        {linearEnabled && (
          <div className={s.section}>
            <div className={s.sectionLabel}>Linear</div>
            {detail.linearIdentifier ? (
              <div className={s.refSection}>
                <div className={s.refRow}>
                  <span className={s.refKey}>Issue</span>
                  <div className={s.refValue}>
                    <a
                      href={detail.linearUrl ?? "#"}
                      target="_blank"
                      rel="noopener noreferrer"
                      className={s.accountLink}
                    >
                      {detail.linearIdentifier}
                      <ExternalLink size={12} style={{ marginLeft: 4, verticalAlign: "middle" }} />
                    </a>
                  </div>
                </div>
              </div>
            ) : (detail.status === "backlog" || detail.status === "unstarted") ? (
              <div className={s.linearPushRow}>
                <select
                  value={selectedTeamId ?? ""}
                  onChange={(e) => setSelectedTeamId(e.target.value || null)}
                  className={s.linearSelect}
                >
                  <option value="">Select a project</option>
                  {teams.map((t) => (
                    <option key={t.id} value={t.id}>{t.name}</option>
                  ))}
                </select>
                <button
                  onClick={handlePushToLinear}
                  disabled={pushing || !selectedTeamId}
                  className={selectedTeamId ? s.linearPushReady : s.linearPushDisabled}
                >
                  {pushing ? "Creating..." : "Create Linear Issue"}
                </button>
              </div>
            ) : (
              <p className={s.autoNote}>
                Push to Linear is available for suggested and active actions.
              </p>
            )}
          </div>
        )}

        {/* ── 6. Action bar ── */}
        <div className={s.actionBar}>
          {saveStatus !== "idle" && (
            <span
              className={s.saveStatus}
              style={{ opacity: saveStatus === "saved" ? 1 : 0.6 }}
            >
              {saveStatus === "saving" ? "Saving…" : "Saved"}
            </span>
          )}
          <button
            onClick={toggleStatus}
            disabled={toggling}
            className={s.actionButton}
            style={{
              cursor: toggling ? "wait" : "pointer",
              opacity: toggling ? 0.5 : 1,
            }}
          >
            {isCompleted ? "Reopen" : "Mark Complete"}
          </button>
        </div>
    </div>
  );
}


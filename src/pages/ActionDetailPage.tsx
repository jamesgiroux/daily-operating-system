import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import type { ReactNode } from "react";
import { useParams, Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import {
  useChapterLayout,
  type RenderableCompositionBlock,
  type RenderableCompositionSection,
} from "@/hooks/useChapterLayout";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import { PriorityPicker } from "@/components/ui/priority-picker";
import { EntityPicker } from "@/components/ui/entity-picker";
import { EditableInline } from "@/components/ui/editable-inline";
import { EditableTextarea } from "@/components/ui/editable-textarea";
import { EditableDate } from "@/components/ui/editable-date";
import { EditableText } from "@/components/ui/EditableText";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { ReactBlockRenderer } from "@/components/composition/ReactBlockRenderer";
import { formatFullDate } from "@/lib/utils";
import { classifyAction } from "@/lib/entity-utils";
import { Check, Circle, ExternalLink, FileText, Flag, Link as LinkIcon, ListChecks, Send } from "lucide-react";
import type { ActionDetail, LinearPushResult } from "@/types";
import type { ProjectedBlock } from "@/services/composition/contracts";
import shared from "@/styles/entity-detail.module.css";
import accountStyles from "./AccountDetailPage.module.css";
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

const SECTION_ICONS: Record<string, ReactNode> = {
  headline: <ListChecks size={18} strokeWidth={1.5} />,
  status: <Check size={18} strokeWidth={1.5} />,
  priority: <Flag size={18} strokeWidth={1.5} />,
  context: <FileText size={18} strokeWidth={1.5} />,
  reference: <LinkIcon size={18} strokeWidth={1.5} />,
  linear: <Send size={18} strokeWidth={1.5} />,
  "action-bar": <ListChecks size={18} strokeWidth={1.5} />,
};

function actionTitleFromBlocks(
  blocks: ProjectedBlock[],
  detail: ActionDetail | null,
  actionId: string | undefined,
): string {
  const overview = blocks.find((block) => block.selected_known_type_id === "account_overview");
  const action = overview?.payload.action;
  if (action && typeof action === "object" && !Array.isArray(action)) {
    const title = (action as Record<string, unknown>).title;
    if (typeof title === "string" && title.trim()) return title;
  }
  const account = overview?.payload.account;
  if (account && typeof account === "object" && !Array.isArray(account)) {
    const displayName = (account as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  return detail?.title ?? actionId ?? "Action";
}

// =============================================================================
// Main component
// =============================================================================

export default function ActionDetailPage() {
  const { actionId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [detail, setDetail] = useState<ActionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const saveTimerRef = useRef<ReturnType<typeof setTimeout>>();
  const composition = useProjectedComposition(
    actionId ? { entityType: "action", entityId: actionId } : undefined,
  );
  const projection = composition.data?.projection ?? null;
  const layout = useChapterLayout({ projection, entityType: "action" });
  const visibleSections = layout.view.sections;
  const actionTitle = actionTitleFromBlocks(projection?.blocks ?? [], detail, actionId);

  // Linear push state
  const [linearEnabled, setLinearEnabled] = useState(false);
  const [teams, setTeams] = useState<Array<{ id: string; name: string }>>([]);
  const [selectedTeamId, setSelectedTeamId] = useState<string | null>(null);
  const [pushing, setPushing] = useState(false);

  useRevealObserver(!composition.loading && !!projection);

  const chapters = useMemo(
    () =>
      visibleSections.map(({ section, label }) => ({
        id: section.section_id,
        label,
        icon: SECTION_ICONS[section.section_id] ?? <FileText size={18} strokeWidth={1.5} />,
      })),
    [visibleSections],
  );

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: actionTitle.length > 30 ? actionTitle.slice(0, 30) + "…" : actionTitle,
      atmosphereColor: "terracotta" as const,
      activePage: "actions" as const,
      breadcrumbs: [
        { label: "Actions", onClick: () => navigate({ to: "/actions", search: { search: undefined } }) },
        { label: actionTitle },
      ],
      chapters,
      folioStatusText: composition.loading
        ? "Composing..."
        : saveStatus === "saving"
          ? "Saving..."
          : saveStatus === "saved"
            ? "Saved"
            : composition.data?.served_from_cache
              ? "Projected from cache"
              : undefined,
      folioActions: (
        <div className={shared.folioActions}>
          <FolioRefreshButton onClick={composition.refetch} loading={composition.loading} />
        </div>
      ),
    }),
    [
      actionTitle,
      chapters,
      composition.data?.served_from_cache,
      composition.loading,
      composition.refetch,
      navigate,
      saveStatus,
    ],
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

  const reloadAfterActionMutation = useCallback(async () => {
    await Promise.all([
      load(),
      composition.refetch({ forceRefresh: true }),
    ]);
  }, [composition.refetch, load]);

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
      await reloadAfterActionMutation();
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
      await reloadAfterActionMutation();
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
      await reloadAfterActionMutation();
    } catch (e) {
      toast.error(`Push failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setPushing(false);
    }
  }

  if (composition.loading && !projection) return <EditorialLoading />;

  if (composition.error) {
    return <EditorialError message={composition.error} onRetry={composition.refetch} />;
  }

  if (!projection || projection.sections.length === 0 || projection.blocks.length === 0) {
    return (
      <EditorialEmpty
        title="No action composition"
        message="DailyOS has not produced an action detail surface yet."
      />
    );
  }

  const renderBlock = (item: RenderableCompositionBlock) => (
    <ReactBlockRenderer
      key={item.block.block_id}
      block={item.block}
      entityId={actionId}
      entityType="action"
      renderedProvenance={composition.renderedProvenance}
    />
  );

  const renderSectionTitle = (section: RenderableCompositionSection) => (
    <h2 className={accountStyles.compositionSectionTitle}>{section.label}</h2>
  );

  let controlsContent: ReactNode;

  if (loading) {
    controlsContent = (
      <section className={s.controlsPanel} aria-label="Action controls">
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
      </section>
    );
  } else if (error || !detail) {
    controlsContent = (
      <section className={s.controlsPanel} aria-label="Action controls">
        <div className={s.errorState}>
          <p className={s.errorTitle}>Action controls unavailable</p>
          <p className={s.errorMessage}>{error ?? "Action not found"}</p>
          <button onClick={load} className={s.retryButton}>
            Try again
          </button>
        </div>
      </section>
    );
  } else {

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

  controlsContent = (
    <section className={s.controlsPanel} aria-label="Action controls">
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
    </section>
  );
  }

  return (
    <main
      className={accountStyles.compositionSurface}
      data-composition-id={projection.composition_id}
      data-composition-version={projection.composition_version ?? 0}
      data-fallback-policy-version={projection.fallback_policy_version}
    >
      {visibleSections.map((renderableSection) => {
        const section = renderableSection.section;
        const isHeadline = section.section_id === "headline";
        return (
          <section
            key={section.section_id}
            id={section.section_id}
            className={isHeadline ? accountStyles.compositionMasthead : accountStyles.compositionSection}
            data-section-id={section.section_id}
            data-section-layout={section.layout}
            data-section-salience={section.salience.band}
          >
            {isHeadline ? (
              <div className={accountStyles.compositionMastheadGrid}>
                {renderableSection.blocks.map(renderBlock)}
              </div>
            ) : (
              <>
                <div className={accountStyles.compositionSectionLabel}>{renderableSection.label}</div>
                <div className={accountStyles.compositionSectionBody}>
                  <header className={accountStyles.compositionSectionHeader}>
                    <div>{renderSectionTitle(renderableSection)}</div>
                    {section.salience.reason && (
                      <p className={accountStyles.compositionSectionMeta}>{section.salience.reason}</p>
                    )}
                  </header>
                  <div className={accountStyles.compositionBlockStack}>
                    {renderableSection.blocks.map(renderBlock)}
                  </div>
                </div>
              </>
            )}
          </section>
        );
      })}
      {controlsContent}
      <FinisMarker />
    </main>
  );
}

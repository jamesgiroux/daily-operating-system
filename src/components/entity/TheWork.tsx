import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Zap } from "lucide-react";
import type {
  AccountObjective,
  SuccessPlanTemplate,
  SuggestedObjective,
} from "@/types";
import type { WorkSource } from "@/lib/entity-types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { ActionRow } from "@/components/shared/ActionRow";
import { MeetingRow } from "@/components/shared/MeetingRow";
import { TemplateSuggestionBanner } from "@/components/entity/TemplateSuggestionBanner";
import { TemplatePreview } from "@/components/entity/TemplatePreview";
import { formatShortDate, formatMeetingType } from "@/lib/utils";
import {
  classifyAction,
  formatMeetingRowDate,
  meetingTypeBadgeStyle,
} from "@/lib/entity-utils";
import s from "./TheWork.module.css";

interface TheWorkProps {
  data: WorkSource;
  sectionId?: string;
  chapterTitle?: string;
  addingAction?: boolean;
  setAddingAction?: (v: boolean) => void;
  newActionTitle?: string;
  setNewActionTitle?: (v: string) => void;
  creatingAction?: boolean;
  onCreateAction?: () => void;
  onRefresh?: () => Promise<void> | void;
}

function filterUnlinkedActions(data: WorkSource): typeof data.openActions {
  const linkedIds = new Set(
    (data.objectives ?? []).flatMap((objective) => objective.linkedActions.map((action) => action.id)),
  );
  return data.openActions.filter((action) => !linkedIds.has(action.id));
}

function isRenewalNear(renewalDate?: string): boolean {
  if (!renewalDate) return false;
  const renewal = new Date(renewalDate);
  const diff = Math.round((renewal.getTime() - Date.now()) / (1000 * 60 * 60 * 24));
  return diff >= 0 && diff <= 120;
}

function labelObjectiveStatus(status: string): string {
  switch (status) {
    case "completed":
      return "Completed";
    case "abandoned":
      return "Abandoned";
    case "draft":
      return "Draft";
    default:
      return "Active";
  }
}

function labelMilestoneStatus(status: string): string {
  switch (status) {
    case "completed":
      return "Completed";
    case "skipped":
      return "Skipped";
    default:
      return "Pending";
  }
}

export function TheWork({
  data,
  sectionId = "the-work",
  chapterTitle = "The Work",
  addingAction,
  setAddingAction,
  newActionTitle,
  setNewActionTitle,
  creatingAction,
  onCreateAction,
  onRefresh,
}: TheWorkProps) {
  const [templates, setTemplates] = useState<SuccessPlanTemplate[]>([]);
  const [templatesOpen, setTemplatesOpen] = useState(false);
  const [suggestionsOpen, setSuggestionsOpen] = useState(false);
  const [dismissedTemplateIds, setDismissedTemplateIds] = useState<string[]>([]);
  const [suggestions, setSuggestions] = useState<SuggestedObjective[]>([]);
  const [loadingSuggestions, setLoadingSuggestions] = useState(false);
  const [addingObjective, setAddingObjective] = useState(false);
  const [newObjectiveTitle, setNewObjectiveTitle] = useState("");
  const [creatingObjective, setCreatingObjective] = useState(false);
  const [addingMilestoneFor, setAddingMilestoneFor] = useState<string | null>(null);
  const [newMilestoneTitle, setNewMilestoneTitle] = useState("");
  const [linkingActionId, setLinkingActionId] = useState<string | null>(null);

  useEffect(() => {
    if (!data.accountId) return;
    invoke<SuccessPlanTemplate[]>("list_success_plan_templates")
      .then(setTemplates)
      .catch((err) => {
        console.error("list_success_plan_templates failed:", err);
        toast.error("Failed to load templates");
      });
  }, [data.accountId]);

  const hasTemplateObjectives = (data.objectives ?? []).some((o) => o.source === "template");

  const suggestedTemplates = useMemo(() => {
    return templates.filter((template) => {
      if (dismissedTemplateIds.includes(template.id)) return false;
      if (template.id === "renewal-preparation") return isRenewalNear(data.renewalDate) && !hasTemplateObjectives;
      if (template.id === "at-risk-recovery") return data.health === "red";
      return template.lifecycleTrigger === data.lifecycle;
    });
  }, [data.health, data.lifecycle, data.renewalDate, dismissedTemplateIds, hasTemplateObjectives, templates]);

  const refresh = async () => {
    await onRefresh?.();
  };

  const createObjective = async (source: "user" | "template" | "ai_suggested", title: string, description?: string) => {
    if (!data.accountId || !title.trim()) return;
    setCreatingObjective(true);
    try {
      await invoke("create_objective", {
        accountId: data.accountId,
        title: title.trim(),
        description: description?.trim() || null,
        source,
      });
      setNewObjectiveTitle("");
      await refresh();
    } catch (err) {
      console.error("create_objective failed:", err);
      toast.error("Failed to create objective");
    } finally {
      setCreatingObjective(false);
    }
  };

  const updateObjective = async (objectiveId: string, fields: Record<string, unknown>) => {
    try {
      await invoke("update_objective", { id: objectiveId, fields });
      await refresh();
    } catch (err) {
      console.error("update_objective failed:", err);
      toast.error("Failed to save objective");
    }
  };

  const completeObjective = async (objectiveId: string) => {
    try {
      await invoke("complete_objective", { id: objectiveId });
      await refresh();
    } catch (err) {
      console.error("complete_objective failed:", err);
      toast.error("Failed to complete objective");
    }
  };

  const abandonObjective = async (objectiveId: string) => {
    try {
      await invoke("abandon_objective", { id: objectiveId });
      await refresh();
    } catch (err) {
      console.error("abandon_objective failed:", err);
      toast.error("Failed to update objective");
    }
  };

  const deleteObjective = async (objectiveId: string) => {
    try {
      await invoke("delete_objective", { id: objectiveId });
      await refresh();
    } catch (err) {
      console.error("delete_objective failed:", err);
      toast.error("Failed to delete objective");
    }
  };

  const reorderObjectives = async (orderedIds: string[]) => {
    if (!data.accountId) return;
    try {
      await invoke("reorder_objectives", { accountId: data.accountId, orderedIds });
      await refresh();
    } catch (err) {
      console.error("reorder_objectives failed:", err);
      toast.error("Failed to reorder objectives");
    }
  };

  const createMilestone = async (objectiveId: string) => {
    if (!newMilestoneTitle.trim()) return;
    try {
      await invoke("create_milestone", {
        objectiveId,
        title: newMilestoneTitle.trim(),
      });
      setNewMilestoneTitle("");
      setAddingMilestoneFor(null);
      await refresh();
    } catch (err) {
      console.error("create_milestone failed:", err);
      toast.error("Failed to add milestone");
    }
  };

  const updateMilestone = async (milestoneId: string, fields: Record<string, unknown>) => {
    try {
      await invoke("update_milestone", { id: milestoneId, fields });
      await refresh();
    } catch (err) {
      console.error("update_milestone failed:", err);
      toast.error("Failed to save milestone");
    }
  };

  const completeMilestone = async (milestoneId: string) => {
    try {
      await invoke("complete_milestone", { id: milestoneId });
      await refresh();
    } catch (err) {
      console.error("complete_milestone failed:", err);
      toast.error("Failed to complete milestone");
    }
  };

  const skipMilestone = async (milestoneId: string) => {
    try {
      await invoke("skip_milestone", { id: milestoneId });
      await refresh();
    } catch (err) {
      console.error("skip_milestone failed:", err);
      toast.error("Failed to skip milestone");
    }
  };

  const deleteMilestone = async (milestoneId: string) => {
    try {
      await invoke("delete_milestone", { id: milestoneId });
      await refresh();
    } catch (err) {
      console.error("delete_milestone failed:", err);
      toast.error("Failed to delete milestone");
    }
  };

  const reorderMilestones = async (objective: AccountObjective, milestoneId: string, direction: -1 | 1) => {
    const index = objective.milestones.findIndex((milestone) => milestone.id === milestoneId);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= objective.milestones.length) return;
    const reordered = [...objective.milestones];
    const [moved] = reordered.splice(index, 1);
    reordered.splice(nextIndex, 0, moved);
    try {
      await invoke("reorder_milestones", {
        objectiveId: objective.id,
        orderedIds: reordered.map((milestone) => milestone.id),
      });
      await refresh();
    } catch (err) {
      console.error("reorder_milestones failed:", err);
      toast.error("Failed to reorder milestones");
    }
  };

  const linkAction = async (actionId: string, objectiveId: string) => {
    try {
      await invoke("link_action_to_objective", { actionId, objectiveId });
      setLinkingActionId(null);
      await refresh();
    } catch (err) {
      console.error("link_action_to_objective failed:", err);
      toast.error("Failed to link action");
    }
  };

  const unlinkAction = async (actionId: string, objectiveId: string) => {
    try {
      await invoke("unlink_action_from_objective", { actionId, objectiveId });
      await refresh();
    } catch (err) {
      console.error("unlink_action_from_objective failed:", err);
      toast.error("Failed to unlink action");
    }
  };

  const loadSuggestions = async () => {
    if (!data.accountId) return;
    setLoadingSuggestions(true);
    try {
      const next = await invoke<SuggestedObjective[]>("get_objective_suggestions", {
        accountId: data.accountId,
      });
      setSuggestions(next);
      setSuggestionsOpen(true);
    } catch (err) {
      console.error("get_objective_suggestions failed:", err);
      toast.error("Failed to load suggestions");
    } finally {
      setLoadingSuggestions(false);
    }
  };

  const acceptSuggestion = async (suggestion: SuggestedObjective) => {
    if (!data.accountId) return;
    try {
      await invoke("create_objective_from_suggestion", {
        accountId: data.accountId,
        suggestionJson: JSON.stringify(suggestion),
      });
      await refresh();
    } catch (err) {
      console.error("create_objective_from_suggestion failed:", err);
      toast.error("Failed to add suggestion");
    }
  };

  const applyTemplate = async (templateId: string) => {
    if (!data.accountId) return;
    try {
      await invoke("apply_success_plan_template", {
        accountId: data.accountId,
        templateId,
      });
      await refresh();
      setTemplatesOpen(false);
    } catch (err) {
      console.error("apply_success_plan_template failed:", err);
      toast.error("Failed to apply template");
    }
  };

  const unlinkedActions = filterUnlinkedActions(data);
  const now = new Date();
  const overdue = unlinkedActions.filter((action) => classifyAction(action, now) === "overdue");
  const thisWeek = unlinkedActions.filter((action) => classifyAction(action, now) === "this-week");
  const upcoming = unlinkedActions.filter((action) => classifyAction(action, now) === "upcoming");
  const noDue = unlinkedActions.filter((action) => classifyAction(action, now) === "no-date");
  const upcomingMeetings = data.upcomingMeetings ?? [];
  const hasContent = (data.objectives?.length ?? 0) > 0 || data.openActions.length > 0 || upcomingMeetings.length > 0 || !!data.accountId;

  if (!hasContent) return null;

  return (
    <section id={sectionId || undefined} className={`${s.section}${sectionId ? ` ${s.scrollTarget}` : ""}`}>
      <div className={s.headerRow}>
        <ChapterHeading title={chapterTitle} />
        {data.accountId && (
          <div className={s.headerActions}>
            <button className={s.headerButton} onClick={loadSuggestions} disabled={loadingSuggestions}>
              {loadingSuggestions ? "Loading…" : "Suggestions"}
            </button>
            <button className={s.headerButton} onClick={() => setTemplatesOpen((open) => !open)}>
              From Template
            </button>
          </div>
        )}
      </div>

      {suggestedTemplates.length > 0 && (
        <TemplateSuggestionBanner
          template={suggestedTemplates[0]}
          onView={() => setTemplatesOpen(true)}
          onDismiss={() => setDismissedTemplateIds((prev) => [...prev, suggestedTemplates[0].id])}
        />
      )}

      {templatesOpen && templates.length > 0 && (
        <TemplatePreview
          templates={templates}
          onApply={applyTemplate}
          onClose={() => setTemplatesOpen(false)}
        />
      )}

      {suggestionsOpen && (
        <div className={s.panel}>
          <div className={s.panelTitle}>Suggested Objectives</div>
          {suggestions.length === 0 ? (
            <p className={s.emptyMessage}>No suggestions available yet.</p>
          ) : (
            suggestions.map((suggestion) => (
              <div key={`${suggestion.title}-${suggestion.confidence}`} className={s.suggestionCard}>
                <div className={s.suggestionTop}>
                  <div>
                    <div className={s.suggestionTitle}>{suggestion.title}</div>
                    {suggestion.description && (
                      <div className={s.suggestionDescription}>{suggestion.description}</div>
                    )}
                  </div>
                  <button className={s.templateApply} onClick={() => acceptSuggestion(suggestion)}>
                    Add
                  </button>
                </div>
                <div className={s.suggestionMeta}>
                  {suggestion.confidence}
                  {suggestion.sourceEvidence ? ` · ${suggestion.sourceEvidence}` : ""}
                </div>
                {suggestion.milestones.length > 0 && (
                  <ul className={s.suggestionMilestones}>
                    {suggestion.milestones.map((milestone) => (
                      <li key={`${suggestion.title}-${milestone.title}`}>{milestone.title}</li>
                    ))}
                  </ul>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {data.accountId && (
        <div className={s.actionComposer}>
          {addingObjective ? (
            <div className={s.newObjectiveRow}>
              <input
                value={newObjectiveTitle}
                onChange={(event) => setNewObjectiveTitle(event.target.value)}
                placeholder="Add an objective..."
                className={s.objectiveInput}
                autoFocus
                onKeyDown={(event) => {
                  if (event.key === "Enter" && newObjectiveTitle.trim()) {
                    void createObjective("user", newObjectiveTitle);
                  }
                  if (event.key === "Escape") {
                    setAddingObjective(false);
                    setNewObjectiveTitle("");
                  }
                }}
              />
              <button
                className={s.templateApply}
                disabled={creatingObjective || !newObjectiveTitle.trim()}
                onClick={() => createObjective("user", newObjectiveTitle)}
              >
                Add
              </button>
              <button className={s.smallActionMuted} onClick={() => { setAddingObjective(false); setNewObjectiveTitle(""); }}>
                Cancel
              </button>
            </div>
          ) : (
            <button className={s.inlineAdder} onClick={() => setAddingObjective(true)}>
              + Objective
            </button>
          )}
        </div>
      )}

      {(data.objectives ?? []).map((objective, index) => {
        const statusClass = objective.status === "draft" ? s.objectiveDraft
          : objective.status === "completed" ? s.objectiveCompleted
          : objective.status === "abandoned" ? s.objectiveAbandoned
          : "";
        const progressRatio = objective.totalMilestoneCount > 0
          ? objective.completedMilestoneCount / objective.totalMilestoneCount
          : 0;
        const pendingMilestones = objective.totalMilestoneCount - objective.completedMilestoneCount;
        const targetDate = objective.targetDate ? new Date(objective.targetDate) : null;
        const daysUntilTarget = targetDate ? Math.round((targetDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)) : null;
        const progressClass = pendingMilestones === 0 ? s.progressOnTrack
          : !targetDate || (daysUntilTarget !== null && daysUntilTarget > 14) ? s.progressOnTrack
          : daysUntilTarget !== null && daysUntilTarget < 0 ? s.progressOverdue
          : s.progressWarning;
        return (
        <div key={objective.id} className={`${s.objectiveCard}${statusClass ? ` ${statusClass}` : ""}`}>
          <div className={s.objectiveTop}>
            <div className={s.objectiveMain}>
              <EditableText
                as="h3"
                value={objective.title}
                onChange={(value) => updateObjective(objective.id, { title: value })}
                multiline={false}
                className={s.objectiveTitle}
              />
              {objective.description && (
                <EditableText
                  as="p"
                  value={objective.description}
                  onChange={(value) => updateObjective(objective.id, { description: value })}
                  className={s.objectiveDescription}
                />
              )}
              <div className={s.objectiveMeta}>
                {labelObjectiveStatus(objective.status)} · {objective.completedMilestoneCount} of {objective.totalMilestoneCount} milestones · {objective.linkedActionCount} linked actions
              </div>
              {objective.targetDate && (
                <div className={s.objectiveTargetDate}>
                  Target: {formatShortDate(objective.targetDate)}
                </div>
              )}
              {objective.totalMilestoneCount > 0 && (
                <div className={`${s.progressBar} ${progressClass}`}>
                  <div className={s.progressFill} style={{ '--progress-width': `${progressRatio * 100}%` } as React.CSSProperties} />
                </div>
              )}
            </div>
            <div className={s.objectiveActions}>
              {(data.objectives?.length ?? 0) > 1 && (
                <>
                  <button
                    className={s.smallAction}
                    onClick={() => reorderObjectives([
                      ...(data.objectives ?? []).slice(0, index - 1).map((item) => item.id),
                      objective.id,
                      (data.objectives ?? [])[index - 1].id,
                      ...(data.objectives ?? []).slice(index + 1).map((item) => item.id),
                    ])}
                    disabled={index === 0}
                    title="Move up"
                  >
                    ↑
                  </button>
                  <button
                    className={s.smallAction}
                    onClick={() => reorderObjectives([
                      ...(data.objectives ?? []).slice(0, index).map((item) => item.id),
                      (data.objectives ?? [])[index + 1].id,
                      objective.id,
                      ...(data.objectives ?? []).slice(index + 2).map((item) => item.id),
                    ])}
                    disabled={index === (data.objectives?.length ?? 1) - 1}
                    title="Move down"
                  >
                    ↓
                  </button>
                </>
              )}
              {objective.status !== "completed" && (
                <button className={s.smallAction} onClick={() => completeObjective(objective.id)}>
                  Complete
                </button>
              )}
              {objective.status !== "abandoned" && (
                <button className={s.smallActionMuted} onClick={() => abandonObjective(objective.id)}>
                  Abandon
                </button>
              )}
              <button className={s.smallActionMuted} onClick={() => deleteObjective(objective.id)}>
                Delete
              </button>
            </div>
          </div>

          <div className={s.milestoneList}>
            {objective.milestones.map((milestone) => (
              <div key={milestone.id} className={s.milestoneRow}>
                <button
                  className={`${s.milestoneToggle} ${milestone.status === "completed" ? s.milestoneToggleDone : ""}`}
                  onClick={() => completeMilestone(milestone.id)}
                  title="Complete milestone"
                />
                <div className={s.milestoneContent}>
                  <EditableText
                    value={milestone.title}
                    onChange={(value) => updateMilestone(milestone.id, { title: value })}
                    multiline={false}
                    className={s.milestoneTitle}
                  />
                  <div className={s.milestoneMeta}>
                    {labelMilestoneStatus(milestone.status)}
                    {milestone.targetDate ? ` · ${formatShortDate(milestone.targetDate)}` : ""}
                    {milestone.autoDetectSignal ? <>{" · "}<Zap size={11} className={s.autoDetectIcon} /></> : ""}
                    {milestone.completedBy === "lifecycle_transition" && milestone.completionTrigger
                      ? " · completed automatically"
                      : ""}
                  </div>
                </div>
                <div className={s.milestoneActions}>
                  {objective.milestones.length > 1 && (
                    <>
                      <button className={s.smallAction} onClick={() => reorderMilestones(objective, milestone.id, -1)} title="Move up">↑</button>
                      <button className={s.smallAction} onClick={() => reorderMilestones(objective, milestone.id, 1)} title="Move down">↓</button>
                    </>
                  )}
                  {milestone.status === "pending" && (
                    <button className={s.smallActionMuted} onClick={() => skipMilestone(milestone.id)}>
                      Skip
                    </button>
                  )}
                  <button className={s.smallActionMuted} onClick={() => deleteMilestone(milestone.id)} title="Delete milestone">
                    ×
                  </button>
                </div>
              </div>
            ))}
            {addingMilestoneFor === objective.id ? (
              <div className={s.newMilestoneRow}>
                <input
                  value={newMilestoneTitle}
                  onChange={(event) => setNewMilestoneTitle(event.target.value)}
                  placeholder="Add a milestone..."
                  className={s.objectiveInput}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && newMilestoneTitle.trim()) {
                      void createMilestone(objective.id);
                    }
                  }}
                />
                <button className={s.templateApply} onClick={() => createMilestone(objective.id)}>
                  Add
                </button>
                <button className={s.smallActionMuted} onClick={() => setAddingMilestoneFor(null)}>
                  Cancel
                </button>
              </div>
            ) : (
              <button className={s.inlineAdder} onClick={() => setAddingMilestoneFor(objective.id)}>
                + Milestone
              </button>
            )}
          </div>

          {objective.linkedActions.length > 0 && (
            <div className={s.linkedActions}>
              <div className={s.subheading}>Linked Actions</div>
              {objective.linkedActions.map((action) => (
                <div key={action.id} className={s.linkedActionRow}>
                  <div className={s.linkedActionMain}>
                    <ActionRow variant="compact" action={action} formatDate={formatShortDate} />
                  </div>
                  <button className={s.smallActionMuted} onClick={() => unlinkAction(action.id, objective.id)}>
                    Unlink
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
        );
      })}

      <div className={s.actionSection}>
        <div className={s.sectionLabel}>Actions</div>
        {unlinkedActions.length > 0 ? (
          <>
            <ActionGroup label="Overdue" labelColor="var(--color-spice-terracotta)" actions={overdue} objectives={data.objectives ?? []} onLink={linkAction} linkingActionId={linkingActionId} setLinkingActionId={setLinkingActionId} accentColor="var(--color-spice-terracotta)" dateColor="var(--color-spice-terracotta)" bold />
            <ActionGroup label="This Week" labelColor="var(--color-spice-turmeric)" actions={thisWeek} objectives={data.objectives ?? []} onLink={linkAction} linkingActionId={linkingActionId} setLinkingActionId={setLinkingActionId} accentColor="var(--color-spice-turmeric)" />
            <ActionGroup label="Upcoming" labelColor="var(--color-text-tertiary)" actions={upcoming} objectives={data.objectives ?? []} onLink={linkAction} linkingActionId={linkingActionId} setLinkingActionId={setLinkingActionId} />
            {noDue.length > 0 && (
              <ActionGroup label="No Due Date" labelColor="var(--color-text-tertiary)" actions={noDue} objectives={data.objectives ?? []} onLink={linkAction} linkingActionId={linkingActionId} setLinkingActionId={setLinkingActionId} />
            )}
          </>
        ) : (
          <p className={s.emptyMessage}>No unlinked actions.</p>
        )}

        {setAddingAction && onCreateAction && (
          <div className={s.actionComposer}>
            {addingAction ? (
              <>
                <input
                  value={newActionTitle ?? ""}
                  onChange={(event) => setNewActionTitle?.(event.target.value)}
                  placeholder="New action..."
                  className={s.objectiveInput}
                  autoFocus
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && (newActionTitle ?? "").trim()) onCreateAction();
                    if (event.key === "Escape") setAddingAction(false);
                  }}
                />
                <button
                  className={s.templateApply}
                  disabled={creatingAction || !(newActionTitle ?? "").trim()}
                  onClick={onCreateAction}
                >
                  {creatingAction ? "…" : "Add"}
                </button>
                <button className={s.smallActionMuted} onClick={() => setAddingAction(false)}>
                  Cancel
                </button>
              </>
            ) : (
              <button className={s.inlineAdder} onClick={() => setAddingAction(true)}>
                + Add Action
              </button>
            )}
          </div>
        )}
      </div>

      {upcomingMeetings.length > 0 && (
        <div className={s.meetingSection}>
          <div className={s.sectionLabel}>Upcoming Meetings</div>
          {upcomingMeetings.map((meeting) => (
            <MeetingRow
              key={meeting.id}
              meeting={meeting}
              formatDate={formatMeetingRowDate}
              formatType={formatMeetingType}
              typeBadgeStyle={meetingTypeBadgeStyle}
            />
          ))}
        </div>
      )}
    </section>
  );
}

interface ActionGroupProps {
  label: string;
  labelColor: string;
  actions: WorkSource["openActions"];
  objectives: AccountObjective[];
  onLink: (actionId: string, objectiveId: string) => Promise<void>;
  linkingActionId: string | null;
  setLinkingActionId: (value: string | null) => void;
  accentColor?: string;
  dateColor?: string;
  bold?: boolean;
}

function ActionGroup({
  label,
  labelColor,
  actions,
  objectives,
  onLink,
  linkingActionId,
  setLinkingActionId,
  accentColor,
  dateColor,
  bold,
}: ActionGroupProps) {
  if (actions.length === 0) return null;
  return (
    <div className={s.actionGroup}>
      <div className={s.groupLabel} style={{ '--label-color': labelColor } as React.CSSProperties}>
        {label}
      </div>
      {actions.map((action) => (
        <div key={action.id} className={s.linkableActionRow}>
          <div className={s.linkableActionMain}>
            <ActionRow
              variant="compact"
              action={action}
              accentColor={accentColor}
              dateColor={dateColor}
              bold={bold}
              formatDate={formatShortDate}
            />
          </div>
          {objectives.length > 0 && (
            <div className={s.linkBox}>
              {linkingActionId === action.id ? (
                <select
                  className={s.linkSelect}
                  defaultValue=""
                  onChange={(event) => {
                    if (event.target.value) {
                      void onLink(action.id, event.target.value);
                    }
                  }}
                >
                  <option value="">Link to objective…</option>
                  {objectives.map((objective) => (
                    <option key={objective.id} value={objective.id}>
                      {objective.title}
                    </option>
                  ))}
                </select>
              ) : (
                <button className={s.smallAction} onClick={() => setLinkingActionId(action.id)}>
                  Link
                </button>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  AlignLeft,
  Archive,
  Briefcase,
  CheckSquare2,
  Compass,
  Eye,
  FileText,
  GitBranch,
  Plus,
  RotateCcw,
  Save,
  Sparkles,
  Users,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { LinearIssuesChapter } from "@/components/entity/LinearIssuesChapter";
import { ReactBlockRenderer } from "@/components/composition/ReactBlockRenderer";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { useProjectDetail } from "@/hooks/useProjectDetail";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import {
  useChapterLayout,
  type RenderableCompositionBlock,
  type RenderableCompositionSection,
} from "@/hooks/useChapterLayout";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import type { ProjectDetail } from "@/types";
import type { ProjectedBlock } from "@/services/composition/contracts";
import shared from "@/styles/entity-detail.module.css";
import accountStyles from "./AccountDetailPage.module.css";
import styles from "./ProjectDetailEditorial.module.css";

const SECTION_ICONS: Record<string, React.ReactNode> = {
  headline: <AlignLeft size={18} strokeWidth={1.5} />,
  portfolio: <Briefcase size={18} strokeWidth={1.5} />,
  trajectory: <Activity size={18} strokeWidth={1.5} />,
  "the-horizon": <Compass size={18} strokeWidth={1.5} />,
  "the-landscape": <Eye size={18} strokeWidth={1.5} />,
  "the-room": <Users size={18} strokeWidth={1.5} />,
  "whats-next": <CheckSquare2 size={18} strokeWidth={1.5} />,
  "the-record": <FileText size={18} strokeWidth={1.5} />,
  "the-work": <CheckSquare2 size={18} strokeWidth={1.5} />,
  "linear-issues": <CheckSquare2 size={18} strokeWidth={1.5} />,
};

function projectNameFromBlocks(
  blocks: ProjectedBlock[],
  detail: ProjectDetail | null,
  projectId: string | undefined,
): string {
  const overview = blocks.find((block) => block.selected_known_type_id === "account_overview");
  const project = overview?.payload.project;
  if (project && typeof project === "object" && !Array.isArray(project)) {
    const displayName = (project as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  const account = overview?.payload.account;
  if (account && typeof account === "object" && !Array.isArray(account)) {
    const displayName = (account as Record<string, unknown>).display_name;
    if (typeof displayName === "string" && displayName.trim()) return displayName;
  }
  return detail?.name ?? projectId ?? "Project";
}

function ProjectControlsPanel({
  detail,
  project,
  onArchive,
  onAfterMutation,
}: {
  detail: ProjectDetail;
  project: ReturnType<typeof useProjectDetail>;
  onArchive: () => void;
  onAfterMutation: () => Promise<void>;
}) {
  const runMutationAndRefresh = (mutation: () => Promise<void>) => {
    void (async () => {
      await mutation();
      await onAfterMutation();
    })();
  };

  return (
    <section className={styles.projectControlPanel} aria-label="Project controls">
      <form
        className={styles.projectControlForm}
        onSubmit={(event) => {
          event.preventDefault();
          runMutationAndRefresh(project.handleSave);
        }}
      >
        <label className={styles.projectControlField}>
          <span>Name</span>
          <Input
            value={project.editName}
            onChange={(event) => {
              project.setEditName(event.target.value);
              project.setDirty(true);
            }}
          />
        </label>
        <label className={styles.projectControlField}>
          <span>Status</span>
          <Input
            value={project.editStatus}
            onChange={(event) => {
              project.setEditStatus(event.target.value);
              project.setDirty(true);
            }}
          />
        </label>
        <label className={styles.projectControlField}>
          <span>Milestone</span>
          <Input
            value={project.editMilestone}
            onChange={(event) => {
              project.setEditMilestone(event.target.value);
              project.setDirty(true);
            }}
          />
        </label>
        <label className={styles.projectControlField}>
          <span>Owner</span>
          <Input
            value={project.editOwner}
            onChange={(event) => {
              project.setEditOwner(event.target.value);
              project.setDirty(true);
            }}
          />
        </label>
        <label className={styles.projectControlField}>
          <span>Target</span>
          <Input
            value={project.editTargetDate}
            onChange={(event) => {
              project.setEditTargetDate(event.target.value);
              project.setDirty(true);
            }}
          />
        </label>
        <div className={styles.projectControlActions}>
          <Button type="submit" disabled={!project.dirty || project.saving}>
            <Save size={15} strokeWidth={1.7} />
            {project.saving ? "Saving" : "Save"}
          </Button>
          <Button
            type="button"
            variant="ghost"
            disabled={!project.dirty}
            onClick={project.handleCancelEdit}
          >
            <X size={15} strokeWidth={1.7} />
            Cancel
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={() => runMutationAndRefresh(project.handleEnrich)}
            disabled={project.enriching}
          >
            <Sparkles size={15} strokeWidth={1.7} />
            {project.enriching ? `${project.enrichSeconds}s` : "Refresh"}
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={detail.archived ? () => runMutationAndRefresh(project.handleUnarchive) : onArchive}
          >
            {detail.archived ? (
              <RotateCcw size={15} strokeWidth={1.7} />
            ) : (
              <Archive size={15} strokeWidth={1.7} />
            )}
            {detail.archived ? "Restore" : "Archive"}
          </Button>
        </div>
      </form>
      <div className={styles.projectActionComposer}>
        {project.addingAction ? (
          <>
            <Input
              value={project.newActionTitle}
              onChange={(event) => project.setNewActionTitle(event.target.value)}
              placeholder="Action title"
            />
            <Button
              type="button"
              onClick={() => runMutationAndRefresh(project.handleCreateAction)}
              disabled={project.creatingAction || !project.newActionTitle.trim()}
            >
              <Plus size={15} strokeWidth={1.7} />
              {project.creatingAction ? "Creating" : "Create"}
            </Button>
            <Button type="button" variant="ghost" onClick={() => project.setAddingAction(false)}>
              <X size={15} strokeWidth={1.7} />
              Cancel
            </Button>
          </>
        ) : (
          <Button type="button" variant="ghost" onClick={() => project.setAddingAction(true)}>
            <Plus size={15} strokeWidth={1.7} />
            Action
          </Button>
        )}
      </div>
    </section>
  );
}

function ProjectHierarchyNav({
  detail,
  navigate,
}: {
  detail: ProjectDetail;
  navigate: ReturnType<typeof useNavigate>;
}) {
  if (!detail.parentId && detail.children.length === 0) return null;
  return (
    <section className={styles.projectHierarchyPanel} aria-label="Project hierarchy">
      <div className={styles.projectPanelLabel}>
        <GitBranch size={14} strokeWidth={1.7} />
        Hierarchy
      </div>
      {detail.parentId && (
        <button
          type="button"
          className={styles.projectHierarchyLink}
          onClick={() =>
            navigate({
              to: "/projects/$projectId",
              params: { projectId: detail.parentId ?? "" },
            })
          }
        >
          {detail.parentName ?? "Parent project"}
        </button>
      )}
      {detail.children.length > 0 && (
        <div className={styles.projectHierarchyChildren}>
          {detail.children.map((child) => (
            <button
              type="button"
              key={child.id}
              className={styles.projectHierarchyChild}
              onClick={() =>
                navigate({
                  to: "/projects/$projectId",
                  params: { projectId: child.id },
                })
              }
            >
              <span>{child.name}</span>
              <span>{child.status.replace(/_/g, " ")}</span>
            </button>
          ))}
        </div>
      )}
    </section>
  );
}

export default function ProjectDetailEditorial() {
  const { projectId } = useParams({ strict: false });
  const navigate = useNavigate();
  const project = useProjectDetail(projectId);
  const composition = useProjectedComposition(
    projectId ? { entityType: "project", entityId: projectId } : undefined,
  );
  const projection = composition.data?.projection ?? null;
  const layout = useChapterLayout({ projection, entityType: "project" });
  const visibleSections = layout.view.sections;
  const projectName = projectNameFromBlocks(projection?.blocks ?? [], project.detail, projectId);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);
  const [ancestors, setAncestors] = useState<{ id: string; name: string }[]>([]);

  useRevealObserver(!composition.loading && !!projection);

  const forceRefreshProjectedComposition = useCallback(
    () => composition.refetch({ forceRefresh: true }),
    [composition.refetch],
  );

  useEffect(() => {
    if (!projectId) return;
    invoke<{ id: string; name: string }[]>("get_project_ancestors", { projectId })
      .then(setAncestors)
      .catch((err) => {
        console.error("get_project_ancestors failed:", err);
        setAncestors([]);
      });
  }, [projectId]);

  const chapters = useMemo(
    () => [
      ...visibleSections.map(({ section, label }) => ({
        id: section.section_id,
        label,
        icon: SECTION_ICONS[section.section_id] ?? <FileText size={18} strokeWidth={1.5} />,
      })),
      ...(project.detail
        ? [
            {
              id: "linear-issues",
              label: "Linear Issues",
              icon: SECTION_ICONS["linear-issues"],
            },
          ]
        : []),
    ],
    [project.detail, visibleSections],
  );

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Project",
      atmosphereColor: "olive" as const,
      activePage: "projects" as const,
      breadcrumbs: [
        { label: "Projects", onClick: () => navigate({ to: "/projects" }) },
        ...ancestors.map((ancestor) => ({
          label: ancestor.name,
          onClick: () => navigate({ to: "/projects/$projectId", params: { projectId: ancestor.id } }),
        })),
        { label: projectName },
      ],
      chapters,
      folioStatusText: composition.loading
        ? "Composing..."
        : project.saving
          ? "Saving..."
          : composition.data?.served_from_cache
            ? "Projected from cache"
            : undefined,
      folioActions: (
        <div className={shared.folioActions}>
          {project.detail?.isParent && (
            <button
              type="button"
              onClick={() => project.setCreateChildOpen(true)}
              className={styles.addChildButton}
            >
              + Sub-Project
            </button>
          )}
          <FolioRefreshButton onClick={composition.refetch} loading={composition.loading} />
        </div>
      ),
    }),
    [
      ancestors,
      chapters,
      composition.data?.served_from_cache,
      composition.loading,
      composition.refetch,
      navigate,
      project.detail?.isParent,
      project.saving,
      project.setCreateChildOpen,
      projectName,
    ],
  );
  useRegisterMagazineShell(shellConfig);

  if (composition.loading && !projection) return <EditorialLoading />;

  if (composition.error) {
    return <EditorialError message={composition.error} onRetry={composition.refetch} />;
  }

  if (!projection || projection.sections.length === 0 || projection.blocks.length === 0) {
    return (
      <EditorialEmpty
        title="No project composition"
        message="DailyOS has not produced a project surface yet."
      />
    );
  }

  const renderBlock = (item: RenderableCompositionBlock) => (
    <ReactBlockRenderer
      key={item.block.block_id}
      block={item.block}
      entityId={projectId}
      entityType="project"
      renderedProvenance={composition.renderedProvenance}
    />
  );

  const renderSectionTitle = (section: RenderableCompositionSection) => (
    <h2 className={accountStyles.compositionSectionTitle}>{section.label}</h2>
  );

  const detail = project.detail;

  return (
    <main
      className={accountStyles.compositionSurface}
      data-composition-id={projection.composition_id}
      data-composition-version={projection.composition_version ?? 0}
      data-fallback-policy-version={projection.fallback_policy_version}
    >
      {project.error && (
        <div className={accountStyles.compositionDegradedState} role="status">
          <p className={accountStyles.compositionStateLabel}>Project controls unavailable</p>
          <p className={accountStyles.compositionStateText}>{project.error}</p>
        </div>
      )}

      {visibleSections.map((renderableSection) => {
        const section = renderableSection.section;
        const blocks = renderableSection.blocks;
        if (section.section_id === "headline") {
          return (
            <section
              key={section.section_id}
              id={section.section_id}
              className={accountStyles.compositionMasthead}
              data-section-id={section.section_id}
              data-section-layout={section.layout}
            >
              <div className={accountStyles.compositionMastheadGrid}>
                {blocks.map(renderBlock)}
              </div>
            </section>
          );
        }

        return (
          <section
            key={section.section_id}
            id={section.section_id}
            className={accountStyles.compositionSection}
            data-section-id={section.section_id}
            data-section-layout={section.layout}
            data-section-salience={section.salience.band}
          >
            <div className={accountStyles.compositionSectionLabel}>{renderableSection.label}</div>
            <div className={accountStyles.compositionSectionBody}>
              <header className={accountStyles.compositionSectionHeader}>
                <div>{renderSectionTitle(renderableSection)}</div>
                <p className={accountStyles.compositionSectionMeta}>{section.salience.reason}</p>
              </header>
              <div className={accountStyles.compositionBlockStack}>
                {blocks.length > 0 ? (
                  blocks.map(renderBlock)
                ) : (
                  <div className={accountStyles.compositionDegradedState}>
                    <p className={accountStyles.compositionStateLabel}>Empty section</p>
                    <p className={accountStyles.compositionStateText}>
                      No renderable blocks are available for this section.
                    </p>
                  </div>
                )}
              </div>
            </div>
          </section>
        );
      })}

      {detail && (
        <>
          <ProjectControlsPanel
            detail={detail}
            project={project}
            onArchive={() => setArchiveDialogOpen(true)}
            onAfterMutation={forceRefreshProjectedComposition}
          />
          <ProjectHierarchyNav detail={detail} navigate={navigate} />
          <LinearIssuesChapter entityRef={{ kind: "project", id: detail.id }} actorScope="user" />
        </>
      )}

      <FinisMarker />

      {detail && (
        <AlertDialog open={archiveDialogOpen} onOpenChange={setArchiveDialogOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Archive Project</AlertDialogTitle>
              <AlertDialogDescription>
                This will hide {detail.name} from active views. You can restore it later.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction onClick={project.handleArchive}>Archive</AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}

      {detail && (
        <Dialog open={project.createChildOpen} onOpenChange={project.setCreateChildOpen}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Create Sub-Project</DialogTitle>
              <DialogDescription>Create a new sub-project under {detail.name}.</DialogDescription>
            </DialogHeader>
            <div className={shared.dialogForm}>
              <Input
                value={project.childName}
                onChange={(event) => project.setChildName(event.target.value)}
                placeholder="Name"
              />
              <Input
                value={project.childDescription}
                onChange={(event) => project.setChildDescription(event.target.value)}
                placeholder="Description (optional)"
              />
              <div className={shared.dialogActions}>
                <Button
                  variant="ghost"
                  onClick={() => project.setCreateChildOpen(false)}
                  className={shared.dialogButton}
                >
                  Cancel
                </Button>
                <Button
                  onClick={project.handleCreateChild}
                  disabled={project.creatingChild || !project.childName.trim()}
                  className={shared.dialogButton}
                >
                  {project.creatingChild ? "Creating..." : "Create"}
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </main>
  );
}

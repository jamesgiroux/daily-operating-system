import React, { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { VitalDisplay } from "@/lib/entity-types";
import { useProjectDetail } from "@/hooks/useProjectDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import {
  AlignLeft,
  Briefcase,
  TrendingUp,
  Compass,
  Users,
  Eye,
  Activity,
  CheckSquare2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
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
import { ProjectHero } from "@/components/project/ProjectHero";
import { ProjectAppendix } from "@/components/project/ProjectAppendix";
import { WatchListMilestones } from "@/components/project/WatchListMilestones";
import { TrajectoryChapter } from "@/components/project/TrajectoryChapter";
import { HorizonChapter } from "@/components/project/HorizonChapter";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { RecommendedActions } from "@/components/entity/RecommendedActions";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { useEntityContextEntries } from "@/hooks/useEntityContextEntries";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import shared from "@/styles/entity-detail.module.css";
import styles from "./ProjectDetailEditorial.module.css";

/* ── Vitals assembly ── */

function buildProjectVitals(detail: {
  status?: string;
  owner?: string;
  targetDate?: string;
  milestones?: { status: string }[];
  signals?: {
    meetingFrequency30d?: number;
    meetingFrequency90d?: number;
    openActionCount?: number;
    daysUntilTarget?: number;
    trend?: string;
  };
}): VitalDisplay[] {
  const vitals: VitalDisplay[] = [];
  if (detail.status) {
    vitals.push({
      text: detail.status.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
      highlight: "olive",
    });
  }
  if (detail.signals?.daysUntilTarget != null) {
    const trend = detail.signals.trend;
    const arrow = trend === "improving" ? " \u2191" : trend === "declining" ? " \u2193" : "";
    vitals.push({ text: `${detail.signals.daysUntilTarget}d to target${arrow}` });
  }
  if (detail.milestones) {
    const done = detail.milestones.filter(
      (m) => m.status.toLowerCase() === "completed" || m.status.toLowerCase() === "done",
    ).length;
    const total = detail.milestones.length;
    if (total > 0) vitals.push({ text: `${done} of ${total} milestones` });
  }
  if (detail.signals?.meetingFrequency30d != null) {
    const f30 = detail.signals.meetingFrequency30d;
    const f90 = detail.signals.meetingFrequency90d;
    const arrow =
      f90 != null && f90 > 0
        ? f30 > f90 / 3 ? " \u2191" : f30 < f90 / 3 ? " \u2193" : ""
        : "";
    vitals.push({ text: `${f30} meetings / 30d${arrow}` });
  }
  if (detail.signals?.openActionCount != null) {
    vitals.push({ text: `${detail.signals.openActionCount} open actions` });
  }
  return vitals;
}

/* ── Chapters ── */

const BASE_CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Mission", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "trajectory", label: "Trajectory", icon: <TrendingUp size={18} strokeWidth={1.5} /> },
  { id: "the-horizon", label: "The Horizon", icon: <Compass size={18} strokeWidth={1.5} /> },
  { id: "the-landscape", label: "The Landscape", icon: <Eye size={18} strokeWidth={1.5} /> },
  { id: "the-room", label: "The Team", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
  { id: "the-work", label: "The Work", icon: <CheckSquare2 size={18} strokeWidth={1.5} /> },
];

const PORTFOLIO_CHAPTER = {
  id: "portfolio",
  label: "Portfolio",
  icon: <Briefcase size={18} strokeWidth={1.5} />,
};

function buildChapters(isParent: boolean) {
  if (!isParent) return BASE_CHAPTERS;
  // Portfolio appears after headline, before Trajectory
  return [BASE_CHAPTERS[0], PORTFOLIO_CHAPTER, ...BASE_CHAPTERS.slice(1)];
}

function getStatusColorClass(status: string): string {
  if (status === "active") return styles.statusActive;
  if (status === "on_hold") return styles.statusOnHold;
  return styles.statusCompleted;
}

function getStatusDotClass(status: string): string {
  if (status === "active") return styles.statusDotActive;
  if (status === "on_hold") return styles.statusDotOnHold;
  return styles.statusDotCompleted;
}

export default function ProjectDetailEditorial() {
  const { projectId } = useParams({ strict: false });
  const navigate = useNavigate();
  const proj = useProjectDetail(projectId);
  const preset = useActivePreset();
  useRevealObserver(!proj.loading && !!proj.detail);

  // Shared intelligence field update hook (must be before shellConfig useMemo)
  const {
    updateField: handleUpdateIntelField,
    saveStatus,
    setSaveStatus: setFolioSaveStatus,
  } = useIntelligenceFieldUpdate("project", projectId, proj.silentRefresh);

  const finishFolioSave = () => {
    setFolioSaveStatus("saved");
    window.setTimeout(() => setFolioSaveStatus("idle"), 2000);
  };

  const saveMetadata = async (updated: Record<string, string>) => {
    if (!projectId) return;
    setFolioSaveStatus("saving");
    try {
      await invoke("update_entity_metadata", {
        entityId: projectId,
        entityType: "project",
        metadata: JSON.stringify(updated),
      });
      finishFolioSave();
    } catch (err) {
      console.error("update_entity_metadata failed:", err);
      toast.error("Failed to save metadata");
      setFolioSaveStatus("idle");
      throw err;
    }
  };

  const saveProjectField = async (field: string, value: string) => {
    if (!proj.detail) return;
    setFolioSaveStatus("saving");
    try {
      await invoke("update_project_field", { projectId: proj.detail.id, field, value });
      await proj.load();
      finishFolioSave();
    } catch (err) {
      console.error("update_project_field failed:", err);
      toast.error("Failed to save field");
      setFolioSaveStatus("idle");
    }
  };

  // Fetch ancestor projects for FolioBar breadcrumb navigation.
  const [ancestors, setAncestors] = useState<{ id: string; name: string }[]>([]);
  useEffect(() => {
    if (!projectId) return;
    invoke<{ id: string; name: string }[]>("get_project_ancestors", { projectId })
      .then(setAncestors)
      .catch((err) => {
        console.error("get_project_ancestors failed:", err); // Expected: background data fetch on mount
        setAncestors([]);
      });
  }, [projectId]);

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
        { label: proj.detail?.name ?? "Project" },
      ],
      chapters: buildChapters(proj.detail?.isParent ?? false),
      folioStatusText: saveStatus === "saving" ? "Saving\u2026" : saveStatus === "saved" ? "\u2713 Saved" : undefined,
      folioActions: proj.detail?.isParent ? (
        <button
          onClick={() => proj.setCreateChildOpen(true)}
          className={styles.addChildButton}
        >
          + Sub-Project
        </button>
      ) : undefined,
    }),
    [ancestors, navigate, proj.detail, proj.setCreateChildOpen, saveStatus],
  );
  useRegisterMagazineShell(shellConfig);

  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  // Preset metadata state
  const [metadataValues, setMetadataValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!projectId) return;
    invoke<string>("get_entity_metadata", { entityType: "project", entityId: projectId })
      .then((json) => {
        try { setMetadataValues(JSON.parse(json) ?? {}); } catch { setMetadataValues({}); }
      })
      .catch((err) => {
        console.error("get_entity_metadata (project) failed:", err); // Expected: background data fetch on mount
        setMetadataValues({});
      });
  }, [projectId]);

  // Intelligence quality feedback
  const feedback = useIntelligenceFeedback(projectId, "project");

  // Context entries — must be before early returns (React hooks rule)
  const entityCtx = useEntityContextEntries("project", projectId ?? null);

  if (proj.loading) return <EditorialLoading />;

  if (proj.error || !proj.detail) {
    return <EditorialError message={proj.error ?? "Project not found"} onRetry={proj.load} />;
  }

  const { detail, intelligence, files } = proj;

  return (
    <>
      {/* Chapter 1: The Mission (Hero) */}
      <section id="headline" className={shared.chapterSection}>
        <ProjectHero
          detail={detail}
          intelligence={intelligence}
          editName={proj.editName}
          setEditName={(v) => { proj.setEditName(v); proj.setDirty(true); }}
          editStatus={proj.editStatus}
          setEditStatus={(v) => { proj.setEditStatus(v); proj.setDirty(true); }}
          onSave={proj.handleSave}
          onSaveField={proj.saveField}
          onEnrich={proj.handleEnrich}
          enriching={proj.enriching}
          enrichSeconds={proj.enrichSeconds}
          onArchive={() => setArchiveDialogOpen(true)}
          onUnarchive={proj.handleUnarchive}
        />
        <div className="editorial-reveal">
          {preset ? (
            <EditableVitalsStrip
              fields={preset.vitals.project}
              metadataFields={preset.metadata.project}
              entityData={detail}
              metadata={metadataValues}
              onFieldChange={(key, columnMapping, source, value) => {
                if (source === "metadata") {
                  setMetadataValues((prev) => {
                    const updated = { ...prev, [key]: value };
                    void saveMetadata(updated);
                    return updated;
                  });
                } else if (source === "column") {
                  const field = columnMapping ?? key;
                  void saveProjectField(field, value);
                }
              }}
            />
          ) : (
            <VitalsStrip vitals={buildProjectVitals(detail)} />
          )}
        </div>
      </section>

      {/* Portfolio chapter — only for parent projects */}
      {detail.isParent && detail.children.length > 0 && (
        <section id="portfolio" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
          <ChapterHeading title="Portfolio" />

          {/* Portfolio narrative */}
          {intelligence?.portfolio?.portfolioNarrative && (
            <div className={shared.portfolioNarrative}>
              <p className={shared.portfolioNarrativeText}>
                {intelligence.portfolio.portfolioNarrative}
              </p>
            </div>
          )}

          {/* Hotspots — child projects needing attention */}
          {intelligence?.portfolio?.hotspots && intelligence.portfolio.hotspots.length > 0 && (
            <div className={shared.portfolioHotspotsSection}>
              <div className={shared.portfolioSectionLabelTerracotta}>
                Needs Attention
              </div>
              {intelligence.portfolio.hotspots.map((hotspot, i) => (
                <div
                  key={hotspot.childId}
                  className={
                    i === intelligence.portfolio!.hotspots.length - 1
                      ? shared.hotspotRow
                      : shared.hotspotRowBorder
                  }
                >
                  <span className={shared.hotspotDot} />
                  <div className={shared.hotspotContent}>
                    <button
                      onClick={() =>
                        navigate({
                          to: "/projects/$projectId",
                          params: { projectId: hotspot.childId },
                        })
                      }
                      className={styles.hotspotLinkOlive}
                    >
                      {hotspot.childName}
                    </button>
                    <p className={shared.hotspotReason}>
                      {hotspot.reason}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Cross-project patterns — only shown when non-empty */}
          {intelligence?.portfolio?.crossBuPatterns && intelligence.portfolio.crossBuPatterns.length > 0 && (
            <div className={shared.crossPatternsBlock}>
              <div className={shared.portfolioSectionLabelLarkspur}>
                Cross-Project Patterns
              </div>
              {intelligence.portfolio.crossBuPatterns.map((pattern, i) => (
                <p
                  key={i}
                  className={i === 0 ? shared.crossPatternTextFirst : shared.crossPatternTextSubsequent}
                >
                  {pattern}
                </p>
              ))}
            </div>
          )}

          {/* Condensed child list */}
          <div className={shared.childListSection}>
            <div className={shared.portfolioSectionLabelTertiary}>
              Sub-Projects
            </div>
            {detail.children.map((child, i) => (
              <div
                key={child.id}
                className={
                  i === detail.children.length - 1
                    ? shared.childRow
                    : shared.childRowBorder
                }
              >
                <button
                  onClick={() =>
                    navigate({
                      to: "/projects/$projectId",
                      params: { projectId: child.id },
                    })
                  }
                  className={shared.childNameButton}
                >
                  {child.name}
                </button>
                {/* Status indicator */}
                <span className={`${shared.statusIndicator} ${getStatusColorClass(child.status)}`}>
                  <span className={getStatusDotClass(child.status)} />
                  {child.status === "active"
                    ? "Active"
                    : child.status === "on_hold"
                      ? "On Hold"
                      : "Completed"}
                </span>
                {/* Open actions count */}
                {child.openActionCount > 0 && (
                  <span className={shared.secondaryMetric}>
                    {child.openActionCount} action{child.openActionCount !== 1 ? "s" : ""}
                  </span>
                )}
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Chapter 2: Trajectory */}
      <div id="trajectory" className={`editorial-reveal ${shared.chapterSection}`}>
        <TrajectoryChapter
          detail={detail}
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          feedbackSlot={
            <IntelligenceFeedback
              value={feedback.getFeedback("trajectory")}
              onFeedback={(type) => feedback.submitFeedback("trajectory", type)}
            />
          }
        />
      </div>

      {/* Chapter 3: The Horizon */}
      <div id="the-horizon" className={`editorial-reveal ${shared.chapterSection}`}>
        <HorizonChapter
          detail={detail}
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          feedbackSlot={
            <IntelligenceFeedback
              value={feedback.getFeedback("horizon")}
              onFeedback={(type) => feedback.submitFeedback("horizon", type)}
            />
          }
        />
      </div>

      {/* Chapter 4: The Landscape */}
      <div id="the-landscape" className={`editorial-reveal ${shared.chapterSection}`}>
        <WatchList
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          sectionId="the-landscape"
          chapterTitle="The Landscape"
          getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
          onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
          bottomSection={
            detail.milestones.length > 0 ? (
              <WatchListMilestones milestones={detail.milestones} />
            ) : undefined
          }
        />
      </div>

      {/* Chapter 5: The Team */}
      <div id="the-room" className={`editorial-reveal ${shared.chapterSection}`}>
        <StakeholderGallery
          intelligence={intelligence}
          linkedPeople={detail.linkedPeople}
          chapterTitle="The Team"
          sectionId="the-room"
          entityId={projectId}
          entityType="project"
          onIntelligenceUpdated={proj.silentRefresh}
        />
      </div>

      {/* Chapter 6: The Record */}
      <div id="the-record" className={`editorial-reveal ${shared.chapterSection}`}>
        <UnifiedTimeline
          data={{ ...detail, contextEntries: entityCtx.entries }}
          sectionId=""
          actionSlot={<AddToRecord onAdd={(title, content) => entityCtx.createEntry(title, content)} />}
        />
      </div>

      {/* Chapter 7: The Work  */}
      <div id="the-work" className={`editorial-reveal ${shared.chapterSection}`}>
        {intelligence?.recommendedActions && intelligence.recommendedActions.length > 0 && (
          <RecommendedActions entityId={detail.id} entityType="project"
            actions={intelligence.recommendedActions} onRefresh={proj.silentRefresh} />
        )}
        <TheWork
          data={detail}
          addingAction={proj.addingAction}
          setAddingAction={proj.setAddingAction}
          newActionTitle={proj.newActionTitle}
          setNewActionTitle={proj.setNewActionTitle}
          creatingAction={proj.creatingAction}
          onCreateAction={proj.handleCreateAction}
        />
      </div>

      {/* Appendix */}
      <div className="editorial-reveal">
        <ProjectAppendix
          detail={detail}
          files={files}
          onIndexFiles={proj.handleIndexFiles}
          indexing={proj.indexing}
          indexFeedback={proj.indexFeedback}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={intelligence?.enrichedAt} />
      </div>

      {/* Archive Confirmation */}
      <AlertDialog open={archiveDialogOpen} onOpenChange={setArchiveDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Archive Project</AlertDialogTitle>
            <AlertDialogDescription>
              This will hide {detail.name} from active views.
              You can unarchive it later.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={proj.handleArchive}>Archive</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Sub-Project Creation Dialog */}
      <Dialog open={proj.createChildOpen} onOpenChange={proj.setCreateChildOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Create Sub-Project</DialogTitle>
            <DialogDescription>
              Create a new sub-project under {detail.name}.
            </DialogDescription>
          </DialogHeader>
          <div className={shared.dialogForm}>
            <Input
              value={proj.childName}
              onChange={(e) => proj.setChildName(e.target.value)}
              placeholder="Name"
            />
            <Input
              value={proj.childDescription}
              onChange={(e) => proj.setChildDescription(e.target.value)}
              placeholder="Description (optional)"
            />
            <div className={shared.dialogActions}>
              <Button
                variant="ghost"
                onClick={() => proj.setCreateChildOpen(false)}
                className={shared.dialogButton}
              >
                Cancel
              </Button>
              <Button
                onClick={proj.handleCreateChild}
                disabled={proj.creatingChild || !proj.childName.trim()}
                className={shared.dialogButton}
              >
                {proj.creatingChild ? "Creating..." : "Create"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

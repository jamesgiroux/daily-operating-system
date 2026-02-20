import React, { useState, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import type { VitalDisplay } from "@/lib/entity-types";
import { buildVitalsFromPreset } from "@/lib/preset-vitals";
import { useProjectDetail } from "@/hooks/useProjectDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import {
  AlignLeft,
  TrendingUp,
  Compass,
  Users,
  Eye,
  Activity,
  CheckSquare2,
} from "lucide-react";
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
import { ProjectHero } from "@/components/project/ProjectHero";
import { ProjectAppendix } from "@/components/project/ProjectAppendix";
import { ProjectFieldsDrawer } from "@/components/project/ProjectFieldsDrawer";
import { WatchListMilestones } from "@/components/project/WatchListMilestones";
import { TrajectoryChapter } from "@/components/project/TrajectoryChapter";
import { HorizonChapter } from "@/components/project/HorizonChapter";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { FinisMarker } from "@/components/editorial/FinisMarker";

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

const CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Mission", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "trajectory", label: "Trajectory", icon: <TrendingUp size={18} strokeWidth={1.5} /> },
  { id: "the-horizon", label: "The Horizon", icon: <Compass size={18} strokeWidth={1.5} /> },
  { id: "the-landscape", label: "The Landscape", icon: <Eye size={18} strokeWidth={1.5} /> },
  { id: "the-room", label: "The Team", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
  { id: "the-work", label: "The Work", icon: <CheckSquare2 size={18} strokeWidth={1.5} /> },
];

export default function ProjectDetailEditorial() {
  const { projectId } = useParams({ strict: false });
  const navigate = useNavigate();
  const proj = useProjectDetail(projectId);
  const preset = useActivePreset();
  useRevealObserver(!proj.loading && !!proj.detail);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Project",
      atmosphereColor: "olive" as const,
      activePage: "projects" as const,
      backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/projects" }) },
      chapters: CHAPTERS,
    }),
    [navigate],
  );
  useRegisterMagazineShell(shellConfig);

  const [fieldsDrawerOpen, setFieldsDrawerOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  // I352: Shared intelligence field update hook
  const { updateField: handleUpdateIntelField } = useIntelligenceFieldUpdate("project", projectId);

  if (proj.loading) return <EditorialLoading />;

  if (proj.error || !proj.detail) {
    return <EditorialError message={proj.error ?? "Project not found"} onRetry={proj.load} />;
  }

  const { detail, intelligence, files } = proj;
  const notesDirty = proj.editNotes !== (detail.notes ?? "");

  return (
    <>
      {/* Chapter 1: The Mission (Hero) */}
      <section id="headline" style={{ scrollMarginTop: 60 }}>
        <ProjectHero
          detail={detail}
          intelligence={intelligence}
          onEditFields={() => setFieldsDrawerOpen(true)}
          onEnrich={proj.handleEnrich}
          enriching={proj.enriching}
          enrichSeconds={proj.enrichSeconds}
          onArchive={() => setArchiveDialogOpen(true)}
          onUnarchive={proj.handleUnarchive}
        />
        <div className="editorial-reveal">
          <VitalsStrip vitals={preset ? buildVitalsFromPreset(preset.vitals.project, detail) : buildProjectVitals(detail)} />
        </div>
      </section>

      {/* Chapter 2: Trajectory */}
      <div id="trajectory" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <TrajectoryChapter detail={detail} intelligence={intelligence} onUpdateField={handleUpdateIntelField} />
      </div>

      {/* Chapter 3: The Horizon */}
      <div id="the-horizon" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <HorizonChapter detail={detail} intelligence={intelligence} onUpdateField={handleUpdateIntelField} />
      </div>

      {/* Chapter 4: The Landscape */}
      <div id="the-landscape" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <WatchList
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          sectionId="the-landscape"
          chapterTitle="The Landscape"
          bottomSection={
            detail.milestones.length > 0 ? (
              <WatchListMilestones milestones={detail.milestones} />
            ) : undefined
          }
        />
      </div>

      {/* Chapter 5: The Team */}
      <div id="the-room" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
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
      <div id="the-record" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <UnifiedTimeline data={detail} />
      </div>

      {/* Chapter 7: The Work (I351) */}
      <div id="the-work" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <TheWork
          data={detail}
          intelligence={intelligence}
          addingAction={proj.addingAction}
          setAddingAction={proj.setAddingAction}
          newActionTitle={proj.newActionTitle}
          setNewActionTitle={proj.setNewActionTitle}
          creatingAction={proj.creatingAction}
          onCreateAction={proj.handleCreateAction}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={intelligence?.enrichedAt} />
      </div>

      {/* Appendix */}
      <div className="editorial-reveal">
        <ProjectAppendix
          detail={detail}
          files={files}
          editNotes={proj.editNotes}
          setEditNotes={(v) => {
            proj.setEditNotes(v);
            proj.setDirty(true);
          }}
          onSaveNotes={proj.handleSave}
          notesDirty={notesDirty}
          onIndexFiles={proj.handleIndexFiles}
          indexing={proj.indexing}
          indexFeedback={proj.indexFeedback}
        />
      </div>

      {/* Fields Drawer */}
      <ProjectFieldsDrawer
        open={fieldsDrawerOpen}
        onOpenChange={setFieldsDrawerOpen}
        editName={proj.editName}
        setEditName={proj.setEditName}
        editStatus={proj.editStatus}
        setEditStatus={proj.setEditStatus}
        editMilestone={proj.editMilestone}
        setEditMilestone={proj.setEditMilestone}
        editOwner={proj.editOwner}
        setEditOwner={proj.setEditOwner}
        editTargetDate={proj.editTargetDate}
        setEditTargetDate={proj.setEditTargetDate}
        setDirty={proj.setDirty}
        onSave={proj.handleSave}
        onCancel={proj.handleCancelEdit}
        saving={proj.saving}
        dirty={proj.dirty}
      />

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
    </>
  );
}

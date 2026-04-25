import { useState, useEffect, useMemo, useCallback } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import { usePersonDetail } from "@/hooks/usePersonDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import {
  AlignLeft,
  Zap,
  RefreshCw,
  Network,
  Users,
  Eye,
  Activity,
  CheckSquare2,
} from "lucide-react";
import { Input } from "@/components/ui/input";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import { PersonHero } from "@/components/person/PersonHero";
import { PersonNetwork } from "@/components/person/PersonNetwork";
import { PersonRelationships } from "@/components/person/PersonRelationships";
import { PersonAppendix } from "@/components/person/PersonAppendix";
import { PersonInsightChapter } from "@/components/person/PersonInsightChapter";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { EditableVitalsStrip } from "@/components/entity/EditableVitalsStrip";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { RecommendedActions } from "@/components/entity/RecommendedActions";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { AddToRecord } from "@/components/entity/AddToRecord";
import { useEntityContextEntries } from "@/hooks/useEntityContextEntries";
import shared from "@/styles/entity-detail.module.css";
import styles from "./PersonDetailEditorial.module.css";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";

// Suppress unused import warning — styles reserved for future person-specific classes
void styles;

/* ── Vitals assembly ── */

function buildPersonVitals(detail: {
  meetingCount: number;
  signals?: {
    meetingFrequency30d: number;
    meetingFrequency90d: number;
    temperature: string;
    trend: string;
    lastMeeting?: string;
  };
}): VitalDisplay[] {
  const vitals: VitalDisplay[] = [];
  const sig = detail.signals;

  if (sig) {
    // Lead with temperature badge — relationship warmth signal
    if (sig.temperature) {
      vitals.push({
        text: sig.temperature,
        highlight: sig.temperature === "hot"
          ? "larkspur"
          : sig.temperature === "warm"
            ? "turmeric"
            : sig.temperature === "cold"
              ? "turmeric"
              : undefined,
      });
    }
    const trendArrow =
      sig.trend === "increasing" ? " \u2191" : sig.trend === "decreasing" ? " \u2193" : "";
    vitals.push({ text: `${sig.meetingFrequency30d} meetings / 30d${trendArrow}` });
    if (sig.lastMeeting) {
      vitals.push({ text: `Last: ${formatShortDate(sig.lastMeeting)}` });
    }
  }

  if (detail.meetingCount > 0) {
    vitals.push({ text: `${detail.meetingCount} total meetings` });
  }

  return vitals;
}

/* ── Chapters (adaptive based on relationship) ── */

function buildChapters(relationship: string) {
  const isInternal = relationship === "internal";
  return [
    { id: "headline", label: "The Profile", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
    {
      id: isInternal ? "the-rhythm" : "the-dynamic",
      label: isInternal ? "The Rhythm" : "The Dynamic",
      icon: isInternal
        ? <RefreshCw size={18} strokeWidth={1.5} />
        : <Zap size={18} strokeWidth={1.5} />,
    },
    { id: "their-orbit", label: "Their Orbit", icon: <Network size={18} strokeWidth={1.5} /> },
    { id: "their-network", label: "Their Network", icon: <Users size={18} strokeWidth={1.5} /> },
    { id: "the-landscape", label: "The Landscape", icon: <Eye size={18} strokeWidth={1.5} /> },
    { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
    { id: "the-work", label: "The Work", icon: <CheckSquare2 size={18} strokeWidth={1.5} /> },
  ];
}

export default function PersonDetailEditorial() {
  const { personId } = useParams({ strict: false });
  const navigate = useNavigate();
  const person = usePersonDetail(personId);
  const preset = useActivePreset();
  useRevealObserver(!person.loading && !!person.detail);

  // I352: Shared intelligence field update hook (must be before shellConfig useMemo)
  const {
    updateField: handleUpdateIntelField,
    saveStatus,
    setSaveStatus: setFolioSaveStatus,
  } = useIntelligenceFieldUpdate("person", personId, person.silentRefresh);

  const finishFolioSave = useCallback(() => {
    setFolioSaveStatus("saved");
    window.setTimeout(() => setFolioSaveStatus("idle"), 2000);
  }, [setFolioSaveStatus]);

  const saveMetadata = useCallback(
    async (updated: Record<string, string>) => {
      if (!personId) return;
      setFolioSaveStatus("saving");
      try {
        await invoke("update_entity_metadata", {
          entityId: personId,
          entityType: "person",
          metadata: JSON.stringify(updated),
        });
        finishFolioSave();
      } catch (err) {
        console.error("update_entity_metadata failed:", err);
        toast.error("Failed to save metadata");
        setFolioSaveStatus("idle");
        throw err;
      }
    },
    [finishFolioSave, personId, setFolioSaveStatus],
  );

  const relationship = person.detail?.relationship ?? "unknown";
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Person",
      atmosphereColor: "larkspur" as const,
      activePage: "people" as const,
      breadcrumbs: [
        { label: "People", onClick: () => navigate({ to: "/people" }) },
        { label: person.detail?.name ?? "Person" },
      ],
      chapters: buildChapters(relationship),
      folioStatusText: saveStatus === "saving" ? "Saving\u2026" : saveStatus === "saved" ? "\u2713 Saved" : undefined,
    }),
    [navigate, person.detail?.name, relationship, saveStatus],
  );
  useRegisterMagazineShell(shellConfig);

  // I390: Person relationships
  const [relationships, setRelationships] = useState<import("@/types").PersonRelationshipEdge[]>([]);
  const loadRelationships = useCallback(() => {
    if (!personId) return;
    invoke<import("@/types").PersonRelationshipEdge[]>("get_person_relationships", { personId })
      .then(setRelationships)
      .catch(() => setRelationships([]));
  }, [personId]);
  useEffect(() => { loadRelationships(); }, [loadRelationships]);

  // I312: Preset metadata state
  const [metadataValues, setMetadataValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!personId) return;
    invoke<string>("get_entity_metadata", { entityType: "person", entityId: personId })
      .then((json) => {
        try { setMetadataValues(JSON.parse(json) ?? {}); } catch { setMetadataValues({}); }
      })
      .catch((err) => {
        console.error("get_entity_metadata (person) failed:", err); // Expected: background data fetch on mount
        setMetadataValues({});
      });
  }, [personId]);

  // I529: Intelligence quality feedback
  const feedback = useIntelligenceFeedback(personId, "person");

  // Context entries — must be before early returns (React hooks rule)
  const entityCtx = useEntityContextEntries("person", personId ?? null);

  if (person.loading) return <EditorialLoading />;

  if (person.error || !person.detail) {
    return <EditorialError message={person.error ?? "Person not found"} onRetry={person.load} />;
  }

  const { detail, intelligence } = person;

  return (
    <>
      {/* Chapter 1: The Profile (Hero) */}
      <section id="headline" className={shared.chapterSection}>
        <PersonHero
          detail={detail}
          intelligence={intelligence}
          editName={person.editName}
          setEditName={(v) => { person.setEditName(v); person.setDirty(true); }}
          editRole={person.editRole}
          setEditRole={(v) => { person.setEditRole(v); person.setDirty(true); }}
          onSave={person.handleSave}
          onSaveField={person.saveField}
          onEnrich={person.handleEnrich}
          enriching={person.enriching}
          enrichSeconds={person.enrichSeconds}
          onMerge={person.openMergeDialog}
          onArchive={() => person.handleArchive()}
          onUnarchive={person.handleUnarchive}
          onDelete={() => person.setDeleteConfirmOpen(true)}
        />
        <div className="editorial-reveal">
          {preset ? (
            <EditableVitalsStrip
              fields={preset.vitals.person}
              metadataFields={preset.metadata.person}
              entityData={{ ...detail, signals: detail.signals as Record<string, unknown> | undefined }}
              metadata={metadataValues}
              onFieldChange={(key, _columnMapping, source, value) => {
                if (source === "metadata") {
                  setMetadataValues((prev) => {
                    const updated = { ...prev, [key]: value };
                    void saveMetadata(updated);
                    return updated;
                  });
                }
              }}
            />
          ) : (
            <VitalsStrip vitals={buildPersonVitals(detail)} />
          )}
        </div>
      </section>

      {/* Chapter 2: The Dynamic / The Rhythm */}
      <div id={relationship === "internal" ? "the-rhythm" : "the-dynamic"} className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
        <PersonInsightChapter
          detail={detail}
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          feedbackSlot={
            <IntelligenceFeedback
              value={feedback.getFeedback("person_insight")}
              onFeedback={(type) => feedback.submitFeedback("person_insight", type)}
            />
          }
        />
      </div>

      {/* Chapter 3: Their Orbit */}
      <div id="their-orbit" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
        <PersonNetwork
          entities={detail.entities}
          personId={personId}
          onLink={person.handleLinkEntity}
          onUnlink={person.handleUnlinkEntity}
          chapterTitle="Their Orbit"
        />
      </div>

      {/* Chapter 4: Their Network */}
      <div id="their-network" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
        <PersonRelationships
          personId={personId ?? ""}
          network={intelligence?.network}
          relationships={relationships}
          preset={preset ?? undefined}
          chapterTitle="Their Network"
          onRelationshipsChanged={loadRelationships}
        />
      </div>

      {/* Chapter 5: The Landscape */}
      <div id="the-landscape" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
        <WatchList
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          sectionId="the-landscape"
          chapterTitle="The Landscape"
          getItemFeedback={(fieldPath) => feedback.getFeedback(fieldPath)}
          onItemFeedback={(fieldPath, type) => feedback.submitFeedback(fieldPath, type)}
        />
      </div>

      {/* Chapter 6: The Record */}
      <div id="the-record" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
        <UnifiedTimeline
          data={{
            recentMeetings: detail.recentMeetings ?? [],
            recentCaptures: detail.recentCaptures,
            recentEmailSignals: detail.recentEmailSignals,
            contextEntries: entityCtx.entries,
          }}
          sectionId=""
          actionSlot={<AddToRecord onAdd={(title, content) => entityCtx.createEntry(title, content)} />}
        />
      </div>

      {/* Chapter 6: The Work (suppressed when empty per I351) */}
      {(detail.openActions.length > 0 || (detail.upcomingMeetings ?? []).length > 0 || (intelligence?.recommendedActions?.length ?? 0) > 0) && (
        <div id="the-work" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
          {intelligence?.recommendedActions && intelligence.recommendedActions.length > 0 && (
            <RecommendedActions entityId={detail.id} entityType="person"
              actions={intelligence.recommendedActions} onRefresh={person.silentRefresh} />
          )}
          <TheWork
            data={detail}
            addingAction={person.addingAction}
            setAddingAction={person.setAddingAction}
            newActionTitle={person.newActionTitle}
            setNewActionTitle={person.setNewActionTitle}
            creatingAction={person.creatingAction}
            onCreateAction={person.handleCreateAction}
          />
        </div>
      )}

      {/* Appendix */}
      <div className="editorial-reveal">
        <PersonAppendix
          detail={detail}
          duplicateCandidates={person.duplicateCandidates}
          onMergeSuggested={person.handleOpenSuggestedMerge}
          merging={person.merging}
          files={person.files}
          onIndexFiles={person.handleIndexFiles}
          indexing={person.indexing}
          indexFeedback={person.indexFeedback}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={intelligence?.enrichedAt} />
      </div>

      {/* Merge Person Picker Dialog */}
      <Dialog open={person.mergeDialogOpen} onOpenChange={person.setMergeDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Merge {detail.name} into...</DialogTitle>
            <DialogDescription>
              Search for the person to merge into. All meetings, entity links, and actions will transfer to the selected person.
            </DialogDescription>
          </DialogHeader>
          <Input
            placeholder="Search by name or email..."
            value={person.mergeSearchQuery}
            onChange={(e) => person.setMergeSearchQuery(e.target.value)}
            autoFocus
          />
          {person.mergeSearchResults.length > 0 && (
            <div className={shared.mergeSearchResults}>
              {person.mergeSearchResults.map((p) => (
                <button
                  key={p.id}
                  onClick={() => {
                    person.setMergeTarget(p);
                    person.setMergeDialogOpen(false);
                    person.setMergeConfirmOpen(true);
                  }}
                  className={`${shared.mergeSearchButton} hover:bg-muted`}
                >
                  <div className={shared.mergeAvatar}>
                    {p.name.charAt(0).toUpperCase()}
                  </div>
                  <div className={shared.mergePersonInfo}>
                    <div className={shared.mergePersonName}>
                      {p.name}
                    </div>
                    <div className={shared.mergePersonEmail}>
                      {p.email}
                      {p.organization && ` \u00B7 ${p.organization}`}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
          {person.mergeSearchQuery.length >= 2 && person.mergeSearchResults.length === 0 && (
            <p className={shared.mergeEmptyState}>
              No matching people found
            </p>
          )}
        </DialogContent>
      </Dialog>

      {/* Merge Confirmation */}
      <AlertDialog open={person.mergeConfirmOpen} onOpenChange={person.setMergeConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Merge {detail.name}?</AlertDialogTitle>
            <AlertDialogDescription>
              Permanently merge <strong>{detail.name}</strong> ({detail.email}) into{" "}
              <strong>{person.mergeTarget?.name}</strong> ({person.mergeTarget?.email}).
              All meetings, entity links, and actions will transfer. This cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={person.merging}>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={person.handleMerge} disabled={person.merging}>
              {person.merging ? "Merging…" : "Merge"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Delete Confirmation */}
      <AlertDialog open={person.deleteConfirmOpen} onOpenChange={person.setDeleteConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete {detail.name}?</AlertDialogTitle>
            <AlertDialogDescription>
              All meeting attendance records, entity links, and action associations for{" "}
              <strong>{detail.name}</strong> will be removed. This cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={person.merging}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={person.handleDelete}
              disabled={person.merging}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {person.merging ? "Deleting…" : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

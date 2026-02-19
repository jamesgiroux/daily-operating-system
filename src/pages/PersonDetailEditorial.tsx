import { useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import { buildVitalsFromPreset } from "@/lib/preset-vitals";
import { usePersonDetail } from "@/hooks/usePersonDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { smoothScrollTo } from "@/lib/smooth-scroll";
import {
  AlignLeft,
  Zap,
  RefreshCw,
  Network,
  Eye,
  Activity,
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
import { PersonAppendix } from "@/components/person/PersonAppendix";
import { PersonInsightChapter } from "@/components/person/PersonInsightChapter";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { FinisMarker } from "@/components/editorial/FinisMarker";

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
    { id: "the-network", label: "The Network", icon: <Network size={18} strokeWidth={1.5} /> },
    { id: "the-landscape", label: "The Landscape", icon: <Eye size={18} strokeWidth={1.5} /> },
    { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
  ];
}

export default function PersonDetailEditorial() {
  const { personId } = useParams({ strict: false });
  const navigate = useNavigate();
  const person = usePersonDetail(personId);
  const preset = useActivePreset();
  useRevealObserver(!person.loading && !!person.detail);

  const relationship = person.detail?.relationship ?? "unknown";
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Person",
      atmosphereColor: "larkspur" as const,
      activePage: "people" as const,
      backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/people" }) },
      chapters: buildChapters(relationship),
    }),
    [navigate, relationship],
  );
  useRegisterMagazineShell(shellConfig);

  // I352: Shared intelligence field update hook
  const { updateField: handleUpdateIntelField } = useIntelligenceFieldUpdate("person", personId);

  if (person.loading) return <EditorialLoading />;

  if (person.error || !person.detail) {
    return <EditorialError message={person.error ?? "Person not found"} onRetry={person.load} />;
  }

  const { detail, intelligence } = person;

  return (
    <>
      {/* Chapter 1: The Profile (Hero) */}
      <section id="headline" style={{ scrollMarginTop: 60 }}>
        <PersonHero
          detail={detail}
          intelligence={intelligence}
          onEditDetails={() => smoothScrollTo("appendix")}
          onEnrich={person.handleEnrich}
          enriching={person.enriching}
          enrichSeconds={person.enrichSeconds}
          onMerge={person.openMergeDialog}
          onArchive={() => person.handleArchive()}
          onUnarchive={person.handleUnarchive}
          onDelete={() => person.setDeleteConfirmOpen(true)}
        />
        <div className="editorial-reveal">
          <VitalsStrip vitals={preset ? buildVitalsFromPreset(preset.vitals.person, { ...detail, signals: detail.signals as Record<string, unknown> | undefined }) : buildPersonVitals(detail)} />
        </div>
      </section>

      {/* Chapter 2: The Dynamic / The Rhythm */}
      <div id={relationship === "internal" ? "the-rhythm" : "the-dynamic"} className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <PersonInsightChapter detail={detail} intelligence={intelligence} onUpdateField={handleUpdateIntelField} />
      </div>

      {/* Chapter 3: The Network */}
      <div id="the-network" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <PersonNetwork
          entities={detail.entities}
          onLink={person.handleLinkEntity}
          onUnlink={person.handleUnlinkEntity}
          sectionId="the-network"
          chapterTitle="The Network"
        />
      </div>

      {/* Chapter 4: The Landscape */}
      <div id="the-landscape" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <WatchList
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          sectionId="the-landscape"
          chapterTitle="The Landscape"
        />
      </div>

      {/* Chapter 5: The Record */}
      <div id="the-record" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <UnifiedTimeline data={{
          recentMeetings: detail.recentMeetings ?? [],
          recentCaptures: detail.recentCaptures,
          recentEmailSignals: detail.recentEmailSignals,
        }} />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={intelligence?.enrichedAt} />
      </div>

      {/* Appendix */}
      <div className="editorial-reveal">
        <PersonAppendix
          detail={detail}
          editName={person.editName}
          setEditName={(v) => { person.setEditName(v); person.setDirty(true); }}
          editRole={person.editRole}
          setEditRole={(v) => { person.setEditRole(v); person.setDirty(true); }}
          editNotes={person.editNotes}
          setEditNotes={(v) => { person.setEditNotes(v); person.setDirty(true); }}
          onSave={person.handleSave}
          dirty={person.dirty}
          saving={person.saving}
          duplicateCandidates={person.duplicateCandidates}
          onMergeSuggested={person.handleOpenSuggestedMerge}
          merging={person.merging}
          files={person.files}
          onIndexFiles={person.handleIndexFiles}
          indexing={person.indexing}
          indexFeedback={person.indexFeedback}
        />
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
            <div style={{ maxHeight: 240, overflowY: "auto", display: "flex", flexDirection: "column", gap: 4 }}>
              {person.mergeSearchResults.map((p) => (
                <button
                  key={p.id}
                  onClick={() => {
                    person.setMergeTarget(p);
                    person.setMergeDialogOpen(false);
                    person.setMergeConfirmOpen(true);
                  }}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 12,
                    padding: "8px 12px",
                    borderRadius: 6,
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    textAlign: "left",
                    width: "100%",
                  }}
                  className="hover:bg-muted"
                >
                  <div
                    style={{
                      width: 32,
                      height: 32,
                      borderRadius: "50%",
                      background: "rgba(143, 163, 196, 0.15)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      fontWeight: 600,
                      color: "var(--color-garden-larkspur)",
                      flexShrink: 0,
                    }}
                  >
                    {p.name.charAt(0).toUpperCase()}
                  </div>
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {p.name}
                    </div>
                    <div style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {p.email}
                      {p.organization && ` \u00B7 ${p.organization}`}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
          {person.mergeSearchQuery.length >= 2 && person.mergeSearchResults.length === 0 && (
            <p style={{ textAlign: "center", padding: "16px 0", fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-tertiary)" }}>
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

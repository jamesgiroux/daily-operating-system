import React, { useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import { usePersonDetail } from "@/hooks/usePersonDetail";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { smoothScrollTo } from "@/lib/smooth-scroll";
import {
  AlignLeft,
  Clock,
  Network,
  Eye,
  Activity,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
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
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
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
    const trendArrow =
      sig.trend === "increasing" ? " \u2191" : sig.trend === "decreasing" ? " \u2193" : "";
    vitals.push({ text: `${sig.meetingFrequency30d} meetings / 30d${trendArrow}` });
    if (sig.meetingFrequency90d > 0) {
      vitals.push({ text: `${sig.meetingFrequency90d} meetings / 90d` });
    }
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
    if (sig.lastMeeting) {
      vitals.push({ text: `Last: ${formatShortDate(sig.lastMeeting)}` });
    }
  }

  if (detail.meetingCount > 0) {
    vitals.push({ text: `${detail.meetingCount} total meetings` });
  }

  return vitals;
}

/* ── Chapters ── */

const CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Profile", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "state-of-play", label: "State of Play", icon: <Clock size={18} strokeWidth={1.5} /> },
  { id: "the-network", label: "The Network", icon: <Network size={18} strokeWidth={1.5} /> },
  { id: "watch-list", label: "Watch List", icon: <Eye size={18} strokeWidth={1.5} /> },
  { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
];

export default function PersonDetailEditorial() {
  const { personId } = useParams({ strict: false });
  const navigate = useNavigate();
  const person = usePersonDetail(personId);
  useRevealObserver(!person.loading && !!person.detail);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Person",
      atmosphereColor: "larkspur" as const,
      activePage: "people" as const,
      backLink: { label: "People", onClick: () => navigate({ to: "/people" }) },
      chapters: CHAPTERS,
    }),
    [navigate],
  );
  useRegisterMagazineShell(shellConfig);

  if (person.loading) {
    return (
      <div className="editorial-loading" style={{ padding: "120px 120px 80px" }}>
        <Skeleton className="mb-4 h-4 w-24" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton className="mb-2 h-12 w-96" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton className="mb-8 h-5 w-full max-w-2xl" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton className="h-px w-full" style={{ background: "var(--color-rule-heavy)" }} />
        <div style={{ marginTop: 48, display: "flex", flexDirection: "column", gap: 32 }}>
          <Skeleton className="h-32 w-full" style={{ background: "var(--color-rule-light)" }} />
          <Skeleton className="h-24 w-full" style={{ background: "var(--color-rule-light)" }} />
        </div>
      </div>
    );
  }

  if (person.error || !person.detail) {
    return (
      <div style={{ padding: "120px 120px 80px", textAlign: "center" }}>
        <p style={{ fontFamily: "var(--font-serif)", fontSize: 24, color: "var(--color-text-primary)", marginBottom: 16 }}>
          Something went wrong
        </p>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)", marginBottom: 24 }}>
          {person.error ?? "Person not found"}
        </p>
        <Button onClick={person.load} variant="outline">Try again</Button>
      </div>
    );
  }

  const { detail, intelligence } = person;

  // Build timeline data — PersonDetail has optional recentMeetings, no emails/captures
  const timelineData = {
    recentMeetings: detail.recentMeetings ?? [],
  };

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
          <VitalsStrip vitals={buildPersonVitals(detail)} />
        </div>
      </section>

      {/* Chapter 2: State of Play */}
      <div className="editorial-reveal">
        <StateOfPlay intelligence={intelligence} />
      </div>

      {/* Chapter 3: The Network */}
      <div className="editorial-reveal">
        <PersonNetwork
          entities={detail.entities}
          onLink={person.handleLinkEntity}
          onUnlink={person.handleUnlinkEntity}
        />
      </div>

      {/* Chapter 4: Watch List */}
      <div className="editorial-reveal">
        <WatchList intelligence={intelligence} />
      </div>

      {/* Chapter 5: The Record */}
      <div className="editorial-reveal">
        <UnifiedTimeline data={timelineData} />
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
              {person.merging ? "Merging\u2026" : "Merge"}
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
              {person.merging ? "Deleting\u2026" : "Delete"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

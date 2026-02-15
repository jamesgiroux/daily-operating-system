import React, { useState } from "react";
import { useParams } from "@tanstack/react-router";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import {
  AlignLeft,
  Clock,
  Users,
  Eye,
  Activity,
  CheckSquare2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
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
import { AccountHero } from "@/components/account/AccountHero";
import { VitalsStrip } from "@/components/account/VitalsStrip";
import { StateOfPlay } from "@/components/account/StateOfPlay";
import { StakeholderGallery } from "@/components/account/StakeholderGallery";
import { WatchList } from "@/components/account/WatchList";
import { UnifiedTimeline } from "@/components/account/UnifiedTimeline";
import { TheWork } from "@/components/account/TheWork";
import { AccountAppendix } from "@/components/account/AccountAppendix";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { AccountFieldsDrawer } from "@/components/account/AccountFieldsDrawer";
import { TeamManagementDrawer } from "@/components/account/TeamManagementDrawer";
import { LifecycleEventDrawer } from "@/components/account/LifecycleEventDrawer";

// Chapter definitions for the editorial layout — icons match the v3 mockup nav island
const CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Headline", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "state-of-play", label: "State of Play", icon: <Clock size={18} strokeWidth={1.5} /> },
  { id: "the-room", label: "The Room", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "watch-list", label: "Watch List", icon: <Eye size={18} strokeWidth={1.5} /> },
  { id: "the-record", label: "The Record", icon: <Activity size={18} strokeWidth={1.5} /> },
  { id: "the-work", label: "The Work", icon: <CheckSquare2 size={18} strokeWidth={1.5} /> },
];

export default function AccountDetailEditorial() {
  const { accountId } = useParams({ strict: false });
  const acct = useAccountDetail(accountId);
  useRevealObserver(!acct.loading && !!acct.detail);

  // Drawer/dialog open state
  const [fieldsDrawerOpen, setFieldsDrawerOpen] = useState(false);
  const [teamDrawerOpen, setTeamDrawerOpen] = useState(false);
  const [eventDrawerOpen, setEventDrawerOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  if (acct.loading) {
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

  if (acct.error || !acct.detail) {
    return (
      <div style={{ padding: "120px 120px 80px", textAlign: "center" }}>
        <p style={{ fontFamily: "var(--font-serif)", fontSize: 24, color: "var(--color-text-primary)", marginBottom: 16 }}>
          Something went wrong
        </p>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)", marginBottom: 24 }}>
          {acct.error ?? "Account not found"}
        </p>
        <Button onClick={acct.load} variant="outline">Try again</Button>
      </div>
    );
  }

  const { detail, intelligence, events, files, programs } = acct;

  // Notes dirty tracking (compare editNotes to saved detail.notes)
  const notesDirty = acct.editNotes !== (detail.notes ?? "");

  return (
    <>
      {/* Chapter 1: The Headline (Hero) — no reveal, visible immediately */}
      <section id="headline" style={{ scrollMarginTop: 60 }}>
        <AccountHero
          detail={detail}
          intelligence={intelligence}
          onEditFields={() => setFieldsDrawerOpen(true)}
          onManageTeam={() => setTeamDrawerOpen(true)}
          onEnrich={acct.handleEnrich}
          enriching={acct.enriching}
          enrichSeconds={acct.enrichSeconds}
          onArchive={() => setArchiveDialogOpen(true)}
          onUnarchive={acct.handleUnarchive}
        />
        <div className="editorial-reveal">
          <VitalsStrip detail={detail} />
        </div>
      </section>

      {/* Chapter 2: State of Play */}
      <div className="editorial-reveal">
        <StateOfPlay intelligence={intelligence} />
      </div>

      {/* Chapter 3: The Room */}
      <div className="editorial-reveal">
        <StakeholderGallery
          intelligence={intelligence}
          linkedPeople={detail.linkedPeople}
          accountTeam={detail.accountTeam}
        />
      </div>

      {/* Chapter 4: Watch List (full-bleed linen band) */}
      <div className="editorial-reveal">
        <WatchList
          intelligence={intelligence}
          programs={programs}
          onProgramUpdate={acct.handleProgramUpdate}
          onProgramDelete={acct.handleProgramDelete}
          onAddProgram={acct.handleAddProgram}
        />
      </div>

      {/* Chapter 5: The Record */}
      <div className="editorial-reveal">
        <UnifiedTimeline detail={detail} />
      </div>

      {/* Chapter 6: The Work */}
      <div className="editorial-reveal">
        <TheWork
          detail={detail}
          intelligence={intelligence}
          addingAction={acct.addingAction}
          setAddingAction={acct.setAddingAction}
          newActionTitle={acct.newActionTitle}
          setNewActionTitle={acct.setNewActionTitle}
          creatingAction={acct.creatingAction}
          onCreateAction={acct.handleCreateAction}
        />
      </div>

      {/* Finis marker — inside The Work per mockup */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={intelligence?.enrichedAt} />
      </div>

      {/* Chapter 7: Appendix */}
      <div className="editorial-reveal">
        <AccountAppendix
          detail={detail}
          intelligence={intelligence}
          events={events}
          files={files}
          editNotes={acct.editNotes}
          setEditNotes={(v) => {
            acct.setEditNotes(v);
            acct.setDirty(true);
          }}
          onSaveNotes={acct.handleSave}
          notesDirty={notesDirty}
          onRecordEvent={() => setEventDrawerOpen(true)}
          onIndexFiles={acct.handleIndexFiles}
          indexing={acct.indexing}
          indexFeedback={acct.indexFeedback}
          onCreateChild={() => acct.setCreateChildOpen(true)}
        />
      </div>

      {/* ─── Drawers ─── */}

      <AccountFieldsDrawer
        open={fieldsDrawerOpen}
        onOpenChange={setFieldsDrawerOpen}
        editName={acct.editName}
        setEditName={acct.setEditName}
        editHealth={acct.editHealth}
        setEditHealth={acct.setEditHealth}
        editLifecycle={acct.editLifecycle}
        setEditLifecycle={acct.setEditLifecycle}
        editArr={acct.editArr}
        setEditArr={acct.setEditArr}
        editNps={acct.editNps}
        setEditNps={acct.setEditNps}
        editRenewal={acct.editRenewal}
        setEditRenewal={acct.setEditRenewal}
        setDirty={acct.setDirty}
        onSave={acct.handleSave}
        onCancel={acct.handleCancelEdit}
        saving={acct.saving}
        dirty={acct.dirty}
      />

      <TeamManagementDrawer
        open={teamDrawerOpen}
        onOpenChange={setTeamDrawerOpen}
        accountTeam={detail.accountTeam}
        accountTeamImportNotes={detail.accountTeamImportNotes}
        teamSearchQuery={acct.teamSearchQuery}
        setTeamSearchQuery={acct.setTeamSearchQuery}
        teamSearchResults={acct.teamSearchResults}
        selectedTeamPerson={acct.selectedTeamPerson}
        setSelectedTeamPerson={acct.setSelectedTeamPerson}
        teamRole={acct.teamRole}
        setTeamRole={acct.setTeamRole}
        teamInlineName={acct.teamInlineName}
        setTeamInlineName={acct.setTeamInlineName}
        teamInlineEmail={acct.teamInlineEmail}
        setTeamInlineEmail={acct.setTeamInlineEmail}
        teamInlineRole={acct.teamInlineRole}
        setTeamInlineRole={acct.setTeamInlineRole}
        teamWorking={acct.teamWorking}
        resolvedImportNotes={acct.resolvedImportNotes}
        teamError={acct.teamError}
        handleAddExistingTeamMember={acct.handleAddExistingTeamMember}
        handleRemoveTeamMember={acct.handleRemoveTeamMember}
        handleCreateInlineTeamMember={acct.handleCreateInlineTeamMember}
        handleImportNoteCreateAndAdd={acct.handleImportNoteCreateAndAdd}
      />

      <LifecycleEventDrawer
        open={eventDrawerOpen}
        onOpenChange={setEventDrawerOpen}
        newEventType={acct.newEventType}
        setNewEventType={acct.setNewEventType}
        newEventDate={acct.newEventDate}
        setNewEventDate={acct.setNewEventDate}
        newArrImpact={acct.newArrImpact}
        setNewArrImpact={acct.setNewArrImpact}
        newEventNotes={acct.newEventNotes}
        setNewEventNotes={acct.setNewEventNotes}
        onSave={acct.handleRecordEvent}
      />

      {/* ─── Archive Confirmation ─── */}
      <AlertDialog open={archiveDialogOpen} onOpenChange={setArchiveDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Archive Account</AlertDialogTitle>
            <AlertDialogDescription>
              This will hide {detail.name} from active views.
              You can unarchive it later.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={acct.handleArchive}>Archive</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* ─── Child Account Creation ─── */}
      <Dialog open={acct.createChildOpen} onOpenChange={acct.setCreateChildOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {detail.isInternal ? "Create Team" : "Create Business Unit"}
            </DialogTitle>
            <DialogDescription>
              Create a new {detail.isInternal ? "team" : "business unit"} under {detail.name}.
            </DialogDescription>
          </DialogHeader>
          <div style={{ display: "flex", flexDirection: "column", gap: 12, marginTop: 8 }}>
            <Input
              value={acct.childName}
              onChange={(e) => acct.setChildName(e.target.value)}
              placeholder="Name"
            />
            <Input
              value={acct.childDescription}
              onChange={(e) => acct.setChildDescription(e.target.value)}
              placeholder="Description (optional)"
            />
            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 8 }}>
              <Button
                variant="ghost"
                onClick={() => acct.setCreateChildOpen(false)}
                style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
              >
                Cancel
              </Button>
              <Button
                onClick={acct.handleCreateChild}
                disabled={acct.creatingChild || !acct.childName.trim()}
                style={{ fontFamily: "var(--font-sans)", fontSize: 13 }}
              >
                {acct.creatingChild ? "Creating…" : "Create"}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

// Re-export chapters for use by the router shell
export { CHAPTERS };

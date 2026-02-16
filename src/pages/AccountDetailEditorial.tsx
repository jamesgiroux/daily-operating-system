import React, { useState, useMemo, useCallback } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { formatArr, formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
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
import { AccountHero } from "@/components/account/AccountHero";
import { AccountAppendix } from "@/components/account/AccountAppendix";
import { WatchListPrograms } from "@/components/account/WatchListPrograms";
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { AccountFieldsDrawer } from "@/components/account/AccountFieldsDrawer";
import { TeamManagementDrawer } from "@/components/account/TeamManagementDrawer";
import { LifecycleEventDrawer } from "@/components/account/LifecycleEventDrawer";

/* ── Vitals assembly (moved from old account/VitalsStrip) ── */

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round(
      (renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24),
    );
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    return `Renewal in ${diffDays}d`;
  } catch {
    return dateStr;
  }
}

const healthColorMap: Record<string, "saffron" | undefined> = {
  yellow: "saffron",
};

function buildAccountVitals(detail: {
  arr?: number | null;
  health?: string;
  lifecycle?: string;
  renewalDate?: string;
  nps?: number | null;
  signals?: { meetingFrequency30d?: number };
  contractStart?: string;
}): VitalDisplay[] {
  const vitals: VitalDisplay[] = [];
  if (detail.arr != null) {
    vitals.push({ text: `$${formatArr(detail.arr)} ARR`, highlight: "turmeric" });
  }
  if (detail.health) {
    vitals.push({
      text: `${detail.health.charAt(0).toUpperCase() + detail.health.slice(1)} Health`,
      highlight: healthColorMap[detail.health],
    });
  }
  if (detail.lifecycle) vitals.push({ text: detail.lifecycle });
  if (detail.renewalDate) vitals.push({ text: formatRenewalCountdown(detail.renewalDate) });
  if (detail.nps != null) vitals.push({ text: `NPS ${detail.nps}` });
  if (detail.signals?.meetingFrequency30d != null) {
    vitals.push({ text: `${detail.signals.meetingFrequency30d} meetings / 30d` });
  }
  if (detail.contractStart) {
    vitals.push({ text: `Contract: ${formatShortDate(detail.contractStart)}` });
  }
  return vitals;
}

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
  const navigate = useNavigate();
  const acct = useAccountDetail(accountId);
  useRevealObserver(!acct.loading && !!acct.detail);

  // Register magazine shell configuration — MagazinePageLayout consumes this
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Account",
      atmosphereColor: "turmeric" as const,
      activePage: "accounts" as const,
      backLink: { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
      chapters: CHAPTERS,
      folioActions: (
        <button
          onClick={() =>
            navigate({
              to: "/accounts/$accountId/risk-briefing",
              params: { accountId: accountId! },
            })
          }
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase" as const,
            color: "var(--color-spice-turmeric)",
            background: "none",
            border: "1px solid var(--color-spice-turmeric)",
            borderRadius: 4,
            padding: "2px 10px",
            cursor: "pointer",
          }}
        >
          Reports
        </button>
      ),
    }),
    [navigate, accountId],
  );
  useRegisterMagazineShell(shellConfig);

  // Drawer/dialog open state
  const [fieldsDrawerOpen, setFieldsDrawerOpen] = useState(false);
  const [teamDrawerOpen, setTeamDrawerOpen] = useState(false);
  const [eventDrawerOpen, setEventDrawerOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);

  // Intelligence field update callback (I261)
  const handleUpdateIntelField = useCallback(
    async (fieldPath: string, value: string) => {
      if (!accountId) return;
      try {
        await invoke("update_intelligence_field", {
          entityId: accountId,
          entityType: "account",
          fieldPath,
          value,
        });
        acct.load(); // Re-fetch to reflect changes
      } catch (e) {
        console.error("Failed to update intelligence field:", e);
      }
    },
    [accountId, acct],
  );

  if (acct.loading) return <EditorialLoading />;

  if (acct.error || !acct.detail) {
    return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;
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
          <VitalsStrip vitals={buildAccountVitals(detail)} />
        </div>
      </section>

      {/* Chapter 2: State of Play */}
      <div className="editorial-reveal">
        <StateOfPlay intelligence={intelligence} onUpdateField={handleUpdateIntelField} />
      </div>

      {/* Chapter 3: The Room */}
      <div className="editorial-reveal">
        <StakeholderGallery
          intelligence={intelligence}
          linkedPeople={detail.linkedPeople}
          accountTeam={detail.accountTeam}
          entityId={accountId}
          entityType="account"
          onIntelligenceUpdated={acct.load}
        />
      </div>

      {/* Chapter 4: Watch List (full-bleed linen band) */}
      <div className="editorial-reveal">
        <WatchList
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
          bottomSection={
            <WatchListPrograms
              programs={programs}
              onProgramUpdate={acct.handleProgramUpdate}
              onProgramDelete={acct.handleProgramDelete}
              onAddProgram={acct.handleAddProgram}
            />
          }
        />
      </div>

      {/* Chapter 5: The Record */}
      <div className="editorial-reveal">
        <UnifiedTimeline data={detail} />
      </div>

      {/* Chapter 6: The Work */}
      <div className="editorial-reveal">
        <TheWork
          data={detail}
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

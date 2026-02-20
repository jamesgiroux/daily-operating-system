import React, { useState, useEffect, useMemo } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { formatArr, formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import { buildVitalsFromPreset } from "@/lib/preset-vitals";
import { useAccountDetail } from "@/hooks/useAccountDetail";
import { useActivePreset } from "@/hooks/useActivePreset";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
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
import { VitalsStrip } from "@/components/entity/VitalsStrip";
import { StateOfPlay } from "@/components/entity/StateOfPlay";
import { StakeholderGallery } from "@/components/entity/StakeholderGallery";
import { WatchList } from "@/components/entity/WatchList";
import { UnifiedTimeline } from "@/components/entity/UnifiedTimeline";
import { TheWork } from "@/components/entity/TheWork";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { TeamManagementDrawer } from "@/components/account/TeamManagementDrawer";
import { LifecycleEventDrawer } from "@/components/account/LifecycleEventDrawer";
import { AccountMergeDialog } from "@/components/account/AccountMergeDialog";

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
  if (detail.renewalDate) {
    const renewal = new Date(detail.renewalDate);
    const now = new Date();
    const diffDays = Math.round((renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
    vitals.push({
      text: formatRenewalCountdown(detail.renewalDate),
      highlight: diffDays <= 60 ? "saffron" : undefined,
    });
  }
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
  const preset = useActivePreset();
  useRevealObserver(!acct.loading && !!acct.detail);

  // Register magazine shell configuration — MagazinePageLayout consumes this
  const shellConfig = useMemo(
    () => ({
      folioLabel: acct.detail?.isInternal ? "Internal" : "Account",
      atmosphereColor: acct.detail?.isInternal ? "larkspur" as const : "turmeric" as const,
      activePage: "accounts" as const,
      backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/accounts" }) },
      chapters: CHAPTERS,
      folioActions: (
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          {acct.detail && (
            <button
              onClick={() => acct.setCreateChildOpen(true)}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase" as const,
                color: "var(--color-garden-eucalyptus)",
                background: "none",
                border: "1px solid var(--color-garden-eucalyptus)",
                borderRadius: 4,
                padding: "2px 10px",
                cursor: "pointer",
              }}
            >
              + Business Unit
            </button>
          )}
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
        </div>
      ),
    }),
    [navigate, accountId, acct.detail, acct.setCreateChildOpen],
  );
  useRegisterMagazineShell(shellConfig);

  // Drawer/dialog open state
  const [teamDrawerOpen, setTeamDrawerOpen] = useState(false);
  const [eventDrawerOpen, setEventDrawerOpen] = useState(false);
  const [mergeDialogOpen, setMergeDialogOpen] = useState(false);
  const [archiveDialogOpen, setArchiveDialogOpen] = useState(false);
  const [rolloverDismissed, setRolloverDismissed] = useState(false);

  // I312: Preset metadata state
  const [metadataValues, setMetadataValues] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!accountId) return;
    invoke<string>("get_entity_metadata", { entityType: "account", entityId: accountId })
      .then((json) => {
        try { setMetadataValues(JSON.parse(json) ?? {}); } catch { setMetadataValues({}); }
      })
      .catch(() => setMetadataValues({}));
  }, [accountId]);

  // I316: Fetch ancestor accounts for breadcrumb navigation
  const [ancestors, setAncestors] = useState<{ id: string; name: string }[]>([]);
  useEffect(() => {
    if (!accountId) return;
    invoke<{ id: string; name: string }[]>("get_account_ancestors", { accountId })
      .then(setAncestors)
      .catch(() => setAncestors([]));
  }, [accountId]);

  // I352: Shared intelligence field update hook
  const { updateField: handleUpdateIntelField } = useIntelligenceFieldUpdate("account", accountId);

  if (acct.loading) return <EditorialLoading />;

  if (acct.error || !acct.detail) {
    return <EditorialError message={acct.error ?? "Account not found"} onRetry={acct.load} />;
  }

  const { detail, intelligence, events, files } = acct;

  // Notes dirty tracking (compare editNotes to saved detail.notes)
  const notesDirty = acct.editNotes !== (detail.notes ?? "");

  return (
    <>
      {/* I316: Ancestor breadcrumbs for nested accounts */}
      {ancestors.length > 0 && (
        <nav
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            letterSpacing: "0.04em",
            color: "var(--color-text-tertiary)",
            padding: "8px 0 4px",
            display: "flex",
            alignItems: "center",
            gap: 4,
            flexWrap: "wrap",
          }}
        >
          <button
            onClick={() => navigate({ to: "/accounts" })}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
              color: "var(--color-text-tertiary)",
              fontFamily: "inherit",
              fontSize: "inherit",
              letterSpacing: "inherit",
            }}
          >
            Accounts
          </button>
          {ancestors.map((anc) => (
            <React.Fragment key={anc.id}>
              <span style={{ color: "var(--color-text-tertiary)", opacity: 0.5 }}>/</span>
              <button
                onClick={() =>
                  navigate({
                    to: "/accounts/$accountId",
                    params: { accountId: anc.id },
                  })
                }
                style={{
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  padding: 0,
                  color: "var(--color-spice-turmeric)",
                  fontFamily: "inherit",
                  fontSize: "inherit",
                  letterSpacing: "inherit",
                }}
              >
                {anc.name}
              </button>
            </React.Fragment>
          ))}
          <span style={{ color: "var(--color-text-tertiary)", opacity: 0.5 }}>/</span>
          <span style={{ color: "var(--color-text-primary)" }}>{detail?.name ?? ""}</span>
        </nav>
      )}

      {/* Chapter 1: The Headline (Hero) — no reveal, visible immediately */}
      <section id="headline" style={{ scrollMarginTop: 60 }}>
        <AccountHero
          detail={detail}
          intelligence={intelligence}
          editName={acct.editName}
          setEditName={(v) => { acct.setEditName(v); acct.setDirty(true); }}
          editHealth={acct.editHealth}
          setEditHealth={(v) => { acct.setEditHealth(v); acct.setDirty(true); }}
          editLifecycle={acct.editLifecycle}
          setEditLifecycle={(v) => { acct.setEditLifecycle(v); acct.setDirty(true); }}
          onSave={acct.handleSave}
          onManageTeam={() => setTeamDrawerOpen(true)}
          onEnrich={acct.handleEnrich}
          enriching={acct.enriching}
          enrichSeconds={acct.enrichSeconds}
          onArchive={() => setArchiveDialogOpen(true)}
          onUnarchive={acct.handleUnarchive}
        />
        <div className="editorial-reveal">
          {!detail.isInternal && (
            <VitalsStrip vitals={preset ? buildVitalsFromPreset(preset.vitals.account, { ...detail, metadata: metadataValues }) : buildAccountVitals(detail)} />
          )}
        </div>
        {/* Auto-rollover prompt for past renewal dates */}
        {detail.renewalDate && new Date(detail.renewalDate) < new Date() && !rolloverDismissed && (
          <div
            style={{
              margin: "24px 0",
              padding: "16px 20px",
              background: "rgba(222, 184, 65, 0.08)",
              borderLeft: "3px solid var(--color-spice-saffron)",
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-primary)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: 16,
            }}
          >
            <span>Renewal date has passed — what happened?</span>
            <div style={{ display: "flex", gap: 8 }}>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  acct.setNewEventType("renewal");
                  acct.setNewEventDate(detail.renewalDate!);
                  setEventDrawerOpen(true);
                }}
                style={{ fontFamily: "var(--font-sans)", fontSize: 12 }}
              >
                Renewed
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  acct.setNewEventType("churn");
                  acct.setNewEventDate(detail.renewalDate!);
                  setEventDrawerOpen(true);
                }}
                style={{ fontFamily: "var(--font-sans)", fontSize: 12 }}
              >
                Churned
              </Button>
              <button
                onClick={() => setRolloverDismissed(true)}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-text-tertiary)",
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                }}
              >
                Dismiss
              </button>
            </div>
          </div>
        )}
      </section>

      {/* Chapter 2: State of Play */}
      <div id="state-of-play" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <StateOfPlay intelligence={intelligence} onUpdateField={handleUpdateIntelField} />
      </div>

      {/* Chapter 3: The Room */}
      <div id="the-room" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <StakeholderGallery
          intelligence={intelligence}
          linkedPeople={detail.linkedPeople}
          accountTeam={detail.accountTeam}
          entityId={accountId}
          entityType="account"
          onIntelligenceUpdated={acct.silentRefresh}
        />
      </div>

      {/* Chapter 4: Watch List (full-bleed linen band) */}
      <div id="watch-list" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <WatchList
          intelligence={intelligence}
          onUpdateField={handleUpdateIntelField}
        />
      </div>

      {/* Chapter 5: The Record */}
      <div id="the-record" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
        <UnifiedTimeline data={{ ...detail, accountEvents: events }} />
      </div>

      {/* Chapter 6: The Work */}
      <div id="the-work" className="editorial-reveal" style={{ scrollMarginTop: 60 }}>
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
          onMerge={() => setMergeDialogOpen(true)}
        />
      </div>

      {/* ─── Drawers ─── */}

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

      <AccountMergeDialog
        open={mergeDialogOpen}
        onOpenChange={setMergeDialogOpen}
        sourceAccountId={accountId!}
        sourceAccountName={detail.name}
        onMerged={() => navigate({ to: "/accounts" })}
      />
    </>
  );
}

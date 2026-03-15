import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useParams, Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  AgendaDraftDialog,
  useAgendaDraft,
} from "@/components/ui/agenda-draft-dialog";
import type {
  FullMeetingPrep,
  Stakeholder,
  StakeholderSignals,
  AttendeeContext,
  AccountSnapshotItem,
  MeetingOutcomeData,
  MeetingIntelligence,
  MeetingPostIntelligence,
  CalendarEvent,
  StakeholderInsight,
  ApplyPrepPrefillResult,
  LinkedEntity,
} from "@/types";
import { parseDate, formatRelativeDateLong, stripHtml } from "@/lib/utils";
import { getPrimaryEntityName } from "@/lib/entity-helpers";
import { MeetingEntityChips } from "@/components/ui/meeting-entity-chips";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { ActionRow } from "@/components/shared/ActionRow";
import { HealthBadge } from "@/components/shared/HealthBadge";

import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { PostMeetingIntelligence } from "@/components/meeting/PostMeetingIntelligence";
import { EditableText } from "@/components/ui/EditableText";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import {
  AlignLeft,
  AlertTriangle,
  Check,
  ChevronRight,
  CircleDot,
  Clock,
  Copy,
  Loader2,
  Mail,
  Paperclip,
  RefreshCw,
  Target,
  Trophy,
  Users,
} from "lucide-react";
import clsx from "clsx";
import styles from "./meeting-intel.module.css";

// ── Chapter Nav definitions ──

const CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Brief", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "risks", label: "Risks", icon: <AlertTriangle size={18} strokeWidth={1.5} /> },
  { id: "the-room", label: "The Room", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "your-plan", label: "Your Plan", icon: <Target size={18} strokeWidth={1.5} /> },
];

// ── Unified attendee type ──

interface UnifiedAttendee {
  name: string;
  personId?: string;
  role?: string;
  organization?: string;
  temperature?: string;
  engagement?: string;
  assessment?: string;
  /** Field path in prep_frozen_json for inline editing (I548) */
  assessmentFieldPath?: string;
  meetingCount?: number;
  lastSeen?: string;
  notes?: string;
}

interface MeetingBriefingRefreshProgress {
  meetingId: string;
  stage: string;
  message: string;
  entityId?: string;
  entityType?: string;
  entityName?: string;
  current?: number;
  total?: number;
}

interface MeetingBriefingRefreshResult {
  meetingId: string;
  refreshedEntities: number;
  failedEntities: number;
  failedEntityIds: string[];
  prepRebuiltSync: boolean;
  prepQueued: boolean;
}

interface PrepReadyPayload {
  meetingId: string;
}

type TranscriptProcessedPayload = MeetingOutcomeData | string;

export default function MeetingDetailPage() {
  const { meetingId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [data, setData] = useState<FullMeetingPrep | null>(null);
  const [outcomes, setOutcomes] = useState<MeetingOutcomeData | null>(null);
  const [canEditUserLayer, setCanEditUserLayer] = useState(false);
  const [meetingMeta, setMeetingMeta] = useState<MeetingIntelligence["meeting"] | null>(null);
  const [linkedEntities, setLinkedEntities] = useState<LinkedEntity[]>([]);
  const [entityHealthMap, setEntityHealthMap] = useState<MeetingIntelligence["entityHealthMap"]>({});
  const [intelligenceQuality, setIntelligenceQuality] = useState<MeetingIntelligence["intelligenceQuality"]>();
  const [postIntel, setPostIntel] = useState<MeetingPostIntelligence | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshingIntel, setRefreshingIntel] = useState(false);
  const transientRetryCount = useRef(0);

  // I529: Intelligence quality feedback
  const feedback = useIntelligenceFeedback(meetingId ?? undefined, "meeting");

  // Entity mutation in progress — shows "Updating briefing..." (I477)
  const [briefingUpdating, setBriefingUpdating] = useState(false);

  // Transcript attach
  const [attaching, setAttaching] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [retryingExtraction, setRetryingExtraction] = useState(false);
  const draft = useAgendaDraft({ onError: setError });
  const [prefillNotice, setPrefillNotice] = useState(false);
  const [prefilling, setPrefilling] = useState(false);

  // Save status for folio bar
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");

  // Clipboard copy indicator for collaboration actions
  const [copiedAction, setCopiedAction] = useState<string | null>(null);

  // Connector state — determines which transcript actions are available
  const [quillEnabled, setQuillEnabled] = useState(false);
  const [granolaEnabled, setGranolaEnabled] = useState(false);

  // Persisted user overrides
  const [dismissedTopics, setDismissedTopics] = useState<string[]>([]);
  const [hiddenAttendees, setHiddenAttendees] = useState<string[]>([]);

  const loadMeetingIntelligence = useCallback(async () => {
    if (!meetingId) {
      setError("No meeting ID specified");
      setLoading(false);
      return;
    }
    try {
      // Only show loading skeleton on initial load — subsequent reloads
      // keep existing content visible to preserve scroll position.
      if (!data) setLoading(true);
      setError(null);
      const intel = await invoke<MeetingIntelligence>("get_meeting_intelligence", {
        meetingId,
      });
      setMeetingMeta(intel.meeting);
      setOutcomes(intel.outcomes ?? null);
      setCanEditUserLayer(intel.canEditUserLayer);
      setLinkedEntities(intel.linkedEntities ?? []);
      setEntityHealthMap(intel.entityHealthMap ?? {});
      setIntelligenceQuality(intel.intelligenceQuality);
      const formatRange = (startRaw?: string, endRaw?: string) => {
        if (!startRaw) return "";
        const start = parseDate(startRaw);
        if (!start) return startRaw;
        const startLabel = start.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
        if (!endRaw) return startLabel;
        const end = parseDate(endRaw);
        if (!end) return startLabel;
        const endLabel = end.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
        return `${startLabel} - ${endLabel}`;
      };
      const basePrep: FullMeetingPrep = intel.prep ?? {
        filePath: "",
        calendarEventId: intel.meeting.calendarEventId,
        title: intel.meeting.title,
        timeRange: formatRange(intel.meeting.startTime, intel.meeting.endTime),
      };
      setDismissedTopics(intel.dismissedTopics ?? []);
      setHiddenAttendees(intel.hiddenAttendees ?? []);
      setData({
        ...basePrep,
        userAgenda: intel.userAgenda ?? basePrep.userAgenda,
        userNotes: intel.userNotes ?? basePrep.userNotes,
      });

      // Fetch post-meeting intelligence (non-blocking — only relevant for past meetings with transcripts)
      if (intel.outcomes || intel.transcriptProcessedAt) {
        invoke<MeetingPostIntelligence>("get_meeting_post_intelligence", { meetingId })
          .then((pi) => {
            const hasPostIntelData =
              pi.interactionDynamics != null ||
              pi.championHealth != null ||
              pi.roleChanges.length > 0 ||
              pi.enrichedCaptures.length > 0;
            setPostIntel(hasPostIntelData ? pi : null);
          })
          .catch(() => {
            // Non-critical — silently fail
            setPostIntel(null);
          });
      } else {
        setPostIntel(null);
      }

      transientRetryCount.current = 0;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      const isTransientDbError =
        message.includes("Database not initialized")
        || message.includes("DbService not initialized")
        || message.includes("Database unavailable");

      if (isTransientDbError && transientRetryCount.current < 3) {
        transientRetryCount.current += 1;
        setTimeout(() => {
          void loadMeetingIntelligence();
        }, 600);
        return;
      }

      setError(message || "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  const handleSyncTranscript = useCallback(async () => {
    if (!meetingId) return;
    setSyncing(true);
    try {
      const result = await invoke<string>("trigger_quill_sync_for_meeting", {
        meetingId,
        force: true,
      });
      if (result === "already_in_progress") {
        toast.success("Sync already in progress");
      } else if (result === "resyncing") {
        toast.success("Re-syncing transcript");
      } else {
        toast.success("Transcript sync started");
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Sync failed");
    } finally {
      setSyncing(false);
    }
  }, [meetingId]);

  const handleAttachTranscript = useCallback(async () => {
    if (!meetingId || !data) return;

    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Transcripts",
          extensions: ["md", "txt", "vtt", "srt", "docx", "pdf"],
        },
      ],
    });
    if (!selected) return;

    setAttaching(true);
    try {
      const calendarEvent: CalendarEvent = {
        id: meetingMeta?.id || meetingId,
        title: meetingMeta?.title || data.title,
        start: meetingMeta?.startTime || new Date().toISOString(),
        end:
          meetingMeta?.endTime ||
          meetingMeta?.startTime ||
          new Date().toISOString(),
        type:
          (meetingMeta?.meetingType as CalendarEvent["type"]) ?? "internal",
        attendees: [],
        isAllDay: false,
      };
      const result = await invoke<{
        status: string;
        message?: string;
        summary?: string;
      }>("attach_meeting_transcript", {
        filePath: selected,
        meeting: calendarEvent,
      });

      if (result.status !== "success") {
        toast.error("Transcript processing failed", {
          description: result.message || result.status,
        });
      } else if (!result.summary) {
        toast.warning("No outcomes extracted", {
          description: result.message || "AI extraction returned empty",
        });
      } else {
        toast.success("Transcript processed");
      }
      await loadMeetingIntelligence();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("Failed to attach transcript:", msg);
      toast.error("Failed to attach transcript", { description: msg });
    } finally {
      setAttaching(false);
    }
  }, [meetingId, data, meetingMeta, loadMeetingIntelligence]);

  const handleRetryTranscriptExtraction = useCallback(async () => {
    if (!meetingId || !data || !outcomes?.transcriptPath) return;

    setRetryingExtraction(true);
    try {
      const calendarEvent: CalendarEvent = {
        id: meetingMeta?.id || meetingId,
        title: meetingMeta?.title || data.title,
        start: meetingMeta?.startTime || new Date().toISOString(),
        end:
          meetingMeta?.endTime ||
          meetingMeta?.startTime ||
          new Date().toISOString(),
        type:
          (meetingMeta?.meetingType as CalendarEvent["type"]) ?? "internal",
        attendees: [],
        isAllDay: false,
      };

      const result = await invoke<{
        status: string;
        message?: string;
        summary?: string;
      }>("attach_meeting_transcript", {
        filePath: outcomes.transcriptPath,
        meeting: calendarEvent,
      });

      if (result.status !== "success") {
        toast.error("Retry failed", {
          description: result.message || result.status,
        });
      } else if (!result.summary) {
        toast.warning("No outcomes extracted", {
          description: result.message || "AI extraction returned empty",
        });
      } else {
        toast.success("Outcomes extracted");
      }
      await loadMeetingIntelligence();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("Failed to retry transcript extraction:", msg);
      toast.error("Retry failed", { description: msg });
    } finally {
      setRetryingExtraction(false);
    }
  }, [meetingId, data, meetingMeta, outcomes?.transcriptPath, loadMeetingIntelligence]);

  const handleDraftAgendaMessage = useCallback(async () => {
    if (!meetingId) return;
    await draft.openDraft(meetingId, data?.meetingContext || undefined);
  }, [meetingId, data?.meetingContext, draft]);

  const handlePrefillFromContext = useCallback(async () => {
    if (!meetingId || !canEditUserLayer) return;
    const candidateItems =
      data?.proposedAgenda
        ?.map((item) => cleanPrepLine(item.topic))
        .filter((item) => item.length > 0)
        .slice(0, 4) ?? [];
    if (candidateItems.length === 0) return;

    setPrefilling(true);
    try {
      const result = await invoke<ApplyPrepPrefillResult>(
        "apply_meeting_prep_prefill",
        {
          meetingId,
          agendaItems: candidateItems,
          notesAppend: data?.meetingContext || "",
        }
      );
      if (result.addedAgendaItems > 0 || result.notesAppended) {
        setPrefillNotice(true);
        setTimeout(() => setPrefillNotice(false), 5000);
      }
      await loadMeetingIntelligence();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to prefill briefing context"
      );
    } finally {
      setPrefilling(false);
    }
  }, [meetingId, canEditUserLayer, data, loadMeetingIntelligence]);

  const handleRefreshIntelligence = useCallback(async () => {
    if (!meetingId) return;
    setRefreshingIntel(true);
    toast("Refreshing briefing…", { duration: 10_000, id: "intel-refresh" });
    try {
      const refreshed = await invoke<MeetingBriefingRefreshResult>(
        "refresh_meeting_briefing",
        { meetingId },
      );
      await loadMeetingIntelligence();
      if (refreshed.failedEntities > 0) {
        toast.success(
          `Briefing updated (${refreshed.failedEntities} retries queued)`,
          { id: "intel-refresh" },
        );
      } else {
        toast.success("Briefing updated", { id: "intel-refresh" });
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Update failed", { id: "intel-refresh" });
    } finally {
      setRefreshingIntel(false);
    }
  }, [meetingId, loadMeetingIntelligence]);

  const handleRefreshProgress = useCallback((progress: MeetingBriefingRefreshProgress) => {
    if (!meetingId || progress.meetingId !== meetingId) return;
    if (!refreshingIntel) return;
    if (progress.stage === "completed") return;
    toast(progress.message, { id: "intel-refresh", duration: 10_000 });
  }, [meetingId, refreshingIntel]);

  useTauriEvent<MeetingBriefingRefreshProgress>(
    "meeting-briefing-refresh-progress",
    handleRefreshProgress,
  );

  const handlePrepReady = useCallback((payload: PrepReadyPayload) => {
    if (!meetingId || payload.meetingId !== meetingId) return;
    setBriefingUpdating(false);
    void loadMeetingIntelligence();
  }, [meetingId, loadMeetingIntelligence]);

  useTauriEvent<PrepReadyPayload>("prep-ready", handlePrepReady);

  const handleTranscriptProcessed = useCallback((payload: TranscriptProcessedPayload) => {
    if (!meetingId) return;
    if (typeof payload === "string") {
      if (payload !== meetingId) return;
      void loadMeetingIntelligence();
      return;
    }
    if (payload.meetingId !== meetingId) return;
    setOutcomes(payload);
    void loadMeetingIntelligence();
  }, [meetingId, loadMeetingIntelligence]);

  useTauriEvent<TranscriptProcessedPayload>("transcript-processed", handleTranscriptProcessed);

  // Safety timeout: clear updating banner after 15s if prep-ready never arrives (I477)
  useEffect(() => {
    if (!briefingUpdating) return;
    const timer = setTimeout(() => {
      setBriefingUpdating(false);
      void loadMeetingIntelligence();
    }, 15_000);
    return () => clearTimeout(timer);
  }, [briefingUpdating, loadMeetingIntelligence]);

  const copyToClipboard = useCallback(async (text: string, action: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedAction(action);
    setTimeout(() => setCopiedAction(null), 2000);
  }, []);

  const handleShareIntelligence = useCallback(async () => {
    if (!data) return;

    const lines: string[] = [];
    lines.push(`Meeting Briefing: ${data.title}`);
    if (data.timeRange) lines.push(data.timeRange);
    lines.push("");

    const summary = data.intelligenceSummary || data.meetingContext;
    if (summary) {
      lines.push("Summary");
      lines.push(sanitizeInlineText(summary));
      lines.push("");
    }

    const risks = [
      ...((data.entityRisks ?? []).map((r) => sanitizeInlineText(r.text))),
      ...(data.risks ?? []).map((r) => sanitizeInlineText(r)),
    ].filter((r) => r.length > 0).slice(0, 3);
    if (risks.length > 0) {
      lines.push("Key Risks");
      risks.forEach((r) => lines.push(`\u2022 ${r}`));
      lines.push("");
    }

    // Plan items — match the filtered proposedAgenda displayed in "Your Plan"
    const planItems = (data.proposedAgenda ?? [])
      .map((item) => sanitizeInlineText(item.topic))
      .filter((t) => t.length > 0);
    const planNonWin = (data.proposedAgenda ?? [])
      .filter((item) => item.source !== "talking_point")
      .map((item) => sanitizeInlineText(item.topic))
      .filter((t) => t.length > 0);
    const sharePlanItems = planNonWin.length > 0 ? planNonWin : planItems;
    if (sharePlanItems.length > 0) {
      lines.push("Your Plan");
      sharePlanItems.forEach((p) => lines.push(`\u2022 ${p}`));
      lines.push("");
    }

    const context = (data.currentState ?? [])
      .map((c) => sanitizeInlineText(c))
      .filter((c) => c.length > 0)
      .slice(0, 3);
    if (context.length > 0) {
      lines.push("Context");
      context.forEach((c) => lines.push(`\u2022 ${c}`));
    }

    await copyToClipboard(lines.join("\n").trim(), "share");
  }, [data, copyToClipboard]);

  const handleRequestInput = useCallback(async () => {
    if (!data) return;

    const userAgendaItems = data.userAgenda && data.userAgenda.length > 0
      ? data.userAgenda.map((item) => `- ${item}`).join("\n")
      : (data.proposedAgenda && data.proposedAgenda.length > 0
        ? data.proposedAgenda.slice(0, 5).map((item) => `- ${cleanPrepLine(item.topic)}`).join("\n")
        : "No agenda items yet");

    const meetingDate = data.timeRange || "upcoming";

    const message = `Hi team,

We have ${data.title} coming up on ${meetingDate}. I'd like to make sure we cover everything important.

Current agenda:
${userAgendaItems}

Please reply with any topics, questions, or materials you'd like to discuss.

Thanks!`;

    await copyToClipboard(message, "request");
  }, [data, copyToClipboard]);

  useEffect(() => {
    loadMeetingIntelligence();
  }, [loadMeetingIntelligence]);

  useEffect(() => {
    invoke<{ enabled: boolean }>("get_quill_status")
      .then((s) => setQuillEnabled(s.enabled))
      .catch(() => {});
    invoke<{ enabled: boolean }>("get_granola_status")
      .then((s) => setGranolaEnabled(s.enabled))
      .catch(() => {});
  }, []);

  // Reveal observer for editorial-reveal animations
  useRevealObserver(!loading && !!data);

  // Time-aware banner: compute minutes until meeting
  const minutesUntilMeeting = useMemo(() => {
    if (!meetingMeta?.startTime) return null;
    const start = parseDate(meetingMeta.startTime);
    if (!start) return null;
    const diff = Math.round((start.getTime() - Date.now()) / 60000);
    return diff > 0 && diff <= 120 ? diff : null;
  }, [meetingMeta?.startTime]);

  // Determine meeting time state for editability (I194)
  const isPastMeeting = !canEditUserLayer;
  const isEditable = canEditUserLayer;

  // Save a single field in prep_frozen_json (I548 — inline editing for briefing sections)
  const savePrepField = useCallback(async (fieldPath: string, value: string, targetPersonId?: string) => {
    if (!meetingId || !isEditable) return;
    setSaveStatus("saving");
    try {
      await invoke("update_meeting_prep_field", {
        meetingId, fieldPath, value,
        targetPersonId: targetPersonId ?? null,
      });
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      console.error("Save failed:", err);
      toast.error("Failed to save change");
      setSaveStatus("idle");
    }
  }, [meetingId, isEditable]);

  // Collaboration action visibility
  const isFutureMeeting = !isPastMeeting;
  const isReadyOrFresh = intelligenceQuality?.level === "ready" || intelligenceQuality?.level === "fresh";
  const isThreeDaysOut = useMemo(() => {
    if (!meetingMeta?.startTime) return false;
    const start = parseDate(meetingMeta.startTime);
    if (!start) return false;
    return start.getTime() > Date.now() + 3 * 24 * 60 * 60 * 1000;
  }, [meetingMeta?.startTime]);

  // Register magazine shell with chapter nav + folio actions
  const shellConfig = useMemo(() => ({
    folioLabel: "Meeting Briefing",
    atmosphereColor: "turmeric" as const,
    activePage: "today" as const,
    backLink: { label: "Back", onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/" }) },
    chapters: CHAPTERS,
    folioStatusText: saveStatus === "saving" ? "Saving…" : saveStatus === "saved" ? "✓ Saved" : undefined,
    folioActions: data ? (
      <div className={styles.folioActions}>
        {copiedAction && (
          <span className={styles.folioCopied}>
            <Check className={styles.iconSm} /> Copied
          </span>
        )}
        {isFutureMeeting && isReadyOrFresh && (
          <button onClick={handleShareIntelligence} title="Share Briefing" className={styles.folioBtnInline}>
            <Copy className={styles.iconSm} />
            Share
          </button>
        )}
        {isFutureMeeting && isThreeDaysOut && (
          <button onClick={handleRequestInput} title="Request Input" className={styles.folioBtnInline}>
            <Mail className={styles.iconSm} />
            Request Input
          </button>
        )}
        {isFutureMeeting && (
          <button onClick={handleDraftAgendaMessage} className={styles.folioBtn}>
            Draft Agenda
          </button>
        )}
        <FolioRefreshButton
          onClick={handleRefreshIntelligence}
          loading={refreshingIntel}
          title="Check for updates"
        />
      </div>
    ) : undefined,
  }), [navigate, saveStatus, data, isEditable, refreshingIntel, isPastMeeting, isFutureMeeting, isReadyOrFresh, isThreeDaysOut, copiedAction, meetingId, handleRefreshIntelligence, handleDraftAgendaMessage, handleShareIntelligence, handleRequestInput, loadMeetingIntelligence]);
  useRegisterMagazineShell(shellConfig);

  // ── Loading state ──
  if (loading) {
    return <EditorialLoading count={5} />;
  }

  // ── Error state ──
  if (error) {
    return <EditorialError message={error} onRetry={() => loadMeetingIntelligence()} />;
  }

  // ── Empty / not-ready state ──
  if (!data) {
    return (
      <div className={styles.pageContainerPadded}>
        <div className={styles.emptyState}>
          <Clock className={styles.emptyIcon} />
          <h2 className={styles.emptyTitle}>Not ready yet</h2>
          <p className={styles.emptyText}>Meeting context will appear here after the daily briefing runs.</p>
        </div>
      </div>
    );
  }

  const hasAnyContent = Boolean(
    data.meetingContext ||
    data.calendarNotes ||
    data.accountSnapshot ||
    (data.quickContext && data.quickContext.length > 0) ||
    (data.attendees && data.attendees.length > 0) ||
    (data.attendeeContext && data.attendeeContext.length > 0) ||
    (data.sinceLast && data.sinceLast.length > 0) ||
    (data.strategicPrograms && data.strategicPrograms.length > 0) ||
    (data.currentState && data.currentState.length > 0) ||
    (data.risks && data.risks.length > 0) ||
    (data.recentWins && data.recentWins.length > 0) ||
    (data.talkingPoints && data.talkingPoints.length > 0) ||
    (data.openItems && data.openItems.length > 0) ||
    (data.questions && data.questions.length > 0) ||
    (data.keyPrinciples && data.keyPrinciples.length > 0) ||
    (data.proposedAgenda && data.proposedAgenda.length > 0) ||
    data.stakeholderSignals
  );
  const hasLinkedEntities = linkedEntities.length > 0;

  // Derived data — track source for inline editing (I548)
  const topRiskEntries = [
    ...((data.entityRisks ?? []).map((risk, i) => ({
      text: sanitizeInlineText(risk.text),
      fieldPath: `entityRisks[${i}].text`,
    }))),
    ...((data.risks ?? []).map((risk, i) => ({
      text: sanitizeInlineText(risk),
      fieldPath: `risks[${i}]`,
    }))),
  ]
    .filter((r) => r.text.length > 0)
    .slice(0, 3);
  const topRisks = topRiskEntries.map((r) => r.text);
  const lifecycle = getLifecycleForDisplay(data);
  const agendaItems = (data.proposedAgenda ?? [])
    .map((item) => ({
      ...item,
      topic: cleanPrepLine(item.topic),
      why: item.why ? cleanPrepLine(item.why) : undefined,
    }))
    .filter((item) => item.topic.length > 0);
  const agendaNonWinItems = agendaItems.filter((item) => item.source !== "talking_point");
  const agendaDisplayItems = agendaNonWinItems.length > 0 ? agendaNonWinItems : agendaItems;
  const calendarNotes = stripHtml(data.calendarNotes);

  // Build unified attendees
  const unifiedAttendees = buildUnifiedAttendees(
    data.attendeeContext,
    data.attendees,
    data.stakeholderInsights,
    data.stakeholderSignals,
  );

  // Key insight — first sentence from intelligence summary
  const keyInsight = extractKeyInsight(data.intelligenceSummary, data.meetingContext);

  // Meeting type label
  const meetingType = meetingMeta?.meetingType
    ? meetingMeta.meetingType.replace(/_/g, " ")
    : undefined;

  // Track which risks are high urgency for the pulse animation
  const topRiskUrgencies = [
    ...((data.entityRisks ?? []).map((risk) => ({ text: sanitizeInlineText(risk.text), urgency: risk.urgency }))),
    ...(data.risks ?? []).map((risk) => ({ text: sanitizeInlineText(risk), urgency: undefined as string | undefined })),
  ]
    .filter((r) => r.text.length > 0)
    .slice(0, 3);

  const hasRisks = topRisks.length > 0;
  const hasRoom = unifiedAttendees.length > 0;
  const hasPlan = agendaDisplayItems.length > 0 || (meetingId && isEditable);
  return (
    <>
      <div className={styles.pageContainer}>
        {/* Post-meeting intelligence — replaces flat outcomes when available */}
        {postIntel && (
          <>
            <div className={styles.outcomesWrap}>
              <PostMeetingIntelligence
                data={postIntel}
                getItemFeedback={feedback.getFeedback}
                onItemFeedback={feedback.submitFeedback}
              />
            </div>
            <div className={styles.outcomesDivider} />
            <p className={styles.preMeetingLabel}>
              {isPastMeeting ? "Pre-Meeting Context" : "Meeting Briefing"}
            </p>
          </>
        )}

        {/* Flat outcomes — shown only when no post-meeting intelligence is available */}
        {outcomes && !postIntel && (
          <>
            <div className={styles.outcomesWrap}>
              <OutcomesSection
                outcomes={outcomes}
                onRefresh={loadMeetingIntelligence}
                onSaveStatus={setSaveStatus}
                onRetryTranscriptExtraction={handleRetryTranscriptExtraction}
                retryingExtraction={retryingExtraction}
              />
            </div>
            <div className={styles.outcomesDivider} />
            <p className={styles.preMeetingLabel}>
              {isPastMeeting ? "Pre-Meeting Context" : "Meeting Briefing"}
            </p>
          </>
        )}

        {!hasAnyContent && !outcomes && !postIntel && (
          <div className={clsx(styles.emptyState, !hasLinkedEntities && styles.emptyStateActionable)}>
            {hasLinkedEntities && <Clock className={styles.emptyIcon} />}
            {!hasLinkedEntities ? (
              <>
                <p className={styles.emptyGenerating}>No linked account or project</p>
                <p className={styles.emptyText}>
                  We don&apos;t have enough context to build this briefing yet.
                  Link this meeting to an account or project.
                </p>
              </>
            ) : isPastMeeting ? (
              <>
                <p className={styles.emptyGenerating}>No briefing available</p>
                <p className={styles.emptyText}>This meeting&apos;s context wasn&apos;t captured before it started.</p>
              </>
            ) : (
              <>
                <p className={styles.emptyGenerating}>Building context</p>
                <p className={styles.emptyText}>Meeting context will appear here once analysis completes.</p>
              </>
            )}
            {meetingId && (
              <div className={styles.entityChipsWrapCentered}>
                <MeetingEntityChips
                  meetingId={meetingId}
                  meetingTitle={meetingMeta?.title ?? data.title}
                  meetingStartTime={meetingMeta?.startTime ?? new Date().toISOString()}
                  meetingType={meetingMeta?.meetingType ?? "internal"}
                  linkedEntities={linkedEntities}
                  onEntitiesChanged={() => {
                    setBriefingUpdating(true);
                    void loadMeetingIntelligence();
                  }}
                />
              </div>
            )}
          </div>
        )}

        {/* Briefing updating banner (I477) — shown during entity mutation */}
        {briefingUpdating && (
          <div className={styles.updatingBanner}>
            <Loader2 size={14} />
            <span>Updating briefing...</span>
          </div>
        )}

        {(hasAnyContent || outcomes || postIntel) && (
          <div className={isPastMeeting && outcomes ? styles.pastMeetingOpacity : undefined}>

            {/* ================================================================
                ACT I: "Ground Me" — visible immediately, NO editorial-reveal
               ================================================================ */}
            <section id="headline" className={styles.heroSection}>
              {/* Time-aware urgency banner */}
              {minutesUntilMeeting != null && (
                <div className={clsx(styles.urgencyBanner, minutesUntilMeeting <= 15 ? styles.urgencyUrgent : styles.urgencySoon)}>
                  <Clock className={styles.iconLg} />
                  Meeting starts in {minutesUntilMeeting} minute{minutesUntilMeeting !== 1 ? "s" : ""}
                </div>
              )}

              {/* Kicker */}
              <p className={styles.monoOverline}>
                Meeting Briefing
              </p>

              {/* Title — 76px editorial hero scale */}
              <h1 className={styles.heroTitle}>
                {data.title}
              </h1>
              {lifecycle && (
                <div className={styles.lifecycleBadge}>
                  <span className={styles.bulletDotTurmeric} />
                  <span className={styles.lifecycleText}>{lifecycle}</span>
                </div>
              )}

              {/* Metadata line */}
              <div className={styles.metadataLine}>
                <p className={styles.metadataText}>
                  {data.timeRange}
                  {meetingType && <> &middot; {meetingType}</>}
                  {getPrimaryEntityName(linkedEntities) && (
                    <> &middot; {getPrimaryEntityName(linkedEntities)}</>
                  )}
                </p>
                {intelligenceQuality && (
                  <IntelligenceQualityBadge quality={intelligenceQuality} showLabel />
                )}
              </div>

              {/* Entity chips */}
              {meetingId && (
                <div className={styles.entityChipsWrap}>
                <MeetingEntityChips
                  meetingId={meetingId}
                  meetingTitle={meetingMeta?.title ?? data.title}
                  meetingStartTime={meetingMeta?.startTime ?? new Date().toISOString()}
                  meetingType={meetingMeta?.meetingType ?? "internal"}
                  linkedEntities={linkedEntities}
                  onEntitiesChanged={() => {
                    setBriefingUpdating(true);
                    void loadMeetingIntelligence();
                  }}
                />
              </div>
            )}

              {/* Account health strip — I502 */}
              {linkedEntities.some((e) => e.entityType === "account" && entityHealthMap?.[e.id]) && (
                <div className={styles.healthStripContainer}>
                  {linkedEntities
                    .filter((e) => e.entityType === "account" && entityHealthMap?.[e.id])
                    .map((e) => {
                      const h = entityHealthMap![e.id];
                      return (
                        <span key={e.id} className={styles.healthStripItem}>
                          <span className={styles.healthStripLabel}>
                            {e.name}
                          </span>
                          <HealthBadge score={h.score} band={h.band} trend={h.trend} size="compact" />
                        </span>
                      );
                    })}
                </div>
              )}

              {/* New signals banner */}
              {intelligenceQuality?.hasNewSignals && (
                <div className={styles.newSignalsBanner}>
                  <span>New information available since your last view</span>
                  <FolioRefreshButton
                    onClick={handleRefreshIntelligence}
                    loading={refreshingIntel}
                  />
                </div>
              )}

              {/* I527: Deterministic consistency status banner */}
              {data.consistencyStatus && data.consistencyStatus !== "ok" && (
                <div
                  className={clsx(
                    styles.consistencyBanner,
                    data.consistencyStatus === "flagged"
                      ? styles.consistencyBannerFlagged
                      : styles.consistencyBannerCorrected,
                  )}
                >
                  <span>
                    {data.consistencyStatus === "corrected"
                      ? "Context auto-corrected against meeting records."
                      : "Some context could not be fully verified from meeting records."}
                  </span>
                  {data.consistencyFindings && data.consistencyFindings.length > 0 && (
                    <span className={styles.consistencyCount}>
                      {data.consistencyFindings.length} check
                      {data.consistencyFindings.length === 1 ? "" : "s"}
                    </span>
                  )}
                </div>
              )}

              {/* The Key Insight — pull quote style */}
              {keyInsight && (
                <blockquote className={styles.keyInsight}>
                  {isEditable ? (
                    <EditableText
                      value={keyInsight}
                      onChange={(v) => savePrepField(
                        data.intelligenceSummary ? "intelligenceSummary" : "meetingContext",
                        v,
                      )}
                      as="p"
                      multiline
                      className={styles.keyInsightText}
                    />
                  ) : (
                    <p className={styles.keyInsightText}>{keyInsight}</p>
                  )}
                </blockquote>
              )}
              {!keyInsight && (
                <p className={styles.keyInsightEmpty}>
                  Intelligence builds as you meet with this account.
                </p>
              )}

              {/* Since Last Meeting — compact timeline of what changed */}
              {data.sinceLast && data.sinceLast.length > 0 && (
                <div className={styles.sinceLastWrap}>
                  <p className={styles.sinceLastHeading}>Since Last Meeting</p>
                  <ul className={styles.sinceLastList}>
                    {data.sinceLast.map((item, i) => (
                      <li key={i} className={styles.sinceLastItem}>
                        <span className={styles.bulletDotTurmericMuted} />
                        <span>{sanitizeInlineText(item)}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Account Pulse — what's working / not working */}
              {data.currentState && data.currentState.length > 0 && (
                <div className={styles.currentStateWrap}>
                  <p className={styles.currentStateHeading}>Account Pulse</p>
                  <div className={styles.currentStateGrid}>
                    {data.currentState.map((item, i) => (
                      <span key={i} className={styles.currentStateItem}>
                        {sanitizeInlineText(item)}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Entity Readiness — "Before This Meeting" checklist */}
              {data.entityReadiness && data.entityReadiness.length > 0 && (
                <div className={styles.readinessWrap}>
                  <p className={styles.readinessHeading}>Before This Meeting</p>
                  <ul className={styles.readinessList}>
                    {data.entityReadiness.slice(0, 4).map((item, i) => (
                      <li key={i} className={styles.readinessItem}>
                        <span className={styles.bulletDotTurmericMuted} />
                        <span>{item}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </section>

            {/* ================================================================
                ACT II: "Brief Me" — editorial-reveal for each chapter
               ================================================================ */}

            {/* Chapter: The Risks */}
            {hasRisks && (
              <section id="risks" className={clsx("editorial-reveal", styles.chapterSection)}>
                <ChapterHeading
                  title="The Risks"
                  feedbackSlot={
                    <IntelligenceFeedback
                      value={feedback.getFeedback("risks")}
                      onFeedback={(type) => feedback.submitFeedback("risks", type)}
                    />
                  }
                />
                <div className={styles.risksContainer}>
                  {topRisks.map((risk, i) => {
                    const isHighUrgency = topRiskUrgencies[i]?.urgency === "high";
                    const riskContent = isEditable ? (
                      <EditableText
                        value={risk}
                        onChange={(v) => savePrepField(topRiskEntries[i].fieldPath, v)}
                        as="p"
                        multiline
                        className={i === 0 ? styles.featuredRiskText : styles.subordinateRiskText}
                      />
                    ) : (
                      <p className={i === 0 ? styles.featuredRiskText : styles.subordinateRiskText}>{risk}</p>
                    );
                    return i === 0 ? (
                      <blockquote
                        key={i}
                        className={clsx(styles.featuredRisk, isHighUrgency && "risk-pulse-once")}
                      >
                        {riskContent}
                      </blockquote>
                    ) : (
                      <div
                        key={i}
                        className={clsx(styles.subordinateRisk, isHighUrgency && styles.subordinateRiskHighUrgency, isHighUrgency && "risk-pulse-once")}
                      >
                        {riskContent}
                      </div>
                    );
                  })}
                </div>
              </section>
            )}

            {/* Recent Wins — positive context after risks */}
            {data.recentWins && data.recentWins.length > 0 && (
              <section className={clsx("editorial-reveal", styles.chapterSection)}>
                <ChapterHeading title="Recent Wins" />
                <div className={styles.recentWinsContainer}>
                  {data.recentWins.map((win, i) => (
                    <div key={i} className={styles.recentWinItem}>
                      <span className={styles.bulletDotSage} />
                      <p className={styles.recentWinText}>{cleanPrepLine(win)}</p>
                    </div>
                  ))}
                </div>
              </section>
            )}

            {/* Open Items — follow-ups from previous meetings */}
            {data.openItems && data.openItems.length > 0 && (
              <section className={clsx("editorial-reveal", styles.chapterSection)}>
                <ChapterHeading title="Open Items" />
                <div className={styles.openItemsContainer}>
                  {data.openItems.map((item, i) => (
                    <div
                      key={i}
                      className={clsx(
                        styles.openItemRow,
                        item.isOverdue && styles.openItemOverdue,
                      )}
                    >
                      <p className={styles.openItemTitle}>{item.title}</p>
                      {item.dueDate && (
                        <span
                          className={clsx(
                            styles.openItemDue,
                            item.isOverdue && styles.openItemDueOverdue,
                          )}
                        >
                          {item.isOverdue ? "Overdue" : "Due"}: {item.dueDate}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </section>
            )}

            {/* Chapter: The Room */}
            {hasRoom && (
              <section id="the-room" className={clsx("editorial-reveal", styles.chapterSection)}>
                <ChapterHeading title="The Room" />
                <UnifiedAttendeeList
                  attendees={unifiedAttendees}
                  isEditable={isEditable}
                  initialHiddenNames={hiddenAttendees}
                  meetingId={meetingId ?? undefined}
                  onSaveStatus={setSaveStatus}
                  onSavePrepField={savePrepField}
                />
              </section>
            )}

            {/* Recent Correspondence — email signals for meeting attendees */}
            {data.recentEmailSignals && data.recentEmailSignals.length > 0 && (
              <section className={clsx("editorial-reveal", styles.chapterSection)}>
                <ChapterHeading
                  title="Recent Correspondence"
                  feedbackSlot={
                    <IntelligenceFeedback
                      value={feedback.getFeedback("recent_correspondence")}
                      onFeedback={(type) =>
                        feedback.submitFeedback("recent_correspondence", type)
                      }
                    />
                  }
                />
                <div className={styles.risksContainer}>
                  {data.recentEmailSignals.map((signal, i) => (
                    <Link key={i} to="/emails" className={styles.emailSignalLink}>
                      <div className={styles.emailSignalHeader}>
                        <span className={styles.emailSignalSender}>
                          {signal.senderEmail}
                        </span>
                        {signal.detectedAt && (
                          <span className={styles.emailSignalDate}>
                            {new Date(signal.detectedAt).toLocaleDateString("en-US", { month: "short", day: "numeric" })}
                          </span>
                        )}
                      </div>
                      <p className={styles.subordinateRiskText}>{signal.signalText}</p>
                    </Link>
                  ))}
                </div>
              </section>
            )}

            {/* Chapter: Your Plan */}
            {hasPlan && (
              <section id="your-plan" className={clsx("editorial-reveal", styles.chapterSection)}>
                <ChapterHeading
                  title="Your Plan"
                  feedbackSlot={
                    <IntelligenceFeedback
                      value={feedback.getFeedback("plan")}
                      onFeedback={(type) => feedback.submitFeedback("plan", type)}
                    />
                  }
                />

                {meetingId && prefillNotice && (
                  <div className={styles.prefillNotice}>
                    Prefill appended new agenda/notes content.
                  </div>
                )}

                <UnifiedPlanEditor
                  proposedItems={agendaDisplayItems}
                  userAgenda={data.userAgenda}
                  meetingId={meetingId ?? undefined}
                  isEditable={isEditable}
                  calendarNotes={calendarNotes}
                  initialDismissedTopics={dismissedTopics}
                  onSaveStatus={setSaveStatus}
                />

                {isEditable && (
                  <button
                    onClick={handlePrefillFromContext}
                    disabled={prefilling}
                    className={clsx(styles.planSecondaryAction, prefilling && styles.folioBtnDisabled)}
                  >
                    {prefilling ? "Prefilling from context…" : "Prefill from context"}
                  </button>
                )}
              </section>
            )}

            {/* Transcript CTA — moved from folio bar to page body */}
            <div className={clsx("editorial-reveal", styles.transcriptCta)}>
              {quillEnabled && (
                <button
                  onClick={handleSyncTranscript}
                  disabled={syncing}
                  className={clsx(styles.transcriptCtaBtn, syncing && styles.transcriptCtaBtnDisabled)}
                >
                  {syncing ? <Loader2 className={clsx(styles.iconMd, styles.spinAnimation)} /> : <RefreshCw className={styles.iconMd} />}
                  {syncing ? "Syncing…" : "Sync Transcript"}
                </button>
              )}
              <button
                onClick={handleAttachTranscript}
                disabled={attaching}
                className={clsx(styles.transcriptCtaBtn, attaching && styles.transcriptCtaBtnDisabled)}
              >
                {attaching ? <Loader2 className={clsx(styles.iconMd, styles.spinAnimation)} /> : <Paperclip className={styles.iconMd} />}
                {attaching ? "Processing…" : "Attach Transcript"}
              </button>
              {granolaEnabled && (
                <span className={styles.transcriptCtaLabel}>
                  Granola transcripts sync automatically
                </span>
              )}
              {!granolaEnabled && (
                <span className={styles.transcriptCtaLabel}>
                  {isPastMeeting ? "Add meeting transcript for outcome extraction" : "Attach transcript or notes"}
                </span>
              )}
            </div>

            {/* ================================================================
                "You're Briefed" — FinisMarker
               ================================================================ */}
            <div className={styles.finisWrap}>
              <FinisMarker />
            </div>

          </div>
        )}

      </div>
      <AgendaDraftDialog
        open={draft.open}
        onOpenChange={draft.setOpen}
        loading={draft.loading}
        subject={draft.subject}
        body={draft.body}
      />
    </>
  );
}

// =============================================================================
// Unified Attendee List (merges attendees, context, insights, signals)
// =============================================================================

function UnifiedAttendeeList({
  attendees,
  isEditable,
  initialHiddenNames,
  meetingId,
  onSaveStatus,
  onSavePrepField,
}: {
  attendees: UnifiedAttendee[];
  isEditable?: boolean;
  initialHiddenNames?: string[];
  meetingId?: string;
  onSaveStatus?: (status: "idle" | "saving" | "saved") => void;
  onSavePrepField?: (fieldPath: string, value: string, targetPersonId?: string) => void;
}) {
  const [showAll, setShowAll] = useState(false);
  const [hiddenNames, setHiddenNames] = useState<Set<string>>(
    new Set((initialHiddenNames ?? []).map(normalizePersonKey))
  );
  const filtered = attendees.filter((p) => !hiddenNames.has(normalizePersonKey(p.name)));
  const visible = showAll ? filtered : filtered.slice(0, 4);
  const remaining = filtered.length - 4;

  const tempIndicatorClass: Record<string, string> = {
    hot: styles.tempIndicatorHot,
    warm: styles.tempIndicatorWarm,
    cool: styles.tempIndicatorCool,
    cold: styles.tempIndicatorCold,
  };

  const tempLabelClass: Record<string, string> = {
    hot: styles.tempLabelHot,
    warm: styles.tempLabelWarm,
    cool: styles.tempLabelCool,
    cold: styles.tempLabelCold,
  };

  const engagementClass: Record<string, string> = {
    champion: styles.engagementChampion,
    detractor: styles.engagementDetractor,
    neutral: styles.engagementNeutral,
  };

  return (
    <div className={styles.attendeeList}>
      {visible.map((person, i) => {
        const tempIndCls = tempIndicatorClass[person.temperature ?? ""] ?? styles.tempIndicatorDefault;
        const tempLblCls = tempLabelClass[person.temperature ?? ""] ?? styles.tempLabelDefault;
        const isNew = person.meetingCount === 0;
        const isCold = person.temperature === "cold";
        const avatarClass = isCold
          ? styles.attendeeAvatarCold
          : isNew
          ? styles.attendeeAvatarNew
          : styles.attendeeAvatarDefault;

        const inner = (
          <div className={styles.attendeeRow}>
            {/* Avatar */}
            <div
              className={clsx(styles.attendeeAvatar, avatarClass)}
            >
              {person.name.charAt(0)}
            </div>

            <div className={styles.attendeeBody}>
              {/* Name + role + temperature dot + engagement badge */}
              <div className={styles.attendeeNameRow}>
                <span className="attendee-tooltip-wrap">
                  <p className={styles.attendeeName}>
                    {person.name}
                  </p>
                  {/* Hover tooltip — last meeting + assessment */}
                  {(person.lastSeen || person.assessment) && (
                    <span className="attendee-tooltip">
                      {person.lastSeen && (
                        <span className={clsx(styles.attendeeMetaMono, person.assessment ? styles.tooltipMetaBlockSpaced : styles.tooltipMetaBlock)}>
                          Last met {formatRelativeDateLong(person.lastSeen)}
                          {person.meetingCount != null && person.meetingCount > 0 && ` \u00b7 ${person.meetingCount} meeting${person.meetingCount !== 1 ? "s" : ""}`}
                        </span>
                      )}
                      {person.assessment && (
                        <span className={styles.tooltipAssessment}>
                          {truncateText(sanitizeInlineText(person.assessment), 140)}
                        </span>
                      )}
                    </span>
                  )}
                </span>
                {person.role && (
                  <span className={styles.attendeeRole}>
                    {sanitizeInlineText(person.role)}
                  </span>
                )}
                {person.temperature && (
                  <span className={styles.attendeeTempDot}>
                    <span className={clsx(styles.attendeeTempIndicator, tempIndCls)} />
                    <span className={clsx(styles.attendeeTempLabel, tempLblCls)}>
                      {person.temperature}
                    </span>
                  </span>
                )}
                {person.engagement && (
                  <span
                    className={clsx(styles.attendeeEngagement, engagementClass[person.engagement] ?? styles.engagementDefault)}
                  >
                    {person.engagement}
                  </span>
                )}
                {isNew && (
                  <span className={styles.attendeeNewContact}>New contact</span>
                )}
              </div>

              {/* Assessment — the killer insight, serif italic, prominent */}
              {person.assessment && (
                isEditable && onSavePrepField && person.assessmentFieldPath ? (
                  <EditableText
                    value={sanitizeInlineText(person.assessment)}
                    onChange={(v) => onSavePrepField(person.assessmentFieldPath!, v, person.personId)}
                    as="p"
                    multiline
                    className={styles.attendeeAssessment}
                  />
                ) : (
                  <p className={styles.attendeeAssessment}>
                    {truncateText(sanitizeInlineText(person.assessment), 200)}
                  </p>
                )
              )}

              {/* Metadata line */}
              <div className={styles.attendeeMeta}>
                {person.organization && (
                  <span className={styles.attendeeOrg}>{person.organization}</span>
                )}
                {person.meetingCount != null && person.meetingCount > 0 && (
                  <span className={styles.attendeeMetaMono}>
                    {person.meetingCount} meeting{person.meetingCount !== 1 ? "s" : ""}
                  </span>
                )}
                {person.lastSeen && (
                  <span className={isCold ? styles.attendeeMetaCold : styles.attendeeMetaMono}>
                    Last seen {formatRelativeDateLong(person.lastSeen)}
                  </span>
                )}
              </div>

              {person.notes && (
                <p className={styles.attendeeNotes}>
                  {person.notes}
                </p>
              )}
            </div>
          </div>
        );

        const row = person.personId ? (
          <Link
            to="/people/$personId"
            params={{ personId: person.personId }}
            className={styles.attendeeLink}
          >
            {inner}
          </Link>
        ) : (
          <div className={styles.attendeeNonLink}>{inner}</div>
        );

        return (
          <div key={i} className={styles.attendeeRowOuter}>
            {row}
            {isEditable && (
              <button
                onClick={async () => {
                  const key = normalizePersonKey(person.name);
                  const newHidden = new Set(hiddenNames).add(key);
                  setHiddenNames(newHidden);
                  if (meetingId) {
                    onSaveStatus?.("saving");
                    try {
                      await invoke("update_meeting_user_agenda", {
                        meetingId,
                        hiddenAttendees: Array.from(newHidden),
                      });
                      onSaveStatus?.("saved");
                      setTimeout(() => onSaveStatus?.("idle"), 2000);
                    } catch (err) {
                      console.error("Save failed:", err);
                      toast.error("Failed to save attendee visibility");
                      onSaveStatus?.("idle");
                    }
                  }
                }}
                className={styles.attendeeHideBtn}
              >
                &times;
              </button>
            )}
          </div>
        );
      })}

      {!showAll && remaining > 0 && (
        <button onClick={() => setShowAll(true)} className={styles.attendeeShowMore}>
          + {remaining} more
        </button>
      )}
    </div>
  );
}

// =============================================================================
// User Editability Components (I194 / ADR-0065)
// =============================================================================

function UnifiedPlanEditor({
  proposedItems,
  userAgenda,
  meetingId,
  isEditable,
  calendarNotes,
  initialDismissedTopics,
  onSaveStatus,
}: {
  proposedItems: Array<{ topic: string; why?: string; source?: string }>;
  userAgenda?: string[];
  meetingId?: string;
  isEditable: boolean;
  calendarNotes?: string;
  initialDismissedTopics?: string[];
  onSaveStatus: (status: "idle" | "saving" | "saved") => void;
}) {
  const [userItems, setUserItems] = useState(userAgenda ?? []);
  const [newItem, setNewItem] = useState("");
  const [newItemWhy, setNewItemWhy] = useState("");
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [editingValue, setEditingValue] = useState("");
  const [editingWhy, setEditingWhy] = useState("");
  const [dismissedTopics, setDismissedTopics] = useState<Set<string>>(
    new Set(initialDismissedTopics ?? [])
  );
  // Overrides for proposed items edited in-place (keeps position, persists as user items)
  const [proposedOverrides, setProposedOverrides] = useState<Map<string, string>>(new Map());

  function parseUserItem(raw: string): { topic: string; why?: string } {
    const sep = raw.indexOf(" — ");
    if (sep > 0) return { topic: raw.slice(0, sep), why: raw.slice(sep + 3) };
    const sep2 = raw.indexOf(" - ");
    if (sep2 > 20) return { topic: raw.slice(0, sep2), why: raw.slice(sep2 + 3) };
    return { topic: raw };
  }

  // Build unified list. Overridden proposed items stay in place during session;
  // their persisted user-item copies are hidden to avoid duplicates.
  const overriddenUserIndices = new Set<number>();
  const overrideValues = new Set(proposedOverrides.values());
  userItems.forEach((raw, i) => { if (overrideValues.has(raw)) overriddenUserIndices.add(i); });

  const allItems: Array<{ topic: string; why?: string; source?: string; isUser: boolean; userIndex?: number; originalProposedTopic?: string }> = [
    ...proposedItems
      .filter((item) => !dismissedTopics.has(item.topic) || proposedOverrides.has(item.topic))
      .map((item) => {
        const override = proposedOverrides.get(item.topic);
        if (override) {
          const parsed = parseUserItem(override);
          return { ...parsed, source: item.source, isUser: false, originalProposedTopic: item.topic };
        }
        return { ...item, isUser: false, originalProposedTopic: item.topic };
      }),
    ...userItems
      .map((raw, i) => ({ ...parseUserItem(raw), isUser: true as const, userIndex: i }))
      .filter((item) => !overriddenUserIndices.has(item.userIndex!)),
  ];

  async function saveLayer(updatedItems: string[], updatedDismissed?: Set<string>) {
    if (!meetingId) return;
    onSaveStatus("saving");
    try {
      const dismissed = Array.from(updatedDismissed ?? dismissedTopics);
      await invoke("update_meeting_user_agenda", {
        meetingId,
        agenda: updatedItems,
        dismissedTopics: dismissed.length > 0 ? dismissed : null,
      });
      setUserItems(updatedItems);
      onSaveStatus("saved");
      setTimeout(() => onSaveStatus("idle"), 2000);
    } catch (err) {
      console.error("Save failed:", err);
      toast.error("Failed to save agenda");
      onSaveStatus("idle");
    }
  }

  function addItem() {
    if (!newItem.trim()) return;
    const topic = newItem.trim();
    const why = newItemWhy.trim();
    const text = why ? `${topic} — ${why}` : topic;
    saveLayer([...userItems, text]);
    setNewItem("");
    setNewItemWhy("");
  }

  function removeItem(userIndex: number) {
    saveLayer(userItems.filter((_, i) => i !== userIndex));
  }

  function startEditing(listIndex: number, field: "topic" | "why" = "topic") {
    if (!isEditable) return;
    setEditingIndex(listIndex);
    setEditingValue(allItems[listIndex].topic);
    setEditingWhy(allItems[listIndex].why ?? "");
    // Focus the why field if that's what was clicked
    if (field === "why") {
      requestAnimationFrame(() => {
        document.getElementById(`plan-why-${listIndex}`)?.focus();
      });
    }
  }

  function commitEdit() {
    if (editingIndex == null) return;
    const item = allItems[editingIndex];
    const trimmed = editingValue.trim();
    const trimmedWhy = editingWhy.trim();
    if (!trimmed) {
      setEditingIndex(null);
      return;
    }
    const topicChanged = trimmed !== item.topic;
    const whyChanged = trimmedWhy !== (item.why ?? "");
    if (item.isUser && item.userIndex != null) {
      const updated = [...userItems];
      updated[item.userIndex] = trimmedWhy ? `${trimmed} — ${trimmedWhy}` : trimmed;
      saveLayer(updated);
    } else if (!item.isUser && item.originalProposedTopic && (topicChanged || whyChanged)) {
      // Override proposed item in-place (stays in same position)
      const newText = trimmedWhy ? `${trimmed} — ${trimmedWhy}` : trimmed;
      setProposedOverrides((prev) => new Map(prev).set(item.originalProposedTopic!, newText));
      // Persist: store the override as a user item + dismiss the original
      const newDismissed = new Set(dismissedTopics).add(item.originalProposedTopic);
      setDismissedTopics(newDismissed);
      saveLayer([...userItems, newText], newDismissed);
    }
    setEditingIndex(null);
  }

  if (allItems.length === 0 && !isEditable && !calendarNotes) {
    return (
      <p className={styles.planEmptyText}>No agenda prepared yet.</p>
    );
  }

  return (
    <div className={styles.planContainer}>
      {/* Unified numbered list */}
      {allItems.length > 0 && (
        <ol className={styles.planList}>
          {allItems.map((item, i) => (
            <li
              key={`${item.isUser ? "u" : "p"}-${i}`}
              className={styles.planItem}
            >
              <span className={styles.planNumber}>
                {i + 1}
              </span>
              <div className={styles.planItemBody}>
                {editingIndex === i && isEditable ? (
                  <div
                    className={styles.planEditWrap}
                    onBlur={(e) => {
                      // Only commit if focus leaves both inputs (not moving between them)
                      if (!e.currentTarget.contains(e.relatedTarget as Node)) {
                        commitEdit();
                      }
                    }}
                  >
                    <input
                      autoFocus
                      value={editingValue}
                      onChange={(e) => setEditingValue(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitEdit();
                        if (e.key === "Escape") setEditingIndex(null);
                      }}
                      className={styles.planEditInput}
                    />
                    <input
                      id={`plan-why-${i}`}
                      value={editingWhy}
                      onChange={(e) => setEditingWhy(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitEdit();
                        if (e.key === "Escape") setEditingIndex(null);
                      }}
                      placeholder="Why this matters..."
                      className={styles.planEditInputWhy}
                    />
                  </div>
                ) : (
                  <>
                    <p
                      onClick={() => startEditing(i)}
                      className={isEditable ? styles.planTopicEditable : styles.planTopic}
                    >
                      {item.topic}
                    </p>
                    {item.why && (
                      <p
                        onClick={() => startEditing(i, "why")}
                        className={isEditable ? styles.planWhyEditable : styles.planWhy}
                      >
                        {item.why}
                      </p>
                    )}
                  </>
                )}
              </div>
              <div className={styles.planItemActions}>
                {isEditable && (
                  <button
                    onClick={() => {
                      if (item.isUser && item.userIndex != null) {
                        removeItem(item.userIndex);
                      } else if (!item.isUser && item.originalProposedTopic) {
                        const origTopic = item.originalProposedTopic;
                        // Remove any override for this item
                        const newOverrides = new Map(proposedOverrides);
                        const overrideText = newOverrides.get(origTopic);
                        newOverrides.delete(origTopic);
                        setProposedOverrides(newOverrides);
                        // Dismiss the original
                        const newDismissed = new Set(dismissedTopics).add(origTopic);
                        setDismissedTopics(newDismissed);
                        // Remove the persisted user-item copy if it exists
                        const cleaned = overrideText
                          ? userItems.filter((u) => u !== overrideText)
                          : userItems;
                        saveLayer(cleaned, newDismissed);
                      }
                    }}
                    className={styles.planDismissBtn}
                  >
                    &times;
                  </button>
                )}
              </div>
            </li>
          ))}
        </ol>
      )}

      {/* Ghost input — topic + why */}
      {isEditable && meetingId && (
        <div className={styles.ghostInputRow}>
          <span className={styles.planNumberGhost}>
            {allItems.length + 1}
          </span>
          <div className={styles.ghostInputBody}>
            <input
              value={newItem}
              onChange={(e) => setNewItem(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addItem()}
              placeholder="Add agenda item..."
              className={styles.ghostInput}
            />
            {newItem.trim() && (
              <input
                value={newItemWhy}
                onChange={(e) => setNewItemWhy(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addItem()}
                placeholder="Why this matters..."
                className={styles.ghostInputWhy}
              />
            )}
          </div>
        </div>
      )}

      {/* Calendar description — collapsed toggle */}
      {calendarNotes && (
        <details className={styles.calendarDetails}>
          <summary className={styles.calendarSummary}>
            <ChevronRight className={clsx(styles.iconMd, styles.detailsChevron)} />
            Calendar Description
          </summary>
          <p className={styles.calendarBody}>{calendarNotes}</p>
        </details>
      )}
    </div>
  );
}

// =============================================================================
// Outcomes Section (I195 / ADR-0066)
// =============================================================================

function OutcomesSection({
  outcomes,
  onRefresh,
  onSaveStatus: _onSaveStatus,
  onRetryTranscriptExtraction,
  retryingExtraction,
}: {
  outcomes: MeetingOutcomeData;
  onRefresh: () => void;
  onSaveStatus: (status: "idle" | "saving" | "saved") => void;
  onRetryTranscriptExtraction: () => void;
  retryingExtraction: boolean;
}) {
  const summary = outcomes.summary?.trim();
  const wins = outcomes.wins.filter((item) => item.trim().length > 0);
  const risks = outcomes.risks.filter((item) => item.trim().length > 0);
  const decisions = outcomes.decisions.filter((item) => item.trim().length > 0);
  const hasContent =
    Boolean(summary) ||
    wins.length > 0 ||
    risks.length > 0 ||
    decisions.length > 0 ||
    outcomes.actions.length > 0;

  return (
    <div className={styles.outcomesContainer}>
      <h2 className={styles.outcomesTitle}>Meeting Outcomes</h2>
      {/* Inlined MeetingOutcomes */}
      <div className={styles.outcomesBody}>
        {!hasContent && (
          <>
            <p className={styles.outcomesSummary}>
              Transcript attached, but no structured outcomes were extracted yet.
            </p>
            {outcomes.transcriptPath && (
              <button
                type="button"
                onClick={onRetryTranscriptExtraction}
                disabled={retryingExtraction}
                className={clsx(styles.planSecondaryAction, retryingExtraction && styles.folioBtnDisabled)}
              >
                {retryingExtraction ? "Retrying extraction…" : "Retry extraction"}
              </button>
            )}
          </>
        )}

        {/* Summary */}
        {summary && (
          <p className={styles.outcomesSummary}>{summary}</p>
        )}

        <div className={styles.outcomesGrid}>
          {/* Wins */}
          {wins.length > 0 && (
            <OutcomeSection
              title="Wins"
              icon={<Trophy className={styles.iconSage} />}
              items={wins}
            />
          )}

          {/* Risks */}
          {risks.length > 0 && (
            <OutcomeSection
              title="Risks"
              icon={<AlertTriangle className={styles.iconTerracotta} />}
              items={risks}
            />
          )}

          {/* Decisions */}
          {decisions.length > 0 && (
            <OutcomeSection
              title="Decisions"
              icon={<CircleDot className={styles.iconTurmeric} />}
              items={decisions}
            />
          )}
        </div>

        {/* Actions */}
        {outcomes.actions.length > 0 && (
          <div className={styles.outcomeActionsWrap}>
            <h4 className={styles.outcomeActionsTitle}>Actions</h4>
            <div className={styles.outcomeActionsList}>
              {outcomes.actions.map((action) => (
                <ActionRow
                  key={action.id}
                  variant="outcome"
                  action={action}
                  onComplete={async () => {
                    try {
                      if (action.status === "completed") {
                        await invoke("reopen_action", { id: action.id });
                      } else {
                        await invoke("complete_action", { id: action.id });
                      }
                      onRefresh();
                    } catch (err) { console.error("Failed to toggle action:", err); }
                  }}
                  onAccept={async () => {
                    try { await invoke("accept_proposed_action", { id: action.id }); onRefresh(); }
                    catch (err) { console.error("Failed to accept action:", err); }
                  }}
                  onReject={async () => {
                    try { await invoke("reject_proposed_action", { id: action.id, source: "meeting_detail" }); onRefresh(); }
                    catch (err) { console.error("Failed to reject action:", err); }
                  }}
                  onCyclePriority={async () => {
                    const cycle: Record<string, string> = { P1: "P2", P2: "P3", P3: "P1" };
                    try { await invoke("update_action_priority", { id: action.id, priority: cycle[action.priority] || "P2" }); onRefresh(); }
                    catch (err) { console.error("Failed to update priority:", err); }
                  }}
                />
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function OutcomeSection({
  title,
  icon,
  items,
}: {
  title: string;
  icon: React.ReactNode;
  items: string[];
}) {
  return (
    <div className={styles.outcomeSectionWrap}>
      <h4 className={styles.outcomeSectionTitle}>
        {icon}
        {title}
        <span className={styles.outcomeSectionCount}>({items.length})</span>
      </h4>
      <ul className={styles.outcomeSectionItems}>
        {items.map((item, i) => (
          <li key={i} className={styles.outcomeSectionItem}>
            {item}
          </li>
        ))}
      </ul>
    </div>
  );
}

// OutcomeActionRow consolidated into shared/ActionRow.tsx variant="outcome" (ADR-0084 C1)

// =============================================================================
// Helpers
// =============================================================================

function extractKeyInsight(intelligenceSummary?: string, meetingContext?: string): string | null {
  const source = intelligenceSummary || meetingContext;
  if (!source) return null;

  const firstLine = source.split("\n").find((line) => line.trim().length > 0);
  if (!firstLine) return null;

  const trimmed = firstLine.trim();
  // Extract first sentence
  const sentenceEnd = trimmed.match(/[.!?](\s|$)/);
  if (sentenceEnd && sentenceEnd.index != null) {
    return trimmed.slice(0, sentenceEnd.index + 1).trim();
  }
  // If no sentence-ending punctuation, use the whole first line
  return trimmed;
}

function buildUnifiedAttendees(
  attendeeContext?: AttendeeContext[],
  attendees?: Stakeholder[],
  insights?: StakeholderInsight[],
  signals?: StakeholderSignals,
): UnifiedAttendee[] {
  const byKey = new Map<string, UnifiedAttendee>();

  // Start with attendeeContext (richest data)
  for (let ci = 0; ci < (attendeeContext ?? []).length; ci++) {
    const ctx = attendeeContext![ci];
    const key = normalizePersonKey(ctx.name);
    byKey.set(key, {
      name: ctx.name,
      personId: ctx.personId,
      role: ctx.role,
      organization: ctx.organization,
      temperature: ctx.temperature,
      meetingCount: ctx.meetingCount,
      lastSeen: ctx.lastSeen,
      notes: ctx.notes,
      assessmentFieldPath: `attendeeContext[${ci}].assessment`,
    });
  }

  // Merge attendees (basic stakeholder data)
  for (const a of attendees ?? []) {
    const key = normalizePersonKey(a.name);
    const existing = byKey.get(key);
    if (existing) {
      if (!existing.role && a.role) existing.role = a.role;
    } else {
      byKey.set(key, {
        name: a.name,
        role: a.role,
      });
    }
  }

  // Merge stakeholder insights (assessment, engagement)
  for (let si = 0; si < (insights ?? []).length; si++) {
    const insight = insights![si];
    const key = normalizePersonKey(insight.name);
    const existing = byKey.get(key);
    if (existing) {
      if (insight.assessment) {
        existing.assessment = insight.assessment;
        existing.assessmentFieldPath = `stakeholderInsights[${si}].assessment`;
      }
      if (insight.engagement) existing.engagement = insight.engagement;
      if (!existing.role && insight.role) existing.role = insight.role;
    }
    // Don't add non-attendees here — they go to extended stakeholders
  }

  // Merge relationship signals into all attendees
  if (signals) {
    for (const entry of byKey.values()) {
      if (!entry.temperature && signals.temperature) entry.temperature = signals.temperature;
    }
  }

  return Array.from(byKey.values());
}

function getLifecycleForDisplay(data: FullMeetingPrep): string | null {
  const lifecycle = findSnapshotValue(data.accountSnapshot, ["lifecycle"])
    ?? findQuickContextValue(data.quickContext, "lifecycle");
  if (!lifecycle) return null;

  const clean = sanitizeInlineText(lifecycle).replace(/[_-]/g, " ").trim();
  if (!clean) return null;
  return clean;
}


function findSnapshotValue(
  items: AccountSnapshotItem[] | undefined,
  labels: string[],
): string | undefined {
  if (!items || items.length === 0) return undefined;
  const target = new Set(labels.map((l) => normalizePersonKey(l)));
  return items.find((item) => target.has(normalizePersonKey(item.label)))?.value;
}

function findQuickContextValue(
  quickContext: [string, string][] | undefined,
  key: string,
): string | undefined {
  if (!quickContext || quickContext.length === 0) return undefined;
  const found = quickContext.find(([label]) => normalizePersonKey(label) === normalizePersonKey(key));
  return found?.[1];
}


function normalizePersonKey(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, " ");
}

function sanitizeInlineText(value: string): string {
  return value
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/[*_`>#]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function splitInlineSourceTail(value: string): { text: string; source?: string } {
  const sourceMatch = value.match(/(?:^|\s)[_*]*\(?\s*source:\s*([^)]+?)\s*\)?[_*\s]*$/i);
  if (!sourceMatch || sourceMatch.index == null) {
    return { text: value.trim() };
  }
  return {
    text: value.slice(0, sourceMatch.index).trim(),
    source: sanitizeInlineText(sourceMatch[1]),
  };
}

function cleanPrepLine(value: string): string {
  const { text } = splitInlineSourceTail(value);
  let raw = text.trim();

  return sanitizeInlineText(raw)
    .replace(/^recent\s+win:\s*/i, "")
    .trim();
}

function truncateText(value: string, maxChars: number): string {
  if (value.length <= maxChars) return value;
  return `${value.slice(0, maxChars - 1).trim()}…`;
}

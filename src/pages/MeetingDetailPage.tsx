import { useState, useEffect, useCallback, useMemo } from "react";
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
  ActionWithContext,
  SourceReference,
  AttendeeContext,
  AccountSnapshotItem,
  MeetingOutcomeData,
  MeetingIntelligence,
  CalendarEvent,
  StakeholderInsight,
  ApplyPrepPrefillResult,
  DbAction,
  LinkedEntity,
} from "@/types";
import { parseDate, formatRelativeDateLong } from "@/lib/utils";
import { getPrimaryEntityName } from "@/lib/entity-helpers";
import { CopyButton } from "@/components/ui/copy-button";
import { MeetingEntityChips } from "@/components/ui/meeting-entity-chips";

import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import {
  AlignLeft,
  AlertTriangle,
  BookOpen,
  Check,
  ChevronRight,
  CircleDot,
  Clock,
  Loader2,
  Paperclip,
  RefreshCw,
  Target,
  Trophy,
  Users,
} from "lucide-react";
import styles from "./meeting-intel.module.css";

// ── Shared style fragments ──

const monoOverline: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.2em",
  color: "var(--color-text-tertiary)",
};

const chapterHeadingStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.12em",
  color: "var(--color-text-tertiary)",
};

const editorialBtn: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 12,
  fontWeight: 500,
  letterSpacing: "0.04em",
  padding: "6px 14px",
  border: "1px solid var(--color-rule-light)",
  borderRadius: 4,
  background: "transparent",
  color: "var(--color-text-secondary)",
  cursor: "pointer",
};

const folioBtn: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 600,
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "var(--color-text-secondary)",
  background: "none",
  border: "1px solid var(--color-rule-light)",
  borderRadius: 4,
  padding: "2px 10px",
  cursor: "pointer",
};

const bulletDot = (color: string): React.CSSProperties => ({
  width: 6,
  height: 6,
  borderRadius: "50%",
  background: color,
  flexShrink: 0,
  marginTop: 7,
});

// ── Chapter Nav definitions ──

const CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Brief", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "risks", label: "Risks", icon: <AlertTriangle size={18} strokeWidth={1.5} /> },
  { id: "the-room", label: "The Room", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "your-plan", label: "Your Plan", icon: <Target size={18} strokeWidth={1.5} /> },
  { id: "deep-dive", label: "Deep Dive", icon: <BookOpen size={18} strokeWidth={1.5} /> },
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
  meetingCount?: number;
  lastSeen?: string;
  notes?: string;
}

export default function MeetingDetailPage() {
  const { meetingId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [data, setData] = useState<FullMeetingPrep | null>(null);
  const [outcomes, setOutcomes] = useState<MeetingOutcomeData | null>(null);
  const [canEditUserLayer, setCanEditUserLayer] = useState(false);
  const [meetingMeta, setMeetingMeta] = useState<MeetingIntelligence["meeting"] | null>(null);
  const [linkedEntities, setLinkedEntities] = useState<LinkedEntity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Transcript attach
  const [attaching, setAttaching] = useState(false);
  const draft = useAgendaDraft({ onError: setError });
  const [prefillNotice, setPrefillNotice] = useState(false);
  const [prefilling, setPrefilling] = useState(false);

  // Save status for folio bar
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");

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
      setLoading(true);
      setError(null);
      const intel = await invoke<MeetingIntelligence>("get_meeting_intelligence", {
        meetingId,
      });
      setMeetingMeta(intel.meeting);
      setOutcomes(intel.outcomes ?? null);
      setCanEditUserLayer(intel.canEditUserLayer);
      setLinkedEntities(intel.linkedEntities ?? []);
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
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
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
        err instanceof Error ? err.message : "Failed to prefill prep context"
      );
    } finally {
      setPrefilling(false);
    }
  }, [meetingId, canEditUserLayer, data, loadMeetingIntelligence]);

  useEffect(() => {
    loadMeetingIntelligence();
  }, [loadMeetingIntelligence]);

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

  // Register magazine shell with chapter nav + folio actions
  const shellConfig = useMemo(() => ({
    folioLabel: "Intelligence Report",
    atmosphereColor: "turmeric" as const,
    activePage: "today" as const,
    backLink: { label: "Today", onClick: () => navigate({ to: "/" }) },
    chapters: CHAPTERS,
    folioStatusText: saveStatus === "saving" ? "Saving…" : saveStatus === "saved" ? "✓ Saved" : undefined,
    folioActions: data ? (
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {isEditable && (
          <button
            onClick={handlePrefillFromContext}
            disabled={prefilling}
            style={{ ...folioBtn, opacity: prefilling ? 0.5 : 1, cursor: prefilling ? "not-allowed" : "pointer" }}
          >
            {prefilling ? "Prefilling…" : "Prefill"}
          </button>
        )}
        <button onClick={handleDraftAgendaMessage} style={folioBtn}>
          Draft Agenda
        </button>
        <button
          onClick={handleAttachTranscript}
          disabled={attaching}
          style={{ ...folioBtn, display: "inline-flex", alignItems: "center", gap: 4, opacity: attaching ? 0.5 : 1, cursor: attaching ? "not-allowed" : "pointer" }}
        >
          {attaching ? <Loader2 style={{ width: 10, height: 10, animation: "spin 1s linear infinite" }} /> : <Paperclip style={{ width: 10, height: 10 }} />}
          {attaching ? "Processing…" : "Transcript"}
        </button>
        <button
          onClick={() => loadMeetingIntelligence()}
          style={{ ...folioBtn, display: "inline-flex", alignItems: "center", gap: 4 }}
        >
          <RefreshCw style={{ width: 10, height: 10 }} />
          Refresh
        </button>
      </div>
    ) : undefined,
  }), [navigate, saveStatus, data, isEditable, prefilling, attaching, handlePrefillFromContext, handleDraftAgendaMessage, handleAttachTranscript, loadMeetingIntelligence]);
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
      <div style={{ maxWidth: 960, margin: "0 auto", padding: "48px 0 80px" }}>
        <div style={{ textAlign: "center", padding: "60px 0" }}>
          <Clock
            style={{
              width: 32,
              height: 32,
              color: "var(--color-text-tertiary)",
              opacity: 0.5,
              margin: "0 auto 16px",
              display: "block",
            }}
          />
          <h2
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontWeight: 600,
              color: "var(--color-text-primary)",
              margin: "0 0 8px",
            }}
          >
            Prep not ready yet
          </h2>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-tertiary)",
              margin: 0,
            }}
          >
            Meeting context will appear here after the daily briefing runs.
          </p>
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

  // Derived data
  const topRisks = [
    ...((data.entityRisks ?? []).map((risk) => risk.text)),
    ...(data.risks ?? []),
  ]
    .map((risk) => sanitizeInlineText(risk))
    .filter((risk) => risk.length > 0)
    .slice(0, 3);
  const lifecycle = getLifecycleForDisplay(data);
  const agendaItems = (data.proposedAgenda ?? [])
    .map((item) => ({
      ...item,
      topic: cleanPrepLine(item.topic),
      why: item.why ? cleanPrepLine(item.why) : undefined,
    }))
    .filter((item) => item.topic.length > 0);
  const { wins: recentWins } = deriveRecentWins(data);
  const agendaNonWinItems = agendaItems.filter((item) => item.source !== "talking_point");
  const agendaDisplayItems = agendaNonWinItems.length > 0 ? agendaNonWinItems : agendaItems;
  const calendarNotes = normalizeCalendarNotes(data.calendarNotes);
  const agendaTopics = new Set(agendaDisplayItems.map((item) => normalizePersonKey(item.topic)));
  const recentWinsForDisplay = recentWins.filter(
    (win) => !agendaTopics.has(normalizePersonKey(win))
  );

  // Build unified attendees
  const unifiedAttendees = buildUnifiedAttendees(
    data.attendeeContext,
    data.attendees,
    data.stakeholderInsights,
    data.stakeholderSignals,
  );
  const extendedStakeholderInsights = (data.stakeholderInsights ?? []).filter(
    (person) => {
      const attendeeNames = new Set<string>([
        ...(data.attendeeContext ?? []).map((p) => normalizePersonKey(p.name)),
        ...(data.attendees ?? []).map((p) => normalizePersonKey(p.name)),
      ]);
      return !attendeeNames.has(normalizePersonKey(person.name));
    }
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
  const hasDeepDive = Boolean(
    recentWinsForDisplay.length > 0 ||
    (data.openItems && data.openItems.length > 0) ||
    (data.recentEmailSignals && data.recentEmailSignals.filter((s) => (s.confidence ?? 0) >= 0.6).length > 0) ||
    hasReferenceContent(data) ||
    (data.sinceLast?.length ?? 0) > 0 ||
    (data.strategicPrograms?.length ?? 0) > 0
  );

  return (
    <>
      <div style={{ maxWidth: 960, margin: "0 auto", padding: "0 0 80px" }}>
        {/* Outcomes always at top when present */}
        {outcomes && (
          <>
            <div style={{ paddingTop: 80 }}>
              <OutcomesSection outcomes={outcomes} onRefresh={loadMeetingIntelligence} onSaveStatus={setSaveStatus} />
            </div>
            <div style={{ height: 1, background: "rgba(30, 37, 48, 0.08)", margin: "48px 0" }} />
            <p style={{ ...chapterHeadingStyle, marginBottom: 20 }}>
              {isPastMeeting ? "Pre-Meeting Context" : "Meeting Prep"}
            </p>
          </>
        )}

        {/* Past meeting: prompt to attach transcript */}
        {isPastMeeting && (
          <div
            style={{
              border: "1px dashed var(--color-rule-light)",
              padding: "20px 24px",
              marginTop: 80,
              marginBottom: 32,
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div>
              <p
                style={{
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                  margin: "0 0 4px",
                }}
              >
                {outcomes ? "Update outcomes" : "No outcomes captured yet"}
              </p>
              <p
                style={{
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                  margin: 0,
                }}
              >
                {outcomes
                  ? "Attach a new transcript to re-process meeting outcomes."
                  : "Attach a transcript or manually capture meeting outcomes."}
              </p>
              {meetingId && (
                <div style={{ marginTop: 8 }}>
                  <QuillSyncBadge meetingId={meetingId} />
                </div>
              )}
            </div>
            <button
              onClick={handleAttachTranscript}
              disabled={attaching}
              style={{
                ...editorialBtn,
                display: "inline-flex",
                alignItems: "center",
                gap: 8,
                opacity: attaching ? 0.6 : 1,
              }}
            >
              {attaching ? (
                <Loader2 style={{ width: 14, height: 14, animation: "spin 1s linear infinite" }} />
              ) : (
                <Paperclip style={{ width: 14, height: 14 }} />
              )}
              {attaching ? "Processing..." : "Attach Transcript"}
            </button>
          </div>
        )}

        {!hasAnyContent && !outcomes && (
          <div style={{ textAlign: "center", padding: "60px 0" }}>
            <Clock
              style={{
                width: 32,
                height: 32,
                color: "var(--color-text-tertiary)",
                opacity: 0.5,
                margin: "0 auto 16px",
                display: "block",
              }}
            />
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 18,
                fontWeight: 500,
                color: "var(--color-text-primary)",
                margin: "0 0 8px",
              }}
            >
              Prep is being generated
            </p>
            <p
              style={{
                fontSize: 14,
                color: "var(--color-text-tertiary)",
                margin: 0,
              }}
            >
              Meeting context will appear here once AI enrichment completes.
            </p>
          </div>
        )}

        {(hasAnyContent || outcomes) && (
          <div style={isPastMeeting && outcomes ? { opacity: 0.7 } : undefined}>

            {/* ================================================================
                ACT I: "Ground Me" — visible immediately, NO editorial-reveal
               ================================================================ */}
            <section id="headline" style={{ paddingTop: 80, scrollMarginTop: 60 }}>
              {/* Time-aware urgency banner */}
              {minutesUntilMeeting != null && (
                <div
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 8,
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    fontWeight: 600,
                    letterSpacing: "0.06em",
                    color: minutesUntilMeeting <= 15 ? "var(--color-spice-terracotta)" : "var(--color-spice-turmeric)",
                    marginBottom: 16,
                  }}
                >
                  <Clock style={{ width: 13, height: 13 }} />
                  Meeting starts in {minutesUntilMeeting} minute{minutesUntilMeeting !== 1 ? "s" : ""}
                </div>
              )}

              {/* Kicker */}
              <p style={monoOverline}>
                Meeting Intelligence Report
              </p>

              {/* Title — 76px editorial hero scale */}
              <h1 className={styles.heroTitle}>
                {data.title}
              </h1>
              {lifecycle && (
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 12 }}>
                  <span style={{ ...bulletDot("var(--color-spice-turmeric)"), marginTop: 0 }} />
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      fontWeight: 500,
                      letterSpacing: "0.06em",
                      color: "var(--color-spice-turmeric)",
                    }}
                  >
                    {lifecycle}
                  </span>
                </div>
              )}

              {/* Metadata line */}
              <p
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  letterSpacing: "0.04em",
                  color: "var(--color-text-tertiary)",
                  margin: "8px 0 0",
                }}
              >
                {data.timeRange}
                {meetingType && <> &middot; {meetingType}</>}
                {getPrimaryEntityName(linkedEntities) && (
                  <> &middot; {getPrimaryEntityName(linkedEntities)}</>
                )}
              </p>

              {/* Entity chips */}
              {meetingId && meetingMeta && (
                <div style={{ marginTop: 10 }}>
                  <MeetingEntityChips
                    meetingId={meetingId}
                    meetingTitle={meetingMeta.title}
                    meetingStartTime={meetingMeta.startTime ?? new Date().toISOString()}
                    meetingType={meetingMeta.meetingType ?? "internal"}
                    linkedEntities={linkedEntities}
                    onEntitiesChanged={() => loadMeetingIntelligence()}
                  />
                </div>
              )}

              {/* The Key Insight — pull quote style */}
              {keyInsight && (
                <blockquote
                  style={{
                    marginTop: 28,
                    marginBottom: 0,
                    marginLeft: 0,
                    marginRight: 0,
                    borderLeft: "3px solid var(--color-spice-turmeric)",
                    paddingLeft: 24,
                    paddingTop: 16,
                    paddingBottom: 16,
                  }}
                >
                  <p
                    style={{
                      fontFamily: "var(--font-serif)",
                      fontSize: 28,
                      fontStyle: "italic",
                      fontWeight: 300,
                      lineHeight: 1.45,
                      color: "var(--color-text-primary)",
                      margin: 0,
                      maxWidth: 620,
                    }}
                  >
                    {keyInsight}
                  </p>
                </blockquote>
              )}
              {!keyInsight && (
                <p
                  style={{
                    marginTop: 28,
                    fontSize: 14,
                    color: "var(--color-text-tertiary)",
                  }}
                >
                  Intelligence builds as you meet with this account.
                </p>
              )}

              {/* Entity Readiness — "Before This Meeting" checklist */}
              {data.entityReadiness && data.entityReadiness.length > 0 && (
                <div
                  style={{
                    marginTop: 32,
                    borderLeft: "3px solid var(--color-spice-turmeric)",
                    paddingLeft: 20,
                    paddingTop: 12,
                    paddingBottom: 12,
                  }}
                >
                  <p
                    style={{
                      ...chapterHeadingStyle,
                      color: "var(--color-spice-turmeric)",
                      marginBottom: 12,
                    }}
                  >
                    Before This Meeting
                  </p>
                  <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 16 }}>
                    {data.entityReadiness.slice(0, 4).map((item, i) => (
                      <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10, fontSize: 14, color: "var(--color-text-primary)" }}>
                        <span style={bulletDot("rgba(201, 162, 39, 0.6)")} />
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
              <section id="risks" className="editorial-reveal" style={{ paddingTop: 80, scrollMarginTop: 60 }}>
                <ChapterHeading title="The Risks" />
                <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
                  {topRisks.map((risk, i) => {
                    const isHighUrgency = topRiskUrgencies[i]?.urgency === "high";
                    return i === 0 ? (
                      /* Featured risk — serif italic, terracotta border, pulse if high urgency */
                      <blockquote
                        key={i}
                        className={isHighUrgency ? "risk-pulse-once" : undefined}
                        style={{
                          borderLeft: "3px solid var(--color-spice-terracotta)",
                          paddingLeft: 24,
                          paddingTop: 16,
                          paddingBottom: 16,
                          margin: 0,
                        }}
                      >
                        <p
                          style={{
                            fontFamily: "var(--font-serif)",
                            fontSize: 20,
                            fontStyle: "italic",
                            fontWeight: 400,
                            lineHeight: 1.5,
                            color: "var(--color-text-primary)",
                            margin: 0,
                          }}
                        >
                          {risk}
                        </p>
                      </blockquote>
                    ) : (
                      /* Subordinate risks — body scale, light rules, generous spacing */
                      <div
                        key={i}
                        className={isHighUrgency ? "risk-pulse-once" : undefined}
                        style={{
                          borderTop: "1px solid rgba(30, 37, 48, 0.04)",
                          borderLeft: isHighUrgency ? "3px solid var(--color-spice-terracotta)" : "none",
                          paddingTop: 16,
                          paddingLeft: isHighUrgency ? 16 : 0,
                        }}
                      >
                        <p
                          style={{
                            fontSize: 14,
                            lineHeight: 1.65,
                            color: "var(--color-text-primary)",
                            margin: 0,
                          }}
                        >
                          {risk}
                        </p>
                      </div>
                    );
                  })}
                </div>
              </section>
            )}

            {/* Chapter: The Room */}
            {hasRoom && (
              <section id="the-room" className="editorial-reveal" style={{ paddingTop: 80, scrollMarginTop: 60 }}>
                <ChapterHeading title="The Room" />
                <UnifiedAttendeeList
                  attendees={unifiedAttendees}
                  isEditable={isEditable}
                  initialHiddenNames={hiddenAttendees}
                  meetingId={meetingId ?? undefined}
                  onSaveStatus={setSaveStatus}
                />
              </section>
            )}

            {/* Chapter: Your Plan */}
            {hasPlan && (
              <section id="your-plan" className="editorial-reveal" style={{ paddingTop: 80, scrollMarginTop: 60 }}>
                <ChapterHeading title="Your Plan" />

                {meetingId && prefillNotice && (
                  <div
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 12,
                      color: "var(--color-spice-turmeric)",
                      borderLeft: "3px solid var(--color-spice-turmeric)",
                      paddingLeft: 12,
                      paddingTop: 6,
                      paddingBottom: 6,
                      marginBottom: 16,
                    }}
                  >
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
              </section>
            )}

            {/* ================================================================
                "You're Briefed" — FinisMarker
               ================================================================ */}
            <div style={{ paddingTop: 80 }}>
              <FinisMarker />
            </div>

            {/* ================================================================
                ACT III: "Go Deeper" — supporting intelligence
               ================================================================ */}
            {hasDeepDive && (
              <section id="deep-dive" className="editorial-reveal-slow" style={{ paddingTop: 80, scrollMarginTop: 60 }}>
                <div style={{ height: 1, background: "rgba(30, 37, 48, 0.08)", marginBottom: 16 }} />
                <p style={monoOverline}>Supporting Intelligence</p>

                <div style={{ display: "flex", flexDirection: "column", gap: 48, marginTop: 48 }}>
                  {/* Recent Wins — sage accent */}
                  {recentWinsForDisplay.length > 0 && (
                    <div>
                      <SectionLabel
                        label="Recent Wins"
                        labelColor="var(--color-garden-sage)"
                      />
                      <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 20 }}>
                        {recentWinsForDisplay.slice(0, 4).map((win, i) => (
                          <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 14 }}>
                            <span style={bulletDot("rgba(126, 170, 123, 0.7)")} />
                            <span style={{ lineHeight: 1.55, color: "var(--color-text-primary)" }}>{win}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {/* Open Items */}
                  {data.openItems && data.openItems.length > 0 && (
                    <div>
                      <SectionLabel
                        label="Open Items"
                        copyText={formatOpenItems(data.openItems)}
                        copyLabel="open items"
                      />
                      <div style={{ display: "flex", flexDirection: "column", gap: 16, marginTop: 16 }}>
                        {data.openItems.map((item, i) => (
                          <ActionItem key={i} action={item} />
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Email Signals — compact, filtered by confidence */}
                  {data.recentEmailSignals && data.recentEmailSignals.filter((s) => (s.confidence ?? 0) >= 0.6).length > 0 && (
                    <div>
                      <SectionLabel
                        label="Email Signals"
                        labelColor="var(--color-spice-turmeric)"
                      />
                      <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 20 }}>
                        {data.recentEmailSignals.filter((s) => (s.confidence ?? 0) >= 0.6).slice(0, 4).map((signal, i) => (
                          <li key={`${signal.id ?? i}-${signal.signalType}`} style={{ fontSize: 14, borderBottom: "1px solid rgba(30, 37, 48, 0.04)", paddingBottom: 10 }}>
                            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                              <span
                                style={{
                                  fontFamily: "var(--font-mono)",
                                  fontSize: 10,
                                  fontWeight: 500,
                                  textTransform: "uppercase",
                                  letterSpacing: "0.06em",
                                  color: "var(--color-spice-turmeric)",
                                }}
                              >
                                {signal.signalType}
                              </span>
                              {signal.detectedAt && (
                                <span
                                  style={{
                                    fontFamily: "var(--font-mono)",
                                    fontSize: 10,
                                    color: "var(--color-text-tertiary)",
                                  }}
                                >
                                  {formatRelativeDateLong(signal.detectedAt)}
                                </span>
                              )}
                            </div>
                            <p style={{ marginTop: 4, marginBottom: 0, lineHeight: 1.55, color: "var(--color-text-primary)" }}>
                              {signal.signalText}
                            </p>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {/* Appendix toggle */}
                  <AppendixSection
                    data={data}
                    extendedStakeholderInsights={extendedStakeholderInsights}
                  />
                </div>
              </section>
            )}

            {/* Finis */}
            <div style={{ paddingTop: 48 }}>
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
// SectionLabel (chapter heading pattern)
// =============================================================================

function SectionLabel({
  label,
  labelColor,
  copyText,
  copyLabel,
}: {
  label: string;
  labelColor?: string;
  copyText?: string;
  copyLabel?: string;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <span
        style={{
          ...chapterHeadingStyle,
          color: labelColor || "var(--color-text-tertiary)",
        }}
      >
        {label}
      </span>
      {copyText && (
        <CopyButton text={copyText} label={copyLabel} />
      )}
    </div>
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
}: {
  attendees: UnifiedAttendee[];
  isEditable?: boolean;
  initialHiddenNames?: string[];
  meetingId?: string;
  onSaveStatus?: (status: "idle" | "saving" | "saved") => void;
}) {
  const [showAll, setShowAll] = useState(false);
  const [hiddenNames, setHiddenNames] = useState<Set<string>>(
    new Set((initialHiddenNames ?? []).map(normalizePersonKey))
  );
  const filtered = attendees.filter((p) => !hiddenNames.has(normalizePersonKey(p.name)));
  const visible = showAll ? filtered : filtered.slice(0, 4);
  const remaining = filtered.length - 4;

  const tempColorMap: Record<string, string> = {
    hot: "var(--color-garden-sage)",
    warm: "var(--color-spice-turmeric)",
    cool: "var(--color-text-tertiary)",
    cold: "var(--color-spice-terracotta)",
  };

  const engagementColor: Record<string, string> = {
    champion: "var(--color-garden-sage)",
    detractor: "var(--color-spice-terracotta)",
    neutral: "var(--color-text-tertiary)",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
      {visible.map((person, i) => {
        const tempColor = tempColorMap[person.temperature ?? ""] ?? "var(--color-text-tertiary)";
        const isNew = person.meetingCount === 0;
        const isCold = person.temperature === "cold";
        const circleColor = isCold
          ? { bg: "rgba(196, 101, 74, 0.1)", fg: "var(--color-spice-terracotta)" }
          : isNew
          ? { bg: "rgba(126, 170, 123, 0.1)", fg: "var(--color-garden-sage)" }
          : { bg: "rgba(201, 162, 39, 0.1)", fg: "var(--color-spice-turmeric)" };

        const inner = (
          <div
            style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 12,
              padding: "12px 0",
              borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
            }}
          >
            {/* Avatar */}
            <div
              style={{
                width: 32,
                height: 32,
                borderRadius: "50%",
                background: circleColor.bg,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 14,
                fontWeight: 500,
                color: circleColor.fg,
                flexShrink: 0,
              }}
            >
              {person.name.charAt(0)}
            </div>

            <div style={{ flex: 1, minWidth: 0 }}>
              {/* Name + role + temperature dot + engagement badge */}
              <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                <span className="attendee-tooltip-wrap">
                  <p style={{ fontWeight: 500, color: "var(--color-text-primary)", margin: 0, fontSize: 14, cursor: "default" }}>
                    {person.name}
                  </p>
                  {/* Hover tooltip — last meeting + assessment */}
                  {(person.lastSeen || person.assessment) && (
                    <span className="attendee-tooltip">
                      {person.lastSeen && (
                        <span style={{ display: "block", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", marginBottom: person.assessment ? 6 : 0 }}>
                          Last met {formatRelativeDateLong(person.lastSeen)}
                          {person.meetingCount != null && person.meetingCount > 0 && ` \u00b7 ${person.meetingCount} meeting${person.meetingCount !== 1 ? "s" : ""}`}
                        </span>
                      )}
                      {person.assessment && (
                        <span style={{ display: "block", fontFamily: "var(--font-serif)", fontSize: 13, fontStyle: "italic", lineHeight: 1.45, color: "var(--color-text-primary)" }}>
                          {truncateText(sanitizeInlineText(person.assessment), 140)}
                        </span>
                      )}
                    </span>
                  )}
                </span>
                {person.role && (
                  <span style={{ fontSize: 13, color: "var(--color-text-tertiary)" }}>
                    {sanitizeInlineText(person.role)}
                  </span>
                )}
                {person.temperature && (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <span style={{ width: 6, height: 6, borderRadius: "50%", background: tempColor, flexShrink: 0 }} />
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        fontWeight: 500,
                        textTransform: "capitalize",
                        color: tempColor,
                      }}
                    >
                      {person.temperature}
                    </span>
                  </span>
                )}
                {person.engagement && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 500,
                      textTransform: "capitalize",
                      color: engagementColor[person.engagement] ?? "var(--color-text-tertiary)",
                      border: "1px solid var(--color-rule-light)",
                      borderRadius: 3,
                      padding: "1px 6px",
                    }}
                  >
                    {person.engagement}
                  </span>
                )}
                {isNew && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      fontWeight: 500,
                      color: "var(--color-garden-sage)",
                    }}
                  >
                    New contact
                  </span>
                )}
              </div>

              {/* Assessment — the killer insight, serif italic, prominent */}
              {person.assessment && (
                <p
                  style={{
                    marginTop: 6,
                    marginBottom: 0,
                    fontFamily: "var(--font-serif)",
                    fontSize: 14,
                    fontStyle: "italic",
                    fontWeight: 400,
                    lineHeight: 1.55,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {truncateText(sanitizeInlineText(person.assessment), 200)}
                </p>
              )}

              {/* Metadata line */}
              <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12, marginTop: 4 }}>
                {person.organization && (
                  <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
                    {person.organization}
                  </span>
                )}
                {person.meetingCount != null && person.meetingCount > 0 && (
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                    {person.meetingCount} meeting{person.meetingCount !== 1 ? "s" : ""}
                  </span>
                )}
                {person.lastSeen && (
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: isCold ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
                    }}
                  >
                    Last seen {formatRelativeDateLong(person.lastSeen)}
                  </span>
                )}
              </div>

              {person.notes && (
                <p
                  style={{
                    marginTop: 4,
                    marginBottom: 0,
                    fontSize: 12,
                    color: "var(--color-text-tertiary)",
                    fontStyle: "italic",
                  }}
                >
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
            style={{ textDecoration: "none", color: "inherit", flex: 1, minWidth: 0 }}
          >
            {inner}
          </Link>
        ) : (
          <div style={{ flex: 1, minWidth: 0 }}>{inner}</div>
        );

        return (
          <div key={i} style={{ display: "flex", alignItems: "flex-start" }}>
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
                      onSaveStatus?.("idle");
                    }
                  }
                }}
                style={{
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  fontSize: 16,
                  lineHeight: 1,
                  color: "var(--color-text-tertiary)",
                  padding: "12px 4px 0",
                  opacity: 0.35,
                  flexShrink: 0,
                }}
              >
                &times;
              </button>
            )}
          </div>
        );
      })}

      {!showAll && remaining > 0 && (
        <button
          onClick={() => setShowAll(true)}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "12px 0",
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            textAlign: "left",
          }}
        >
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
      <p style={{ fontSize: 14, color: "var(--color-text-tertiary)", fontStyle: "italic" }}>
        No agenda prepared yet.
      </p>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
      {/* Unified numbered list */}
      {allItems.length > 0 && (
        <ol style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 0 }}>
          {allItems.map((item, i) => (
            <li
              key={`${item.isUser ? "u" : "p"}-${i}`}
              style={{
                display: "flex",
                alignItems: "flex-start",
                gap: 12,
                borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
                padding: "12px 0",
              }}
            >
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  fontWeight: 600,
                  color: "var(--color-spice-turmeric)",
                  width: 24,
                  textAlign: "right",
                  flexShrink: 0,
                  paddingTop: 1,
                }}
              >
                {i + 1}
              </span>
              <div style={{ flex: 1, minWidth: 0 }}>
                {editingIndex === i && isEditable ? (
                  <div
                    style={{ display: "flex", flexDirection: "column", gap: 4 }}
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
                      style={{
                        width: "100%",
                        border: "none",
                        borderBottom: "1px solid var(--color-spice-turmeric)",
                        background: "transparent",
                        padding: "2px 0",
                        fontSize: 14,
                        fontWeight: 500,
                        color: "var(--color-text-primary)",
                        fontFamily: "var(--font-sans)",
                        outline: "none",
                      }}
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
                      style={{
                        width: "100%",
                        border: "none",
                        borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
                        background: "transparent",
                        padding: "2px 0",
                        fontSize: 13,
                        color: "var(--color-text-tertiary)",
                        fontFamily: "var(--font-sans)",
                        outline: "none",
                      }}
                    />
                  </div>
                ) : (
                  <>
                    <p
                      onClick={() => startEditing(i)}
                      style={{
                        fontSize: 14,
                        fontWeight: 500,
                        lineHeight: 1.4,
                        margin: 0,
                        color: "var(--color-text-primary)",
                        cursor: isEditable ? "text" : "default",
                      }}
                    >
                      {item.topic}
                    </p>
                    {item.why && (
                      <p
                        onClick={() => startEditing(i, "why")}
                        style={{
                          fontSize: 13,
                          color: "var(--color-text-tertiary)",
                          marginTop: 4,
                          marginBottom: 0,
                          lineHeight: 1.5,
                          cursor: isEditable ? "text" : "default",
                        }}
                      >
                        {item.why}
                      </p>
                    )}
                  </>
                )}
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
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
                    style={{
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      fontSize: 16,
                      lineHeight: 1,
                      color: "var(--color-text-tertiary)",
                      padding: "0 4px",
                      opacity: 0.5,
                    }}
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
        <div style={{ display: "flex", gap: 12, paddingTop: 12, alignItems: "flex-start" }}>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              fontWeight: 600,
              color: "rgba(201, 162, 39, 0.3)",
              width: 24,
              textAlign: "right",
              flexShrink: 0,
              paddingTop: 5,
            }}
          >
            {allItems.length + 1}
          </span>
          <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
            <input
              value={newItem}
              onChange={(e) => setNewItem(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addItem()}
              placeholder="Add agenda item..."
              style={{
                width: "100%",
                border: "none",
                borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
                background: "transparent",
                padding: "4px 0",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-text-primary)",
                fontFamily: "var(--font-sans)",
                outline: "none",
              }}
            />
            {newItem.trim() && (
              <input
                value={newItemWhy}
                onChange={(e) => setNewItemWhy(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addItem()}
                placeholder="Why this matters..."
                style={{
                  width: "100%",
                  border: "none",
                  borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
                  background: "transparent",
                  padding: "4px 0",
                  fontSize: 13,
                  color: "var(--color-text-tertiary)",
                  fontFamily: "var(--font-sans)",
                  outline: "none",
                }}
              />
            )}
          </div>
        </div>
      )}

      {/* Calendar description — collapsed toggle */}
      {calendarNotes && (
        <details style={{ marginTop: 24 }}>
          <summary
            style={{
              ...chapterHeadingStyle,
              cursor: "pointer",
              userSelect: "none",
              listStyle: "none",
              display: "flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <ChevronRight
              style={{
                width: 12,
                height: 12,
                transition: "transform 0.2s",
              }}
              className={styles.detailsChevron}
            />
            Calendar Description
          </summary>
          <p
            style={{
              marginTop: 12,
              marginBottom: 0,
              whiteSpace: "pre-wrap",
              fontSize: 14,
              color: "var(--color-text-tertiary)",
              lineHeight: 1.65,
              paddingLeft: 18,
            }}
          >
            {calendarNotes}
          </p>
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
}: {
  outcomes: MeetingOutcomeData;
  onRefresh: () => void;
  onSaveStatus: (status: "idle" | "saving" | "saved") => void;
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          fontWeight: 600,
          color: "var(--color-text-primary)",
          margin: 0,
        }}
      >
        Meeting Outcomes
      </h2>
      {/* Inlined MeetingOutcomes */}
      <div style={{ display: "flex", flexDirection: "column", gap: 16, fontSize: 14 }}>
        {/* Summary */}
        {outcomes.summary && (
          <p style={{ color: "var(--color-text-tertiary)", margin: 0, lineHeight: 1.65 }}>{outcomes.summary}</p>
        )}

        <div style={{ display: "grid", gap: 20, gridTemplateColumns: "repeat(2, 1fr)" }}>
          {/* Wins */}
          {outcomes.wins.length > 0 && (
            <OutcomeSection
              title="Wins"
              icon={<Trophy style={{ width: 13, height: 13, color: "var(--color-garden-sage)" }} />}
              items={outcomes.wins}
            />
          )}

          {/* Risks */}
          {outcomes.risks.length > 0 && (
            <OutcomeSection
              title="Risks"
              icon={<AlertTriangle style={{ width: 13, height: 13, color: "var(--color-spice-terracotta)" }} />}
              items={outcomes.risks}
            />
          )}

          {/* Decisions */}
          {outcomes.decisions.length > 0 && (
            <OutcomeSection
              title="Decisions"
              icon={<CircleDot style={{ width: 13, height: 13, color: "var(--color-spice-turmeric)" }} />}
              items={outcomes.decisions}
            />
          )}
        </div>

        {/* Actions */}
        {outcomes.actions.length > 0 && (
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <h4
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              Actions
            </h4>
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {outcomes.actions.map((action) => (
                <OutcomeActionRow
                  key={action.id}
                  action={action}
                  onRefresh={onRefresh}
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
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <h4
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          fontWeight: 500,
          color: "var(--color-text-primary)",
          margin: 0,
        }}
      >
        {icon}
        {title}
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            fontWeight: 400,
            color: "var(--color-text-tertiary)",
          }}
        >
          ({items.length})
        </span>
      </h4>
      <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 4 }}>
        {items.map((item, i) => (
          <li
            key={i}
            style={{
              fontSize: 14,
              color: "var(--color-text-tertiary)",
              lineHeight: 1.55,
            }}
          >
            {item}
          </li>
        ))}
      </ul>
    </div>
  );
}

function OutcomeActionRow({
  action,
  onRefresh,
}: {
  action: DbAction;
  onRefresh: () => void;
}) {
  const isCompleted = action.status === "completed";
  const isProposed = action.status === "proposed";

  const handleComplete = useCallback(async () => {
    try {
      if (isCompleted) {
        await invoke("reopen_action", { id: action.id });
      } else {
        await invoke("complete_action", { id: action.id });
      }
      onRefresh();
    } catch (err) {
      console.error("Failed to toggle action:", err);
    }
  }, [action.id, isCompleted, onRefresh]);

  const handleAccept = useCallback(async () => {
    try {
      await invoke("accept_proposed_action", { id: action.id });
      onRefresh();
    } catch (err) {
      console.error("Failed to accept action:", err);
    }
  }, [action.id, onRefresh]);

  const handleReject = useCallback(async () => {
    try {
      await invoke("reject_proposed_action", { id: action.id });
      onRefresh();
    } catch (err) {
      console.error("Failed to reject action:", err);
    }
  }, [action.id, onRefresh]);

  const handleCyclePriority = useCallback(async () => {
    const cycle: Record<string, string> = { P1: "P2", P2: "P3", P3: "P1" };
    const next = cycle[action.priority] || "P2";
    try {
      await invoke("update_action_priority", {
        id: action.id,
        priority: next,
      });
      onRefresh();
    } catch (err) {
      console.error("Failed to update priority:", err);
    }
  }, [action.id, action.priority, onRefresh]);

  const priorityColor: Record<string, string> = {
    P1: "var(--color-spice-terracotta)",
    P3: "var(--color-text-tertiary)",
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "3px 4px",
        borderLeft: isProposed ? "2px dashed var(--color-spice-turmeric)" : "none",
        paddingLeft: isProposed ? 8 : 4,
      }}
    >
      {isProposed ? (
        /* Accept / Reject buttons for proposed actions */
        <div style={{ display: "flex", gap: 4, flexShrink: 0 }}>
          <button
            onClick={handleAccept}
            title="Accept"
            style={{
              width: 20,
              height: 20,
              borderRadius: 3,
              border: "1px solid var(--color-garden-sage)",
              background: "transparent",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: 0,
            }}
          >
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
              <path d="M2.5 6L5 8.5L9.5 4" stroke="var(--color-garden-sage)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
          <button
            onClick={handleReject}
            title="Reject"
            style={{
              width: 20,
              height: 20,
              borderRadius: 3,
              border: "1px solid var(--color-spice-terracotta)",
              background: "transparent",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: 0,
            }}
          >
            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
              <path d="M3 3L9 9M9 3L3 9" stroke="var(--color-spice-terracotta)" strokeWidth="2" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      ) : (
        /* Checkbox for accepted actions */
        <button
          onClick={handleComplete}
          style={{
            width: 16,
            height: 16,
            borderRadius: 3,
            border: isCompleted
              ? "1px solid var(--color-garden-sage)"
              : "1px solid var(--color-text-tertiary)",
            background: isCompleted ? "rgba(126, 170, 123, 0.2)" : "transparent",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
            padding: 0,
          }}
        >
          {isCompleted && <Check style={{ width: 12, height: 12, color: "var(--color-garden-sage)" }} />}
        </button>
      )}

      {isProposed ? (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: "var(--color-spice-turmeric)",
          }}
        >
          Suggested
        </span>
      ) : (
        <button
          onClick={handleCyclePriority}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            letterSpacing: "0.04em",
            padding: "1px 6px",
            border: "1px solid var(--color-rule-light)",
            borderRadius: 3,
            background: "transparent",
            color: priorityColor[action.priority] ?? "var(--color-text-secondary)",
            cursor: "pointer",
          }}
        >
          {action.priority}
        </button>
      )}

      <span
        style={{
          flex: 1,
          fontSize: 13,
          color: isCompleted ? "var(--color-text-tertiary)" : "var(--color-text-primary)",
          textDecoration: isCompleted ? "line-through" : "none",
        }}
      >
        {action.title}
      </span>

      {action.dueDate && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            color: "var(--color-text-tertiary)",
          }}
        >
          {action.dueDate}
        </span>
      )}
    </div>
  );
}

// =============================================================================
// Shared Components
// =============================================================================

function ActionItem({ action }: { action: ActionWithContext }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 10,
        borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
        paddingBottom: 10,
        paddingLeft: action.isOverdue ? 12 : 0,
        borderLeft: action.isOverdue ? "3px solid var(--color-spice-terracotta)" : "none",
      }}
    >
      <span style={bulletDot(action.isOverdue ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)")} />
      <div style={{ flex: 1 }}>
        <p style={{ fontWeight: 500, fontSize: 14, color: "var(--color-text-primary)", margin: 0 }}>
          {action.title}
        </p>
        {action.dueDate && (
          <p
            style={{
              fontSize: 13,
              color: action.isOverdue ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
              margin: "2px 0 0",
            }}
          >
            Due: {action.dueDate}
          </p>
        )}
        {action.context && (
          <p style={{ marginTop: 4, marginBottom: 0, fontSize: 13, color: "var(--color-text-tertiary)" }}>
            {action.context}
          </p>
        )}
      </div>
    </div>
  );
}

function ReferenceRow({ reference }: { reference: SourceReference }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "8px 12px",
        borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
      }}
    >
      <div>
        <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)", margin: 0 }}>
          {reference.label}
        </p>
        {reference.path && (
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              margin: "2px 0 0",
            }}
          >
            {reference.path}
          </p>
        )}
      </div>
      {reference.lastUpdated && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
          }}
        >
          {reference.lastUpdated}
        </span>
      )}
    </div>
  );
}

// =============================================================================
// Appendix Section
// =============================================================================

function AppendixSection({
  data,
  extendedStakeholderInsights,
}: {
  data: FullMeetingPrep;
  extendedStakeholderInsights: StakeholderInsight[];
}) {
  const [open, setOpen] = useState(false);

  const hasContent = Boolean(
    (data.intelligenceSummary && data.intelligenceSummary.split("\n").filter((l) => l.trim()).length > 1) ||
    (data.sinceLast && data.sinceLast.length > 0) ||
    (data.strategicPrograms && data.strategicPrograms.length > 0) ||
    (data.meetingContext && data.meetingContext.split("\n").length > 3) ||
    (data.currentState && data.currentState.length > 0) ||
    (data.questions && data.questions.length > 0) ||
    (data.keyPrinciples && data.keyPrinciples.length > 0) ||
    (data.references && data.references.length > 0) ||
    extendedStakeholderInsights.length > 0
  );

  if (!hasContent) return null;

  return (
    <section id="appendix" style={{ paddingTop: 24, display: "flex", flexDirection: "column", gap: 16 }}>
      <p style={monoOverline}>Appendix</p>
      <button
        onClick={() => setOpen(!open)}
        style={{
          background: "none",
          border: "none",
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "4px 0",
          width: "100%",
          ...chapterHeadingStyle,
        }}
      >
        <ChevronRight
          style={{
            width: 14,
            height: 14,
            transition: "transform 0.2s",
            transform: open ? "rotate(90deg)" : "rotate(0deg)",
          }}
        />
        Open Supporting Context
      </button>
      {open && (
        <div style={{ display: "flex", flexDirection: "column", gap: 32, marginTop: 12 }}>
          {/* Full intelligence summary */}
          {data.intelligenceSummary && data.intelligenceSummary.split("\n").filter((l) => l.trim()).length > 1 && (
            <section>
              <SectionLabel
                label="Full Intelligence Summary"
                copyText={data.intelligenceSummary}
                copyLabel="summary"
              />
              <div style={{ marginTop: 12 }}>
                {data.intelligenceSummary
                  .split("\n")
                  .filter((line) => line.trim())
                  .map((line, i) => (
                    <p
                      key={i}
                      style={{
                        fontSize: 14,
                        lineHeight: 1.75,
                        color: "var(--color-text-primary)",
                        margin: 0,
                        marginTop: i > 0 ? 8 : 0,
                      }}
                    >
                      {line}
                    </p>
                  ))}
              </div>
            </section>
          )}

          {data.sinceLast && data.sinceLast.length > 0 && (
            <section>
              <SectionLabel
                label="Since Last Meeting"
                copyText={formatBulletList(data.sinceLast)}
                copyLabel="since last meeting"
              />
              <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 16 }}>
                {data.sinceLast.map((item, i) => (
                  <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10, fontSize: 14, lineHeight: 1.65 }}>
                    <span style={bulletDot("var(--color-spice-turmeric)")} />
                    <span style={{ color: "var(--color-text-primary)" }}>{item}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {data.strategicPrograms && data.strategicPrograms.length > 0 && (
            <section>
              <SectionLabel
                label="Strategic Programs"
                copyText={formatBulletList(data.strategicPrograms)}
                copyLabel="programs"
              />
              <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 16 }}>
                {data.strategicPrograms.map((item, i) => (
                  <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10, fontSize: 14, lineHeight: 1.65 }}>
                    <span
                      style={{
                        marginTop: 3,
                        fontSize: 14,
                        color: item.startsWith("✓") ? "var(--color-garden-sage)" : "var(--color-text-tertiary)",
                      }}
                    >
                      {item.startsWith("✓") ? "✓" : "○"}
                    </span>
                    <span style={{ color: "var(--color-text-primary)" }}>{item.replace(/^[✓○]\s*/, "")}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {data.meetingContext && data.meetingContext.split("\n").length > 3 && (
            <section>
              <SectionLabel
                label="Full Context"
                copyText={data.meetingContext}
                copyLabel="context"
              />
              <p
                style={{
                  marginTop: 12,
                  marginBottom: 0,
                  whiteSpace: "pre-wrap",
                  fontSize: 14,
                  lineHeight: 1.65,
                  color: "var(--color-text-primary)",
                }}
              >
                {data.meetingContext}
              </p>
            </section>
          )}

          {data.currentState && data.currentState.length > 0 && (
            <section>
              <SectionLabel label="Current State" copyText={formatBulletList(data.currentState)} copyLabel="current state" />
              <ul style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 16 }}>
                {data.currentState.map((item, i) => (
                  <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10, fontSize: 14, lineHeight: 1.65 }}>
                    <span style={bulletDot("var(--color-text-tertiary)")} />
                    <span style={{ color: "var(--color-text-primary)" }}>{item}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {data.questions && data.questions.length > 0 && (
            <section>
              <SectionLabel
                label="Questions to Surface"
                copyText={formatNumberedList(data.questions)}
                copyLabel="questions"
              />
              <ol style={{ listStyle: "none", margin: "16px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 16 }}>
                {data.questions.map((q, i) => (
                  <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10, fontSize: 14, lineHeight: 1.65 }}>
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        fontWeight: 500,
                        color: "var(--color-text-tertiary)",
                        width: 16,
                        textAlign: "right",
                        flexShrink: 0,
                        paddingTop: 2,
                      }}
                    >
                      {i + 1}.
                    </span>
                    <span style={{ color: "var(--color-text-primary)" }}>{q}</span>
                  </li>
                ))}
              </ol>
            </section>
          )}

          {data.keyPrinciples && data.keyPrinciples.length > 0 && (
            <section>
              <SectionLabel
                label="Key Principles"
                copyText={formatBulletList(data.keyPrinciples)}
                copyLabel="principles"
              />
              <div style={{ display: "flex", flexDirection: "column", gap: 16, marginTop: 16 }}>
                {data.keyPrinciples.map((principle, i) => (
                  <blockquote
                    key={i}
                    style={{
                      borderLeft: "2px solid rgba(201, 162, 39, 0.3)",
                      paddingLeft: 16,
                      margin: 0,
                      fontSize: 14,
                      fontStyle: "italic",
                      color: "var(--color-text-tertiary)",
                      lineHeight: 1.55,
                    }}
                  >
                    {principle}
                  </blockquote>
                ))}
              </div>
            </section>
          )}

          {/* Extended stakeholder map */}
          {extendedStakeholderInsights.length > 0 && (
            <section>
              <SectionLabel
                label={`Extended Stakeholder Map (${extendedStakeholderInsights.length})`}
              />
              <StakeholderInsightList people={extendedStakeholderInsights} />
            </section>
          )}

          {data.references && data.references.length > 0 && (
            <section>
              <SectionLabel label="References" />
              <div style={{ display: "flex", flexDirection: "column", gap: 0, marginTop: 12 }}>
                {data.references.map((ref_, i) => (
                  <ReferenceRow key={i} reference={ref_} />
                ))}
              </div>
            </section>
          )}
        </div>
      )}
    </section>
  );
}

// =============================================================================
// StakeholderInsightList (used in appendix for extended stakeholders)
// =============================================================================

function StakeholderInsightList({ people }: { people: StakeholderInsight[] }) {
  const engagementColor: Record<string, string> = {
    champion: "var(--color-garden-sage)",
    detractor: "var(--color-spice-terracotta)",
    neutral: "var(--color-text-tertiary)",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 20, marginTop: 16 }}>
      {people.map((person, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 12,
            borderBottom: "1px solid rgba(30, 37, 48, 0.04)",
            paddingBottom: 10,
          }}
        >
          <div
            style={{
              width: 28,
              height: 28,
              borderRadius: "50%",
              background: "rgba(201, 162, 39, 0.1)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 12,
              fontWeight: 500,
              color: "var(--color-spice-turmeric)",
              flexShrink: 0,
            }}
          >
            {person.name.charAt(0)}
          </div>
          <div style={{ minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)", margin: 0 }}>
                {person.name}
              </p>
              {person.role && (
                <span style={{ fontSize: 13, color: "var(--color-text-tertiary)" }}>
                  {sanitizeInlineText(person.role)}
                </span>
              )}
              {person.engagement && (
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    fontWeight: 500,
                    textTransform: "capitalize",
                    color: engagementColor[person.engagement] ?? "var(--color-text-tertiary)",
                  }}
                >
                  {person.engagement}
                </span>
              )}
            </div>
            {person.assessment && (
              <p
                style={{
                  marginTop: 3,
                  marginBottom: 0,
                  fontSize: 13,
                  lineHeight: 1.55,
                  color: "var(--color-text-tertiary)",
                }}
              >
                {truncateText(sanitizeInlineText(person.assessment), 180)}
              </p>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

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
  for (const ctx of attendeeContext ?? []) {
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
  for (const insight of insights ?? []) {
    const key = normalizePersonKey(insight.name);
    const existing = byKey.get(key);
    if (existing) {
      if (insight.assessment) existing.assessment = insight.assessment;
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

function hasReferenceContent(data: FullMeetingPrep): boolean {
  return Boolean(
    (data.meetingContext && data.meetingContext.split("\n").length > 3) ||
    (data.currentState && data.currentState.length > 0) ||
    (data.questions && data.questions.length > 0) ||
    (data.keyPrinciples && data.keyPrinciples.length > 0) ||
    (data.references && data.references.length > 0)
  );
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

function normalizeCalendarNotes(notes: string | undefined): string | undefined {
  if (!notes) return undefined;

  const raw = notes.trim();
  if (!raw) return undefined;

  if (!/[<>]/.test(raw)) return raw;

  const withStructure = raw
    .replace(/<a\s+[^>]*href=["']([^"']+)["'][^>]*>(.*?)<\/a>/gi, "$2 ($1)")
    .replace(/<\s*br\s*\/?>/gi, "\n")
    .replace(/<\s*li[^>]*>/gi, "- ")
    .replace(/<\/\s*(p|div|section|article|li|tr|h[1-6])\s*>/gi, "\n");

  try {
    const doc = new DOMParser().parseFromString(withStructure, "text/html");
    const text = (doc.body?.textContent ?? "").replace(/\u00a0/g, " ");
    const normalized = text
      .split("\n")
      .map((line) => line.replace(/\s+/g, " ").trim())
      .filter((line, i, arr) => line.length > 0 || (i > 0 && arr[i - 1].length > 0))
      .join("\n")
      .trim();

    if (normalized) return normalized;
  } catch {
    // Fall through to regex fallback.
  }

  const fallback = withStructure
    .replace(/<[^>]+>/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return fallback || raw;
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

function deriveRecentWins(data: FullMeetingPrep): { wins: string[]; sources: SourceReference[] } {
  const wins: string[] = [];
  const sources: SourceReference[] = [];
  const seenWins = new Set<string>();
  const seenSources = new Set<string>();

  const addSource = (rawSource: string | undefined) => {
    if (!rawSource) return;
    const clean = sanitizeInlineText(rawSource);
    if (!clean) return;
    const label = clean.split(/[\\/]/).filter(Boolean).pop() ?? clean;
    const key = normalizePersonKey(clean);
    if (!seenSources.has(key)) {
      seenSources.add(key);
      sources.push({
        label,
        path: clean,
      });
    }
  };

  for (const source of data.recentWinSources ?? []) {
    const key = normalizePersonKey(source.path ?? source.label);
    if (!seenSources.has(key)) {
      seenSources.add(key);
      sources.push({
        label: sanitizeInlineText(source.label),
        path: source.path ? sanitizeInlineText(source.path) : undefined,
        lastUpdated: source.lastUpdated,
      });
    }
  }

  const hasStructuredWins = Boolean(data.recentWins && data.recentWins.length > 0);
  const winCandidates = hasStructuredWins ? (data.recentWins ?? []) : (data.talkingPoints ?? []);

  for (const point of winCandidates) {
    const { source } = splitInlineSourceTail(point);
    if (!data.recentWinSources || data.recentWinSources.length === 0) {
      addSource(source);
    }

    const win = cleanPrepLine(point);
    const winKey = normalizePersonKey(win);
    if (win && !seenWins.has(winKey)) {
      seenWins.add(winKey);
      wins.push(win);
    }
  }

  if (!hasStructuredWins) {
    for (const point of data.talkingPoints ?? []) {
      const { source } = splitInlineSourceTail(point);
      if (!data.recentWinSources || data.recentWinSources.length === 0) {
        addSource(source);
      }
    }
  }

  return { wins, sources };
}

function truncateText(value: string, maxChars: number): string {
  if (value.length <= maxChars) return value;
  return `${value.slice(0, maxChars - 1).trim()}…`;
}

// =============================================================================
// Copy-to-clipboard formatters
// =============================================================================

function formatBulletList(items: string[]): string {
  return items.map((item) => `- ${item}`).join("\n");
}

function formatNumberedList(items: string[]): string {
  return items.map((item, i) => `${i + 1}. ${item}`).join("\n");
}

function formatOpenItems(items: ActionWithContext[]): string {
  return items
    .map((item) => {
      let line = `- ${item.title}`;
      if (item.dueDate) line += ` (due: ${item.dueDate})`;
      if (item.isOverdue) line += " [OVERDUE]";
      return line;
    })
    .join("\n");
}


// ═══════════════════════════════════════════════════════════════════════════
// QuillSyncBadge — shows transcript sync status for a meeting
// ═══════════════════════════════════════════════════════════════════════════

function QuillSyncBadge({ meetingId }: { meetingId: string }) {
  const [syncState, setSyncState] = useState<import("@/types").QuillSyncState | null>(null);

  const loadSync = useCallback(async () => {
    try {
      const rows = await invoke<import("@/types").QuillSyncState[]>(
        "get_quill_sync_states",
        { meetingId },
      );
      setSyncState(rows.length > 0 ? rows[0] : null);
    } catch {
      // Quill not enabled or no sync state — expected
    }
  }, [meetingId]);

  useEffect(() => {
    loadSync();
  }, [loadSync]);

  // Poll for state transitions while sync is in progress
  useEffect(() => {
    if (!syncState || syncState.state === "completed" || syncState.state === "failed" || syncState.state === "abandoned") {
      return;
    }
    const interval = setInterval(loadSync, 10_000);
    return () => clearInterval(interval);
  }, [syncState?.state, loadSync]);

  // Listen for transcript-processed event to refresh
  useEffect(() => {
    let cancelled = false;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<string>("transcript-processed", (event) => {
        if (!cancelled && event.payload === meetingId) {
          loadSync();
        }
      });
    });
    return () => { cancelled = true; };
  }, [meetingId, loadSync]);

  if (!syncState) return null;

  const { state, matchConfidence, source } = syncState;
  const sourceLabel = source === "granola" ? "via Granola" : "via Quill";

  // Define badge content based on state
  let icon: React.ReactNode;
  let label: string;
  let color: string;

  switch (state) {
    case "pending":
      icon = <Loader2 style={{ width: 14, height: 14, animation: "spin 1s linear infinite" }} />;
      label = `Waiting to sync transcript ${sourceLabel}`;
      color = "var(--color-golden-turmeric)";
      break;
    case "polling":
      icon = <Loader2 style={{ width: 14, height: 14, animation: "spin 1s linear infinite" }} />;
      label = `Searching for transcript ${sourceLabel}`;
      color = "var(--color-golden-turmeric)";
      break;
    case "fetching":
      icon = <Loader2 style={{ width: 14, height: 14, animation: "spin 1s linear infinite" }} />;
      label = `Downloading transcript ${sourceLabel}`;
      color = "var(--color-golden-turmeric)";
      break;
    case "processing":
      icon = <Loader2 style={{ width: 14, height: 14, animation: "spin 1s linear infinite" }} />;
      label = `Processing transcript ${sourceLabel}`;
      color = "var(--color-sky-larkspur)";
      break;
    case "completed":
      icon = <Check style={{ width: 14, height: 14 }} />;
      label = matchConfidence
        ? `Transcript synced ${sourceLabel} (${Math.round(matchConfidence * 100)}% match)`
        : `Transcript synced ${sourceLabel}`;
      color = "var(--color-garden-olive)";
      break;
    case "failed":
    case "abandoned":
      icon = <AlertTriangle style={{ width: 14, height: 14 }} />;
      label = state === "abandoned" ? "Sync abandoned" : "Sync failed";
      color = "var(--color-spice-terracotta)";
      break;
    default:
      return null;
  }

  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        fontWeight: 500,
        letterSpacing: "0.04em",
        color,
        padding: "4px 10px",
        border: `1px solid ${color}`,
        borderRadius: 4,
      }}
    >
      {icon}
      {label}
    </div>
  );
}

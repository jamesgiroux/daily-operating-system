import { useState, useEffect, useRef, useCallback, useMemo } from "react";
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
  AgendaItem,
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
import { CopyButton } from "@/components/ui/copy-button";
import { MeetingEntityChips } from "@/components/ui/meeting-entity-chips";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import {
  AlertCircle,
  Check,
  ChevronRight,
  Clock,
  Copy,
  Users,
  FileText,
  HelpCircle,
  BookOpen,
  AlertTriangle,
  CheckCircle,
  History,
  Target,
  CalendarDays,
  Paperclip,
  Loader2,
  Trophy,
  CircleDot,
} from "lucide-react";

// ── Shared style fragments ──

const monoOverline: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.2em",
  color: "var(--color-text-tertiary)",
};

const chapterHeading: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.12em",
  color: "var(--color-text-tertiary)",
};

const editorialRule: React.CSSProperties = {
  height: 1,
  background: "var(--color-rule-heavy)",
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

const sidebarCard: React.CSSProperties = {
  border: "1px solid var(--color-rule-light)",
  padding: 20,
};

const bulletDot = (color: string): React.CSSProperties => ({
  width: 6,
  height: 6,
  borderRadius: "50%",
  background: color,
  flexShrink: 0,
  marginTop: 7,
});

const pulseBg: React.CSSProperties = {
  background: "var(--color-rule-light)",
  borderRadius: 4,
};

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

  // Register magazine shell
  const shellConfig = useMemo(() => ({
    folioLabel: "Intelligence Report",
    atmosphereColor: "turmeric" as const,
    activePage: "today" as const,
    backLink: { label: "Today", onClick: () => navigate({ to: "/" }) },
    folioStatusText: saveStatus === "saving" ? "Saving\u2026" : saveStatus === "saved" ? "\u2713 Saved" : undefined,
  }), [navigate, saveStatus]);
  useRegisterMagazineShell(shellConfig);

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

  // Determine meeting time state for editability (I194)
  const isPastMeeting = !canEditUserLayer;
  const isEditable = canEditUserLayer;

  // ── Loading state ──
  if (loading) {
    return (
      <div style={{ maxWidth: 960, margin: "0 auto", padding: "48px 0 80px" }}>
        <div style={{ ...pulseBg, height: 14, width: 120, marginBottom: 20, animation: "pulse 2s ease-in-out infinite" }} />
        <div style={{ ...pulseBg, height: 32, width: "75%", marginBottom: 12, animation: "pulse 2s ease-in-out infinite" }} />
        <div style={{ ...pulseBg, height: 12, width: 200, marginBottom: 40, animation: "pulse 2s ease-in-out infinite" }} />
        <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
          <div style={{ ...pulseBg, height: 120, animation: "pulse 2s ease-in-out infinite" }} />
          <div style={{ ...pulseBg, height: 180, animation: "pulse 2s ease-in-out infinite" }} />
          <div style={{ ...pulseBg, height: 120, animation: "pulse 2s ease-in-out infinite" }} />
        </div>
      </div>
    );
  }

  // ── Error state ──
  if (error) {
    return (
      <div style={{ maxWidth: 960, margin: "0 auto", padding: "48px 0 80px" }}>
        <div
          style={{
            borderLeft: "3px solid var(--color-spice-terracotta)",
            paddingLeft: 24,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
            <AlertCircle style={{ width: 20, height: 20, color: "var(--color-spice-terracotta)" }} />
            <h2
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 22,
                fontWeight: 600,
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              Unable to Load Intelligence
            </h2>
          </div>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 15,
              lineHeight: 1.65,
              color: "var(--color-text-secondary)",
              margin: "0 0 20px",
            }}
          >
            {error}
          </p>
          <button
            onClick={() => loadMeetingIntelligence()}
            style={editorialBtn}
          >
            Retry
          </button>
        </div>
      </div>
    );
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

  const attendeeNames = new Set<string>([
    ...(data.attendeeContext ?? []).map((p) => normalizePersonKey(p.name)),
    ...(data.attendees ?? []).map((p) => normalizePersonKey(p.name)),
  ]);
  const matchingStakeholderInsights = (data.stakeholderInsights ?? []).filter((person) =>
    attendeeNames.has(normalizePersonKey(person.name))
  );
  const extendedStakeholderInsights = (data.stakeholderInsights ?? []).filter(
    (person) => !attendeeNames.has(normalizePersonKey(person.name))
  );
  const topRisks = [
    ...((data.entityRisks ?? []).map((risk) => risk.text)),
    ...(data.risks ?? []),
  ]
    .map((risk) => sanitizeInlineText(risk))
    .filter((risk) => risk.length > 0)
    .slice(0, 3);
  const lifecycle = getLifecycleForDisplay(data);
  const heroMeta = getHeroMetaItems(data);
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
  const recentWinsForSidebar = recentWins.filter(
    (win) => !agendaTopics.has(normalizePersonKey(win))
  );
  const reportNav = [
    { id: "executive-brief", label: "Executive Brief", show: Boolean(data.intelligenceSummary || data.meetingContext) },
    { id: "agenda", label: "Agenda", show: Boolean(data.proposedAgenda?.length || data.userAgenda?.length) },
    { id: "risks", label: "Risks", show: Boolean((data.entityRisks?.length ?? 0) > 0 || (data.risks?.length ?? 0) > 0) },
    { id: "people", label: "People", show: Boolean(data.attendeeContext?.length || data.attendees?.length || data.stakeholderInsights?.length) },
    { id: "actions", label: "Open Items", show: Boolean(data.openItems?.length) },
    { id: "appendix", label: "Appendix", show: hasReferenceContent(data) || Boolean(data.sinceLast?.length || data.strategicPrograms?.length || data.currentState?.length || data.references?.length) },
  ].filter((item) => item.show);

  return (
    <>
      <div style={{ maxWidth: 960, margin: "0 auto", padding: "0 0 80px" }}>
        {/* Post-meeting: outcomes first (I195) */}
        {isPastMeeting && outcomes && (
          <>
            <OutcomesSection outcomes={outcomes} onRefresh={loadMeetingIntelligence} onSaveStatus={setSaveStatus} />
            <div style={{ ...editorialRule, margin: "48px 0" }} />
            <p style={{ ...chapterHeading, marginBottom: 20 }}>
              Pre-Meeting Context
            </p>
          </>
        )}

        {/* Past meeting without outcomes: prompt to attach transcript */}
        {isPastMeeting && !outcomes && (
          <div
            style={{
              border: "1px dashed var(--color-rule-light)",
              padding: "20px 24px",
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
                No outcomes captured yet
              </p>
              <p
                style={{
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                  margin: 0,
                }}
              >
                Attach a transcript or manually capture meeting outcomes.
              </p>
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
            <div
              style={{
                display: "grid",
                gap: 48,
                gridTemplateColumns: "minmax(0, 1fr) 260px",
              }}
            >
              {/* ── Main column ── */}
              <div style={{ display: "flex", flexDirection: "column", gap: 48 }}>
                {/* ── Hero section ── */}
                <section id="executive-brief">
                  <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16 }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                      <p style={monoOverline}>
                        Meeting Intelligence Report
                      </p>
                      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}>
                        <h1
                          style={{
                            fontFamily: "var(--font-serif)",
                            fontSize: 34,
                            fontWeight: 600,
                            letterSpacing: "-0.01em",
                            color: "var(--color-text-primary)",
                            margin: 0,
                            lineHeight: 1.15,
                          }}
                        >
                          {data.title}
                        </h1>
                        {lifecycle && (
                          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
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
                          </span>
                        )}
                      </div>
                      <p
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 12,
                          letterSpacing: "0.04em",
                          color: "var(--color-text-tertiary)",
                          margin: 0,
                        }}
                      >
                        {data.timeRange}
                      </p>
                      {/* Entity assignment */}
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
                    </div>
                    <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0, paddingTop: 4 }}>
                      {isEditable && (
                        <button
                          onClick={handlePrefillFromContext}
                          disabled={prefilling}
                          style={{
                            ...editorialBtn,
                            opacity: prefilling ? 0.6 : 1,
                          }}
                        >
                          {prefilling ? "Prefilling..." : "Prefill Prep"}
                        </button>
                      )}
                      <button
                        onClick={handleDraftAgendaMessage}
                        style={editorialBtn}
                      >
                        Draft agenda message
                      </button>
                      <CopyAllButton data={data} />
                    </div>
                  </div>

                  {(data.intelligenceSummary || data.meetingContext) ? (
                    <blockquote
                      style={{
                        marginTop: 28,
                        marginBottom: 0,
                        marginLeft: 0,
                        marginRight: 0,
                        borderLeft: "3px solid var(--color-spice-turmeric)",
                        paddingLeft: 24,
                      }}
                    >
                      {(data.intelligenceSummary || data.meetingContext || "")
                        .split("\n")
                        .filter((line) => line.trim())
                        .slice(0, 3)
                        .map((line, i) => (
                          <p
                            key={i}
                            style={{
                              fontFamily: "var(--font-sans)",
                              fontSize: 17,
                              lineHeight: 1.75,
                              color: "var(--color-text-primary)",
                              margin: 0,
                              marginTop: i > 0 ? 8 : 0,
                            }}
                          >
                            {line}
                          </p>
                        ))}
                    </blockquote>
                  ) : (
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

                  {topRisks.length > 0 && (
                    <div
                      style={{
                        marginTop: 28,
                        display: "grid",
                        gap: 10,
                        gridTemplateColumns: "repeat(3, 1fr)",
                      }}
                    >
                      {topRisks.map((risk, i) => (
                        <div
                          key={i}
                          style={{
                            borderLeft: "3px solid var(--color-spice-terracotta)",
                            paddingLeft: 14,
                            paddingTop: 10,
                            paddingBottom: 10,
                          }}
                        >
                          <p
                            style={{
                              fontFamily: "var(--font-mono)",
                              fontSize: 10,
                              fontWeight: 600,
                              textTransform: "uppercase",
                              letterSpacing: "0.12em",
                              color: "var(--color-spice-terracotta)",
                              margin: 0,
                            }}
                          >
                            Risk {i + 1}
                          </p>
                          <p
                            style={{
                              fontSize: 14,
                              lineHeight: 1.55,
                              color: "var(--color-text-primary)",
                              marginTop: 4,
                              marginBottom: 0,
                            }}
                          >
                            {risk}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}

                  {heroMeta.length > 0 && (
                    <div
                      style={{
                        marginTop: 28,
                        display: "grid",
                        gridTemplateColumns: "repeat(4, 1fr)",
                        gap: 12,
                      }}
                    >
                      {heroMeta.map((item) => (
                        <div
                          key={item.label}
                          style={{
                            borderLeft: "1px solid var(--color-rule-light)",
                            paddingLeft: 12,
                            paddingTop: 4,
                            paddingBottom: 4,
                          }}
                        >
                          <p
                            style={{
                              fontFamily: "var(--font-mono)",
                              fontSize: 10,
                              fontWeight: 600,
                              textTransform: "uppercase",
                              letterSpacing: "0.14em",
                              color: "var(--color-text-tertiary)",
                              margin: 0,
                            }}
                          >
                            {item.label}
                          </p>
                          <p
                            style={{
                              fontSize: 14,
                              fontWeight: 500,
                              color: resolveMetaToneColor(item.tone),
                              marginTop: 4,
                              marginBottom: 0,
                            }}
                          >
                            {item.value}
                          </p>
                        </div>
                      ))}
                    </div>
                  )}
                </section>

                {/* ── Agenda section ── */}
                <section id="agenda" style={{ display: "flex", flexDirection: "column", gap: 20 }}>
                  <SectionLabel
                    label="Agenda"
                    icon={<Target style={{ width: 14, height: 14 }} />}
                    copyText={agendaDisplayItems.length > 0 ? formatProposedAgenda(agendaDisplayItems) : undefined}
                    copyLabel="agenda"
                  />
                  {agendaDisplayItems.length > 0 ? (
                    <ol style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 10 }}>
                      {agendaDisplayItems.map((item, i) => (
                        <li
                          key={i}
                          style={{
                            display: "flex",
                            alignItems: "flex-start",
                            gap: 12,
                            borderBottom: "1px solid var(--color-rule-light)",
                            paddingBottom: 10,
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
                            <p style={{ fontSize: 14, fontWeight: 500, lineHeight: 1.4, margin: 0, color: "var(--color-text-primary)" }}>
                              {item.topic}
                            </p>
                            {item.why && (
                              <p style={{ fontSize: 13, color: "var(--color-text-tertiary)", marginTop: 3, marginBottom: 0, lineHeight: 1.5 }}>
                                {item.why}
                              </p>
                            )}
                          </div>
                          {item.source && (
                            <span
                              style={{
                                fontFamily: "var(--font-mono)",
                                fontSize: 10,
                                fontWeight: 500,
                                letterSpacing: "0.04em",
                                flexShrink: 0,
                                color: agendaSourceColor(item.source),
                              }}
                            >
                              {item.source === "calendar_note"
                                ? "calendar"
                                : item.source === "talking_point"
                                ? "win"
                                : item.source === "open_item"
                                  ? "action"
                                  : item.source}
                            </span>
                          )}
                        </li>
                      ))}
                    </ol>
                  ) : (
                    <p
                      style={{
                        fontSize: 14,
                        color: "var(--color-text-tertiary)",
                        fontStyle: "italic",
                      }}
                    >
                      No proposed agenda yet. Add your own agenda items below.
                    </p>
                  )}
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
                      }}
                    >
                      Prefill appended new agenda/notes content.
                    </div>
                  )}
                  {meetingId && (
                    <UserAgendaEditor
                      meetingId={meetingId}
                      initialAgenda={data.userAgenda}
                      isEditable={isEditable}
                      onSaveStatus={setSaveStatus}
                    />
                  )}
                  {meetingId && (
                    <UserNotesEditor
                      meetingId={meetingId}
                      initialNotes={data.userNotes}
                      isEditable={isEditable}
                      onSaveStatus={setSaveStatus}
                    />
                  )}
                  {calendarNotes && (
                    <section>
                      <SectionLabel label="Calendar Notes" icon={<CalendarDays style={{ width: 14, height: 14 }} />} />
                      <p
                        style={{
                          marginTop: 12,
                          whiteSpace: "pre-wrap",
                          fontSize: 14,
                          color: "var(--color-text-tertiary)",
                          lineHeight: 1.65,
                        }}
                      >
                        {calendarNotes}
                      </p>
                    </section>
                  )}
                </section>

                {/* ── Risks section ── */}
                {((data.entityRisks && data.entityRisks.length > 0) || (data.risks && data.risks.length > 0)) && (
                  <section id="risks" style={{ display: "flex", flexDirection: "column", gap: 16 }}>
                    <SectionLabel
                      label="Risks"
                      icon={<AlertTriangle style={{ width: 14, height: 14, color: "var(--color-spice-terracotta)" }} />}
                      labelColor="var(--color-spice-terracotta)"
                      copyText={formatBulletList([
                        ...(data.entityRisks?.map((r) => r.text) ?? []),
                        ...(data.risks ?? []),
                      ])}
                      copyLabel="risks"
                    />
                    <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
                      {data.entityRisks?.map((risk, i) => (
                        <li
                          key={`entity-${i}`}
                          style={{
                            display: "flex",
                            alignItems: "flex-start",
                            gap: 10,
                            fontSize: 14,
                            lineHeight: 1.65,
                            borderBottom: "1px solid var(--color-rule-light)",
                            paddingBottom: 8,
                          }}
                        >
                          <span
                            style={bulletDot(
                              risk.urgency === "high"
                                ? "var(--color-spice-terracotta)"
                                : "rgba(196, 101, 74, 0.5)"
                            )}
                          />
                          <span style={{ flex: 1, color: "var(--color-text-primary)" }}>{risk.text}</span>
                        </li>
                      ))}
                      {data.risks?.map((risk, i) => (
                        <li
                          key={`ai-${i}`}
                          style={{
                            display: "flex",
                            alignItems: "flex-start",
                            gap: 10,
                            fontSize: 14,
                            lineHeight: 1.65,
                            borderBottom: "1px solid var(--color-rule-light)",
                            paddingBottom: 8,
                          }}
                        >
                          <span style={bulletDot("rgba(196, 101, 74, 0.5)")} />
                          <span style={{ color: "var(--color-text-primary)" }}>{risk}</span>
                        </li>
                      ))}
                    </ul>
                  </section>
                )}

                {/* ── People section ── */}
                <section id="people" style={{ display: "flex", flexDirection: "column", gap: 24 }}>
                  <PeopleInTheRoom attendeeContext={data.attendeeContext} attendees={data.attendees} />

                  {matchingStakeholderInsights.length > 0 && (
                    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                      <p style={chapterHeading}>
                        Attendee Intelligence
                      </p>
                      <StakeholderInsightList people={matchingStakeholderInsights} />
                    </div>
                  )}

                  {extendedStakeholderInsights.length > 0 && (
                    <ExtendedStakeholderToggle people={extendedStakeholderInsights} />
                  )}
                </section>

                {/* ── Open Items section ── */}
                {data.openItems && data.openItems.length > 0 && (
                  <section id="actions" style={{ display: "flex", flexDirection: "column", gap: 16 }}>
                    <SectionLabel
                      label="Open Items"
                      icon={<CheckCircle style={{ width: 14, height: 14 }} />}
                      copyText={formatOpenItems(data.openItems)}
                      copyLabel="open items"
                    />
                    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                      {data.openItems.map((item, i) => (
                        <ActionItem key={i} action={item} />
                      ))}
                    </div>
                  </section>
                )}

                {/* ── Appendix section ── */}
                {(hasReferenceContent(data) || (data.sinceLast?.length ?? 0) > 0 || (data.strategicPrograms?.length ?? 0) > 0) && (
                  <AppendixSection data={data} />
                )}

                {/* ── End of Brief ── */}
                <FinisMarker />
              </div>

              {/* ── Sidebar ── */}
              <aside>
                <div style={{ position: "sticky", top: 24, display: "flex", flexDirection: "column", gap: 24 }}>
                  {/* Jump To nav */}
                  <div style={sidebarCard}>
                    <p style={{ ...chapterHeading, letterSpacing: "0.16em", marginBottom: 12 }}>Jump To</p>
                    <nav style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                      {reportNav.map((item) => (
                        <a
                          key={item.id}
                          href={`#${item.id}`}
                          style={{
                            display: "block",
                            padding: "6px 8px",
                            fontSize: 14,
                            color: "var(--color-text-tertiary)",
                            textDecoration: "none",
                            borderRadius: 3,
                          }}
                        >
                          {item.label}
                        </a>
                      ))}
                    </nav>
                  </div>

                  {/* Relationship Signals */}
                  {data.stakeholderSignals && (
                    <div style={sidebarCard}>
                      <p style={{ ...chapterHeading, letterSpacing: "0.16em", marginBottom: 12 }}>Relationship Signals</p>
                      <RelationshipPills signals={data.stakeholderSignals} />
                    </div>
                  )}

                  {/* Before This Meeting */}
                  {data.entityReadiness && data.entityReadiness.length > 0 && (
                    <div
                      style={{
                        ...sidebarCard,
                        borderColor: "var(--color-spice-turmeric)",
                        borderLeftWidth: 3,
                      }}
                    >
                      <p
                        style={{
                          ...chapterHeading,
                          letterSpacing: "0.16em",
                          color: "var(--color-spice-turmeric)",
                          marginBottom: 12,
                        }}
                      >
                        Before This Meeting
                      </p>
                      <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
                        {data.entityReadiness.slice(0, 4).map((item, i) => (
                          <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 14, color: "var(--color-text-primary)" }}>
                            <span style={bulletDot("rgba(201, 162, 39, 0.6)")} />
                            <span>{item}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  {/* Recent Email Signals */}
                  {data.recentEmailSignals && data.recentEmailSignals.length > 0 && (
                    <div
                      style={{
                        ...sidebarCard,
                        borderColor: "var(--color-spice-turmeric)",
                      }}
                    >
                      <p
                        style={{
                          ...chapterHeading,
                          letterSpacing: "0.16em",
                          color: "var(--color-spice-turmeric)",
                          marginBottom: 12,
                        }}
                      >
                        Recent Email Signals
                      </p>
                      <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 12 }}>
                        {data.recentEmailSignals.slice(0, 4).map((signal, i) => (
                          <li key={`${signal.id ?? i}-${signal.signalType}`} style={{ fontSize: 14 }}>
                            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
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

                  {/* Recent Wins */}
                  {recentWinsForSidebar.length > 0 && (
                    <div
                      style={{
                        ...sidebarCard,
                        borderColor: "var(--color-garden-sage)",
                        borderLeftWidth: 3,
                      }}
                    >
                      <p
                        style={{
                          ...chapterHeading,
                          letterSpacing: "0.16em",
                          color: "var(--color-garden-sage)",
                          marginBottom: 12,
                        }}
                      >
                        Recent Wins
                      </p>
                      <ul style={{ listStyle: "none", margin: 0, padding: 0, display: "flex", flexDirection: "column", gap: 10 }}>
                        {recentWinsForSidebar.slice(0, 4).map((win, i) => (
                          <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 8, fontSize: 14 }}>
                            <span style={bulletDot("rgba(126, 170, 123, 0.7)")} />
                            <span style={{ lineHeight: 1.55, color: "var(--color-text-primary)" }}>{win}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              </aside>
            </div>
          </div>
        )}

        {/* Pre-meeting: outcomes below prep if they exist (I195) */}
        {!isPastMeeting && outcomes && (
          <>
            <div style={{ ...editorialRule, margin: "48px 0" }} />
            <OutcomesSection outcomes={outcomes} onRefresh={loadMeetingIntelligence} onSaveStatus={setSaveStatus} />
          </>
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
  icon,
  labelColor,
  copyText,
  copyLabel,
}: {
  label: string;
  icon?: React.ReactNode;
  labelColor?: string;
  copyText?: string;
  copyLabel?: string;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <div
        style={{
          ...chapterHeading,
          display: "flex",
          alignItems: "center",
          gap: 6,
          color: labelColor || "var(--color-text-tertiary)",
        }}
      >
        {icon}
        {label}
      </div>
      <div style={{ flex: 1, height: 1, background: "var(--color-rule-light)" }} />
      {copyText && (
        <CopyButton text={copyText} label={copyLabel} />
      )}
    </div>
  );
}

// =============================================================================
// RelationshipPills
// =============================================================================

function RelationshipPills({ signals }: { signals: StakeholderSignals }) {
  const tempColor: Record<string, string> = {
    hot: "var(--color-garden-sage)",
    warm: "var(--color-spice-turmeric)",
    cool: "var(--color-text-tertiary)",
    cold: "var(--color-spice-terracotta)",
  };
  const color = tempColor[signals.temperature] ?? "var(--color-text-tertiary)";

  const lastMeetingText = signals.lastMeeting
    ? formatRelativeDateLong(signals.lastMeeting)
    : "No meetings recorded";

  const pillStyle: React.CSSProperties = {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    letterSpacing: "0.04em",
    padding: "3px 8px",
    border: "1px solid var(--color-rule-light)",
    borderRadius: 3,
  };

  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
      <span style={{ ...pillStyle, color, textTransform: "capitalize" }}>
        {signals.temperature}
      </span>
      <span style={{ ...pillStyle, color: "var(--color-text-secondary)" }}>
        Last: {lastMeetingText}
      </span>
      <span style={{ ...pillStyle, color: "var(--color-text-secondary)" }}>
        {signals.meetingFrequency30d} meeting{signals.meetingFrequency30d !== 1 ? "s" : ""} / 30d
      </span>
    </div>
  );
}

// =============================================================================
// User Editability Components (I194 / ADR-0065)
// =============================================================================

function UserNotesEditor({
  meetingId,
  initialNotes,
  isEditable,
  onSaveStatus,
}: {
  meetingId: string;
  initialNotes?: string;
  isEditable: boolean;
  onSaveStatus: (status: "idle" | "saving" | "saved") => void;
}) {
  const [notes, setNotes] = useState(initialNotes || "");
  const saveTimer = useRef<ReturnType<typeof setTimeout>>();

  // Don't render if no notes and not editable (past meeting)
  if (!isEditable && !notes) return null;

  function handleChange(value: string) {
    setNotes(value);

    if (saveTimer.current) clearTimeout(saveTimer.current);
    onSaveStatus("saving");
    saveTimer.current = setTimeout(async () => {
      try {
        await invoke("update_meeting_user_notes", { meetingId, notes: value });
        onSaveStatus("saved");
        setTimeout(() => onSaveStatus("idle"), 2000);
      } catch (err) {
        console.error("Save failed:", err);
        onSaveStatus("idle");
      }
    }, 1000);
  }

  return (
    <div
      style={{
        borderLeft: "3px solid rgba(201, 162, 39, 0.2)",
        paddingLeft: 20,
        paddingTop: 16,
        paddingBottom: 16,
      }}
    >
      <p
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "rgba(201, 162, 39, 0.7)",
          margin: "0 0 12px",
        }}
      >
        My Notes
      </p>
      {isEditable ? (
        <textarea
          value={notes}
          onChange={(e) => handleChange(e.target.value)}
          placeholder="Add your own notes for this meeting..."
          style={{
            width: "100%",
            minHeight: 80,
            border: "none",
            background: "transparent",
            padding: 0,
            fontSize: 14,
            lineHeight: 1.65,
            color: "var(--color-text-primary)",
            fontFamily: "var(--font-sans)",
            resize: "vertical",
            outline: "none",
          }}
        />
      ) : (
        <div
          style={{
            whiteSpace: "pre-wrap",
            fontSize: 14,
            lineHeight: 1.65,
            color: "var(--color-text-primary)",
          }}
        >
          {notes}
        </div>
      )}
    </div>
  );
}

function UserAgendaEditor({
  meetingId,
  initialAgenda,
  isEditable,
  onSaveStatus,
}: {
  meetingId: string;
  initialAgenda?: string[];
  isEditable: boolean;
  onSaveStatus: (status: "idle" | "saving" | "saved") => void;
}) {
  const [agenda, setAgenda] = useState(initialAgenda || []);
  const [newItem, setNewItem] = useState("");

  // Don't render if no agenda and not editable
  if (!isEditable && agenda.length === 0) return null;

  async function saveAgenda(updatedAgenda: string[]) {
    onSaveStatus("saving");
    try {
      await invoke("update_meeting_user_agenda", { meetingId, agenda: updatedAgenda });
      setAgenda(updatedAgenda);
      onSaveStatus("saved");
      setTimeout(() => onSaveStatus("idle"), 2000);
    } catch (err) {
      console.error("Save failed:", err);
      onSaveStatus("idle");
    }
  }

  function addItem() {
    if (!newItem.trim()) return;
    saveAgenda([...agenda, newItem.trim()]);
    setNewItem("");
  }

  function removeItem(index: number) {
    saveAgenda(agenda.filter((_, i) => i !== index));
  }

  return (
    <div
      style={{
        borderLeft: "3px solid rgba(201, 162, 39, 0.2)",
        paddingLeft: 20,
        paddingTop: 16,
        paddingBottom: 16,
      }}
    >
      <p
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "rgba(201, 162, 39, 0.7)",
          margin: "0 0 12px",
        }}
      >
        My Agenda
      </p>
      {agenda.length > 0 && (
        <ol style={{ listStyle: "none", margin: "0 0 16px", padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
          {agenda.map((item, i) => (
            <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  fontWeight: 500,
                  color: "rgba(201, 162, 39, 0.5)",
                  width: 16,
                  textAlign: "right",
                  flexShrink: 0,
                  paddingTop: 1,
                }}
              >
                {i + 1}.
              </span>
              <span style={{ flex: 1, fontSize: 14, lineHeight: 1.55, color: "var(--color-text-primary)" }}>{item}</span>
              {isEditable && (
                <button
                  onClick={() => removeItem(i)}
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
            </li>
          ))}
        </ol>
      )}
      {isEditable && (
        <div style={{ display: "flex", gap: 8 }}>
          <input
            value={newItem}
            onChange={(e) => setNewItem(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addItem()}
            placeholder="Add agenda item..."
            style={{
              flex: 1,
              border: "none",
              borderBottom: "1px solid var(--color-rule-light)",
              background: "transparent",
              padding: "4px 0",
              fontSize: 14,
              color: "var(--color-text-primary)",
              fontFamily: "var(--font-sans)",
              outline: "none",
            }}
          />
          <button
            onClick={addItem}
            style={{
              ...editorialBtn,
              color: "var(--color-spice-turmeric)",
              borderColor: "transparent",
              padding: "4px 12px",
            }}
          >
            Add
          </button>
        </div>
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
    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 4px" }}>
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

function CopyAllButton({ data }: { data: FullMeetingPrep }) {
  const { copied, copy } = useCopyToClipboard();

  return (
    <button
      onClick={() => copy(formatFullPrep(data))}
      style={{
        background: "none",
        border: "none",
        cursor: "pointer",
        padding: 4,
        color: "var(--color-text-tertiary)",
        flexShrink: 0,
      }}
    >
      {copied ? (
        <Check style={{ width: 14, height: 14, color: "var(--color-garden-sage)" }} />
      ) : (
        <Copy style={{ width: 14, height: 14 }} />
      )}
    </button>
  );
}

function PeopleInTheRoom({
  attendeeContext,
  attendees,
}: {
  attendeeContext?: AttendeeContext[];
  attendees?: Stakeholder[];
}) {
  if (attendeeContext && attendeeContext.length > 0) {
    return (
      <section>
        <SectionLabel
          label="People in the Room"
          icon={<Users style={{ width: 14, height: 14 }} />}
          copyText={formatAttendeeContext(attendeeContext)}
          copyLabel="people"
        />
        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 12 }}>
          {attendeeContext.map((person, i) => (
            <AttendeeRow key={i} person={person} />
          ))}
        </div>
      </section>
    );
  }

  if (attendees && attendees.length > 0) {
    return (
      <section>
        <SectionLabel
          label="Key Attendees"
          icon={<Users style={{ width: 14, height: 14 }} />}
          copyText={formatAttendees(attendees)}
          copyLabel="attendees"
        />
        <div style={{ display: "flex", flexDirection: "column", gap: 12, marginTop: 12 }}>
          {attendees.map((attendee, i) => (
            <div key={i} style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
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
                {attendee.name.charAt(0)}
              </div>
              <div>
                <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)", margin: 0 }}>
                  {attendee.name}
                </p>
                {attendee.role && (
                  <p style={{ fontSize: 13, color: "var(--color-text-tertiary)", margin: "2px 0 0" }}>
                    {attendee.role}
                  </p>
                )}
                {attendee.focus && (
                  <p style={{ fontSize: 13, color: "var(--color-text-tertiary)", margin: "2px 0 0" }}>
                    {attendee.focus}
                  </p>
                )}
              </div>
            </div>
          ))}
        </div>
      </section>
    );
  }

  return null;
}

function StakeholderInsightList({ people }: { people: StakeholderInsight[] }) {
  const engagementColor: Record<string, string> = {
    champion: "var(--color-garden-sage)",
    detractor: "var(--color-spice-terracotta)",
    neutral: "var(--color-text-tertiary)",
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
      {people.map((person, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 12,
            borderBottom: "1px solid var(--color-rule-light)",
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

function ExtendedStakeholderToggle({ people }: { people: StakeholderInsight[] }) {
  const [open, setOpen] = useState(false);

  return (
    <div>
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
          ...chapterHeading,
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
        Extended Stakeholder Map ({people.length})
      </button>
      {open && (
        <div style={{ marginTop: 16 }}>
          <StakeholderInsightList people={people} />
        </div>
      )}
    </div>
  );
}

function AttendeeRow({ person }: { person: AttendeeContext }) {
  const tempColorMap: Record<string, string> = {
    hot: "var(--color-garden-sage)",
    warm: "var(--color-spice-turmeric)",
    cool: "var(--color-text-tertiary)",
    cold: "var(--color-spice-terracotta)",
  };
  const tempColor = tempColorMap[person.temperature ?? ""] ?? "var(--color-text-tertiary)";

  const isNew = person.meetingCount === 0;
  const isCold = person.temperature === "cold";
  const lastSeenText = person.lastSeen ? formatRelativeDateLong(person.lastSeen) : undefined;

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
        padding: "8px 0",
        borderBottom: "1px solid var(--color-rule-light)",
      }}
    >
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
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <p style={{ fontWeight: 500, color: "var(--color-text-primary)", margin: 0, fontSize: 14 }}>
            {person.name}
          </p>
          {person.temperature && (
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
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12 }}>
          {person.role && (
            <p style={{ fontSize: 13, color: "var(--color-text-tertiary)", margin: 0 }}>
              {person.role}
            </p>
          )}
          {person.organization && (
            <p style={{ fontSize: 13, color: "var(--color-text-tertiary)", margin: 0 }}>
              {person.organization}
            </p>
          )}
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 12, marginTop: 2 }}>
          {person.meetingCount != null && person.meetingCount > 0 && (
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
              {person.meetingCount} meeting{person.meetingCount !== 1 ? "s" : ""}
            </span>
          )}
          {lastSeenText && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: isCold ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
              }}
            >
              Last seen {lastSeenText}
            </span>
          )}
        </div>
        {isCold && (
          <p
            style={{
              marginTop: 4,
              marginBottom: 0,
              fontSize: 12,
              color: "var(--color-spice-terracotta)",
            }}
          >
            Cold -- hasn't been seen in 60+ days
          </p>
        )}
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

  if (person.personId) {
    return (
      <Link
        to="/people/$personId"
        params={{ personId: person.personId }}
        style={{ textDecoration: "none", color: "inherit" }}
      >
        {inner}
      </Link>
    );
  }
  return inner;
}

function ActionItem({ action }: { action: ActionWithContext }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 10,
        borderBottom: action.isOverdue
          ? "1px solid var(--color-spice-terracotta)"
          : "1px solid var(--color-rule-light)",
        paddingBottom: 10,
        paddingLeft: action.isOverdue ? 12 : 0,
        borderLeft: action.isOverdue ? "3px solid var(--color-spice-terracotta)" : "none",
      }}
    >
      {action.isOverdue ? (
        <AlertTriangle style={{ width: 16, height: 16, color: "var(--color-spice-terracotta)", marginTop: 2, flexShrink: 0 }} />
      ) : (
        <CheckCircle style={{ width: 16, height: 16, color: "var(--color-text-tertiary)", marginTop: 2, flexShrink: 0 }} />
      )}
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
        borderBottom: "1px solid var(--color-rule-light)",
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
// Appendix Section (useState toggle replaces Collapsible)
// =============================================================================

function AppendixSection({ data }: { data: FullMeetingPrep }) {
  const [open, setOpen] = useState(false);

  return (
    <section id="appendix" style={{ borderTop: "1px solid var(--color-rule-heavy)", paddingTop: 24, display: "flex", flexDirection: "column", gap: 16 }}>
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
          ...chapterHeading,
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
        <div style={{ display: "flex", flexDirection: "column", gap: 28, marginTop: 8 }}>
          {data.sinceLast && data.sinceLast.length > 0 && (
            <section>
              <SectionLabel
                label="Since Last Meeting"
                icon={<History style={{ width: 14, height: 14 }} />}
                copyText={formatBulletList(data.sinceLast)}
                copyLabel="since last meeting"
              />
              <ul style={{ listStyle: "none", margin: "12px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
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
                icon={<Target style={{ width: 14, height: 14 }} />}
                copyText={formatBulletList(data.strategicPrograms)}
                copyLabel="programs"
              />
              <ul style={{ listStyle: "none", margin: "12px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
                {data.strategicPrograms.map((item, i) => (
                  <li key={i} style={{ display: "flex", alignItems: "flex-start", gap: 10, fontSize: 14, lineHeight: 1.65 }}>
                    <span
                      style={{
                        marginTop: 3,
                        fontSize: 14,
                        color: item.startsWith("\u2713") ? "var(--color-garden-sage)" : "var(--color-text-tertiary)",
                      }}
                    >
                      {item.startsWith("\u2713") ? "\u2713" : "\u25CB"}
                    </span>
                    <span style={{ color: "var(--color-text-primary)" }}>{item.replace(/^[\u2713\u25CB]\s*/, "")}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {data.meetingContext && data.meetingContext.split("\n").length > 3 && (
            <section>
              <SectionLabel
                label="Full Context"
                icon={<FileText style={{ width: 14, height: 14 }} />}
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
              <ul style={{ listStyle: "none", margin: "12px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
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
                icon={<HelpCircle style={{ width: 14, height: 14 }} />}
                copyText={formatNumberedList(data.questions)}
                copyLabel="questions"
              />
              <ol style={{ listStyle: "none", margin: "12px 0 0", padding: 0, display: "flex", flexDirection: "column", gap: 8 }}>
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
                icon={<BookOpen style={{ width: 14, height: 14 }} />}
                copyText={formatBulletList(data.keyPrinciples)}
                copyLabel="principles"
              />
              <div style={{ display: "flex", flexDirection: "column", gap: 12, marginTop: 12 }}>
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
// Helpers
// =============================================================================

function agendaSourceColor(source: string): string {
  switch (source) {
    case "calendar_note": return "var(--color-spice-turmeric)";
    case "risk": return "var(--color-spice-terracotta)";
    case "question": return "var(--color-text-tertiary)";
    case "open_item": return "var(--color-spice-turmeric)";
    case "talking_point": return "var(--color-garden-sage)";
    default: return "var(--color-text-tertiary)";
  }
}

function resolveMetaToneColor(tone?: string): string {
  if (!tone) return "var(--color-text-primary)";
  if (tone === "text-destructive") return "var(--color-spice-terracotta)";
  if (tone === "text-success") return "var(--color-garden-sage)";
  if (tone === "text-primary") return "var(--color-spice-turmeric)";
  return "var(--color-text-primary)";
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

function getHeroMetaItems(data: FullMeetingPrep): Array<{ label: string; value: string; tone?: string }> {
  const rows: Array<{ label: string; value: string; tone?: string }> = [];
  const add = (label: string, value: string | null | undefined) => {
    const clean = value ? sanitizeInlineText(value) : "";
    if (!clean) return;
    if (rows.some((r) => r.label === label)) return;
    rows.push({
      label,
      value: clean,
      tone: resolveMetaTone(label, clean),
    });
  };

  add(
    "Health",
    findSnapshotValue(data.accountSnapshot, ["health"]) ??
      findQuickContextValue(data.quickContext, "health"),
  );
  add(
    "ARR",
    findSnapshotValue(data.accountSnapshot, ["arr"]) ??
      findQuickContextValue(data.quickContext, "arr"),
  );
  add(
    "Renewal",
    findSnapshotValue(data.accountSnapshot, ["renewal"]) ??
      findQuickContextValue(data.quickContext, "renewal"),
  );
  add(
    "Ring",
    findSnapshotValue(data.accountSnapshot, ["ring"]) ??
      findQuickContextValue(data.quickContext, "ring"),
  );

  return rows.slice(0, 4);
}

function resolveMetaTone(label: string, value: string): string | undefined {
  const v = value.toLowerCase();
  if (label.toLowerCase() === "health") {
    if (v.includes("red") || v.includes("risk")) return "text-destructive";
    if (v.includes("green")) return "text-success";
    if (v.includes("yellow")) return "text-primary";
  }
  return undefined;
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
  return `${value.slice(0, maxChars - 1).trim()}\u2026`;
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

function formatQuickContext(items: [string, string][]): string {
  return items.map(([key, value]) => `${key}: ${value}`).join("\n");
}

function formatProposedAgenda(items: AgendaItem[]): string {
  return items
    .map((a, i) => {
      let line = `${i + 1}. ${cleanPrepLine(a.topic)}`;
      if (a.why) line += ` \u2014 ${cleanPrepLine(a.why)}`;
      return line;
    })
    .join("\n");
}

function formatAttendeeContext(people: AttendeeContext[]): string {
  return people
    .map((p) => {
      const parts = [p.name];
      if (p.role) parts.push(p.role);
      if (p.organization) parts.push(p.organization);
      const meta: string[] = [];
      if (p.temperature) meta.push(p.temperature);
      if (p.meetingCount != null) meta.push(`${p.meetingCount} meetings`);
      if (meta.length > 0) parts.push(`(${meta.join(", ")})`);
      return `- ${parts.join(" \u2014 ")}`;
    })
    .join("\n");
}

function formatAttendees(attendees: Stakeholder[]): string {
  return attendees
    .map((a) => {
      const parts = [a.name];
      if (a.role) parts.push(a.role);
      return `- ${parts.join(" \u2014 ")}`;
    })
    .join("\n");
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

function formatFullPrep(data: FullMeetingPrep): string {
  const sections: string[] = [];

  sections.push(`# ${data.title}`);
  if (data.timeRange) sections.push(data.timeRange);

  if (data.accountSnapshot && data.accountSnapshot.length > 0) {
    sections.push(`\n## Account Snapshot\n${data.accountSnapshot.map(s => `${s.label}: ${s.value}`).join("\n")}`);
  } else if (data.quickContext && data.quickContext.length > 0) {
    sections.push(`\n## Quick Context\n${formatQuickContext(data.quickContext)}`);
  }

  if (data.meetingContext) {
    sections.push(`\n## Context\n${data.meetingContext}`);
  }

  const calNotes = normalizeCalendarNotes(data.calendarNotes);
  if (calNotes) {
    sections.push(`\n## Calendar Notes\n${calNotes}`);
  }

  if (data.proposedAgenda && data.proposedAgenda.length > 0) {
    const cleanAgenda = data.proposedAgenda
      .map((item) => ({ ...item, topic: cleanPrepLine(item.topic), why: item.why ? cleanPrepLine(item.why) : undefined }))
      .filter((item) => item.topic.length > 0);
    if (cleanAgenda.length > 0) {
      sections.push(`\n## Agenda\n${formatProposedAgenda(cleanAgenda)}`);
    }
  }

  if (data.userAgenda && data.userAgenda.length > 0) {
    sections.push(`\n## My Agenda\n${data.userAgenda.map((a, i) => `${i + 1}. ${a}`).join("\n")}`);
  }

  if (data.userNotes) {
    sections.push(`\n## My Notes\n${data.userNotes}`);
  }

  if (data.attendeeContext && data.attendeeContext.length > 0) {
    sections.push(`\n## People in the Room\n${formatAttendeeContext(data.attendeeContext)}`);
  } else if (data.attendees && data.attendees.length > 0) {
    sections.push(`\n## Key Attendees\n${formatAttendees(data.attendees)}`);
  }

  if (data.sinceLast && data.sinceLast.length > 0) {
    sections.push(`\n## Since Last Meeting\n${formatBulletList(data.sinceLast)}`);
  }

  if (data.strategicPrograms && data.strategicPrograms.length > 0) {
    sections.push(`\n## Current Strategic Programs\n${formatBulletList(data.strategicPrograms)}`);
  }

  if (data.currentState && data.currentState.length > 0) {
    sections.push(`\n## Current State\n${formatBulletList(data.currentState)}`);
  }

  if (data.risks && data.risks.length > 0) {
    sections.push(`\n## Risks\n${formatBulletList(data.risks)}`);
  }

  const { wins: summaryWins } = deriveRecentWins(data);
  if (summaryWins.length > 0) {
    sections.push(`\n## Recent Wins\n${formatNumberedList(summaryWins)}`);
  }

  if (data.openItems && data.openItems.length > 0) {
    sections.push(`\n## Open Items\n${formatOpenItems(data.openItems)}`);
  }

  if (data.questions && data.questions.length > 0) {
    sections.push(`\n## Questions\n${formatNumberedList(data.questions)}`);
  }

  if (data.keyPrinciples && data.keyPrinciples.length > 0) {
    sections.push(`\n## Key Principles\n${formatBulletList(data.keyPrinciples)}`);
  }

  return sections.join("\n");
}

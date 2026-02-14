import { useState, useEffect, useRef, useCallback } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  AgendaDraftDialog,
  useAgendaDraft,
} from "@/components/ui/agenda-draft-dialog";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
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
} from "@/types";
import { cn, formatRelativeDateLong } from "@/lib/utils";
import { CopyButton } from "@/components/ui/copy-button";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { MeetingOutcomes } from "@/components/dashboard/MeetingOutcomes";
import {
  AlertCircle,
  ArrowLeft,
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
} from "lucide-react";

export default function MeetingDetailPage() {
  const { meetingId } = useParams({ strict: false });
  const [data, setData] = useState<FullMeetingPrep | null>(null);
  const [outcomes, setOutcomes] = useState<MeetingOutcomeData | null>(null);
  const [canEditUserLayer, setCanEditUserLayer] = useState(false);
  const [meetingMeta, setMeetingMeta] = useState<MeetingIntelligence["meeting"] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Transcript attach (from MeetingDetailPage)
  const [attaching, setAttaching] = useState(false);
  const draft = useAgendaDraft({ onError: setError });
  const [prefillNotice, setPrefillNotice] = useState(false);
  const [prefilling, setPrefilling] = useState(false);

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
      const formatRange = (startRaw?: string, endRaw?: string) => {
        if (!startRaw) return "";
        const start = new Date(startRaw);
        const startLabel = start.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
        if (!endRaw) return startLabel;
        const end = new Date(endRaw);
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
        end: meetingMeta?.endTime || meetingMeta?.startTime || new Date().toISOString(),
        type: "internal",
        attendees: [],
        isAllDay: false,
      };
      await invoke("attach_meeting_transcript", {
        filePath: selected,
        meeting: calendarEvent,
      });
      await loadMeetingIntelligence();
    } catch (err) {
      console.error("Failed to attach transcript:", err);
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

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-10 w-3/4" />
        <Skeleton className="mb-6 h-4 w-48" />
        <div className="space-y-4">
          <Skeleton className="h-32" />
          <Skeleton className="h-48" />
          <Skeleton className="h-32" />
        </div>
      </main>
    );
  }

  if (error || !data) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Link to="/">
          <Button variant="ghost" size="sm" className="mb-4">
            <ArrowLeft className="mr-2 size-4" />
            Back to Dashboard
          </Button>
        </Link>
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="size-5" />
              <p>{error || "Meeting prep not available"}</p>
            </div>
            <p className="mt-2 text-sm text-muted-foreground">
              This meeting doesn't have a prep file yet. The system generates prep
              files for customer meetings when running the Daily Briefing.
            </p>
          </CardContent>
        </Card>
      </main>
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
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="mx-auto max-w-6xl p-6 pb-16">
          {/* Back button */}
          <Link to="/">
            <Button variant="ghost" size="sm" className="mb-4">
              <ArrowLeft className="mr-2 size-4" />
              Back to Dashboard
            </Button>
          </Link>

          {/* Post-meeting: outcomes first (I195) */}
          {isPastMeeting && outcomes && (
            <>
              <OutcomesSection outcomes={outcomes} onRefresh={loadMeetingIntelligence} />
              <Separator className="my-8" />
              <p className="text-sm font-medium text-muted-foreground mb-4">
                Pre-Meeting Context
              </p>
            </>
          )}

          {/* Past meeting without outcomes: prompt to attach transcript */}
          {isPastMeeting && !outcomes && (
            <Card className="mb-6 border-dashed">
              <CardContent className="flex items-center justify-between py-4">
                <div className="text-sm text-muted-foreground">
                  <p className="font-medium text-foreground">No outcomes captured yet</p>
                  <p>Attach a transcript or manually capture meeting outcomes.</p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleAttachTranscript}
                  disabled={attaching}
                >
                  {attaching ? (
                    <Loader2 className="mr-2 size-4 animate-spin" />
                  ) : (
                    <Paperclip className="mr-2 size-4" />
                  )}
                  {attaching ? "Processing..." : "Attach Transcript"}
                </Button>
              </CardContent>
            </Card>
          )}

          {!hasAnyContent && !outcomes && (
            <div className="text-center py-12 text-muted-foreground">
              <Clock className="mx-auto mb-3 size-8 opacity-50" />
              <p className="text-lg font-medium">Prep is being generated</p>
              <p className="text-sm mt-2">
                Meeting context will appear here once AI enrichment completes.
              </p>
            </div>
          )}

          {(hasAnyContent || outcomes) && (
            <div className={cn(isPastMeeting && outcomes && "opacity-70")}>
              <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_260px]">
                <div className="space-y-10">
                  <section
                    id="executive-brief"
                    className="relative overflow-hidden rounded-2xl border border-border/70 bg-gradient-to-br from-card via-card to-primary/5 p-6"
                  >
                    <div className="absolute -right-10 -top-10 size-36 rounded-full bg-primary/10 blur-2xl" />
                    <div className="relative">
                      <div className="flex items-start justify-between gap-4">
                        <div className="space-y-2">
                          <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                            Meeting Intelligence Report
                          </p>
                          <div className="flex flex-wrap items-center gap-3">
                            <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
                              {data.title}
                            </h1>
                            {lifecycle && (
                              <Badge
                                variant="outline"
                                className="border-primary/30 bg-primary/10 text-primary font-medium tracking-wide"
                              >
                                {lifecycle}
                              </Badge>
                            )}
                          </div>
                          <p className="font-mono text-xs tracking-wide text-muted-foreground">
                            {data.timeRange}
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          {isEditable && (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={handlePrefillFromContext}
                              disabled={prefilling}
                            >
                              {prefilling ? "Prefilling..." : "Prefill Prep"}
                            </Button>
                          )}
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={handleDraftAgendaMessage}
                          >
                            Draft agenda message
                          </Button>
                          <CopyAllButton data={data} />
                        </div>
                      </div>

                      {(data.intelligenceSummary || data.meetingContext) ? (
                        <blockquote className="mt-6 border-l-2 border-l-primary/45 pl-5">
                          {(data.intelligenceSummary || data.meetingContext || "")
                            .split("\n")
                            .filter((line) => line.trim())
                            .slice(0, 3)
                            .map((line, i) => (
                              <p
                                key={i}
                                className={cn(
                                  "text-base leading-8 text-foreground/90 sm:text-[17px]",
                                  i > 0 && "mt-2",
                                )}
                              >
                                {line}
                              </p>
                            ))}
                        </blockquote>
                      ) : (
                        <p className="mt-6 text-sm text-muted-foreground/80">
                          Intelligence builds as you meet with this account.
                        </p>
                      )}

                      {topRisks.length > 0 && (
                        <div className="mt-6 grid gap-2 sm:grid-cols-3">
                          {topRisks.map((risk, i) => (
                            <div key={i} className="rounded-lg border border-destructive/20 bg-destructive/5 px-3 py-2">
                              <p className="text-[11px] font-semibold uppercase tracking-wider text-destructive/80">Risk {i + 1}</p>
                              <p className="mt-1 text-sm leading-relaxed">{risk}</p>
                            </div>
                          ))}
                        </div>
                      )}

                      {heroMeta.length > 0 && (
                        <div className="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
                          {heroMeta.map((item) => (
                            <div key={item.label} className="rounded-lg border border-border/70 bg-card/60 px-3 py-2">
                              <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                                {item.label}
                              </p>
                              <p className={cn("mt-1 text-sm font-medium", item.tone)}>
                                {item.value}
                              </p>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  </section>

                  <section id="agenda" className="space-y-5">
                    <SectionLabel
                      label="Agenda"
                      icon={<Target className="size-3.5" />}
                      copyText={agendaDisplayItems.length > 0 ? formatProposedAgenda(agendaDisplayItems) : undefined}
                      copyLabel="agenda"
                    />
                    {agendaDisplayItems.length > 0 ? (
                      <ol className="space-y-3">
                        {agendaDisplayItems.map((item, i) => (
                          <li key={i} className="flex items-start gap-3 rounded-lg border border-border/70 bg-card/50 p-3">
                            <span className="flex size-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary">
                              {i + 1}
                            </span>
                            <div className="flex-1 min-w-0">
                              <p className="text-sm font-medium leading-snug">{item.topic}</p>
                              {item.why && (
                                <p className="mt-1 text-xs text-muted-foreground">{item.why}</p>
                              )}
                            </div>
                            {item.source && (
                              <span
                                className={cn(
                                  "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium",
                                  item.source === "calendar_note" && "bg-amber-500/10 text-amber-700 dark:text-amber-300",
                                  item.source === "risk" && "bg-destructive/10 text-destructive",
                                  item.source === "question" && "bg-muted text-muted-foreground",
                                  item.source === "open_item" && "bg-primary/10 text-primary",
                                  item.source === "talking_point" && "bg-success/10 text-success",
                                )}
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
                      <div className="rounded-lg border border-dashed border-border/80 bg-card/40 px-4 py-3 text-sm text-muted-foreground">
                        No proposed agenda yet. Add your own agenda items below.
                      </div>
                    )}
                    {meetingId && (
                      <>
                        {prefillNotice && (
                          <div className="rounded-md border border-primary/25 bg-primary/10 px-3 py-2 text-xs text-primary">
                            Prefill appended new agenda/notes content.
                          </div>
                        )}
                      </>
                    )}
                    {meetingId && (
                      <UserAgendaEditor
                        meetingId={meetingId}
                        initialAgenda={data.userAgenda}
                        isEditable={isEditable}
                      />
                    )}
                    {meetingId && (
                      <UserNotesEditor
                        meetingId={meetingId}
                        initialNotes={data.userNotes}
                        isEditable={isEditable}
                      />
                    )}
                    {calendarNotes && (
                      <section>
                        <SectionLabel label="Calendar Notes" icon={<CalendarDays className="size-3.5" />} />
                        <p className="mt-3 whitespace-pre-wrap text-sm text-muted-foreground leading-relaxed">
                          {calendarNotes}
                        </p>
                      </section>
                    )}
                  </section>

                  {((data.entityRisks && data.entityRisks.length > 0) || (data.risks && data.risks.length > 0)) && (
                    <section id="risks" className="space-y-4">
                      <SectionLabel
                        label="Risks"
                        icon={<AlertTriangle className="size-3.5" />}
                        className="text-destructive"
                        copyText={formatBulletList([
                          ...(data.entityRisks?.map((r) => r.text) ?? []),
                          ...(data.risks ?? []),
                        ])}
                        copyLabel="risks"
                      />
                      <ul className="space-y-2.5">
                        {data.entityRisks?.map((risk, i) => (
                          <li key={`entity-${i}`} className="flex items-start gap-2.5 rounded-lg border border-border/70 bg-card/50 p-3 text-sm leading-relaxed">
                            <span
                              className={cn(
                                "mt-2 size-1.5 shrink-0 rounded-full",
                                risk.urgency === "high" ? "bg-destructive" : "bg-destructive/50",
                              )}
                            />
                            <span className="flex-1">{risk.text}</span>
                          </li>
                        ))}
                        {data.risks?.map((risk, i) => (
                          <li key={`ai-${i}`} className="flex items-start gap-2.5 rounded-lg border border-border/70 bg-card/50 p-3 text-sm leading-relaxed">
                            <span className="mt-2 size-1.5 shrink-0 rounded-full bg-destructive/50" />
                            <span>{risk}</span>
                          </li>
                        ))}
                      </ul>
                    </section>
                  )}

                  <section id="people" className="space-y-5">
                    <PeopleInTheRoom attendeeContext={data.attendeeContext} attendees={data.attendees} />

                    {matchingStakeholderInsights.length > 0 && (
                      <div className="space-y-3">
                        <p className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                          Attendee Intelligence
                        </p>
                        <StakeholderInsightList people={matchingStakeholderInsights} />
                      </div>
                    )}

                    {extendedStakeholderInsights.length > 0 && (
                      <Collapsible defaultOpen={false}>
                        <CollapsibleTrigger className="group flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground w-full py-1">
                          <ChevronRight className="size-3.5 transition-transform duration-200 [[data-state=open]>&]:rotate-90" />
                          Extended Stakeholder Map ({extendedStakeholderInsights.length})
                        </CollapsibleTrigger>
                        <CollapsibleContent className="mt-4">
                          <StakeholderInsightList people={extendedStakeholderInsights} />
                        </CollapsibleContent>
                      </Collapsible>
                    )}
                  </section>

                  {data.openItems && data.openItems.length > 0 && (
                    <section id="actions" className="space-y-8">
                      <section>
                        <SectionLabel
                          label="Open Items"
                          icon={<CheckCircle className="size-3.5" />}
                          copyText={formatOpenItems(data.openItems)}
                          copyLabel="open items"
                        />
                        <div className="mt-3 space-y-2">
                          {data.openItems.map((item, i) => (
                            <ActionItem key={i} action={item} />
                          ))}
                        </div>
                      </section>
                    </section>
                  )}

                  {(hasReferenceContent(data) || (data.sinceLast?.length ?? 0) > 0 || (data.strategicPrograms?.length ?? 0) > 0) && (
                    <section id="appendix" className="space-y-4 border-t border-border/70 pt-6">
                      <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                        Appendix
                      </p>
                      <Collapsible defaultOpen={false}>
                        <CollapsibleTrigger className="group flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground w-full py-1">
                          <ChevronRight className="size-3.5 transition-transform duration-200 [[data-state=open]>&]:rotate-90" />
                          Open Supporting Context
                        </CollapsibleTrigger>
                        <CollapsibleContent className="mt-4 space-y-6">
                          {data.sinceLast && data.sinceLast.length > 0 && (
                            <section>
                              <SectionLabel
                                label="Since Last Meeting"
                                icon={<History className="size-3.5" />}
                                copyText={formatBulletList(data.sinceLast)}
                                copyLabel="since last meeting"
                              />
                              <ul className="mt-3 space-y-2">
                                {data.sinceLast.map((item, i) => (
                                  <li key={i} className="flex items-start gap-2.5 text-sm leading-relaxed">
                                    <span className="mt-2 size-1.5 shrink-0 rounded-full bg-primary" />
                                    <span>{item}</span>
                                  </li>
                                ))}
                              </ul>
                            </section>
                          )}

                          {data.strategicPrograms && data.strategicPrograms.length > 0 && (
                            <section>
                              <SectionLabel
                                label="Strategic Programs"
                                icon={<Target className="size-3.5" />}
                                copyText={formatBulletList(data.strategicPrograms)}
                                copyLabel="programs"
                              />
                              <ul className="mt-3 space-y-2">
                                {data.strategicPrograms.map((item, i) => (
                                  <li key={i} className="flex items-start gap-2.5 text-sm leading-relaxed">
                                    <span className={cn("mt-0.5", item.startsWith("✓") ? "text-success" : "text-muted-foreground")}>
                                      {item.startsWith("✓") ? "✓" : "○"}
                                    </span>
                                    <span>{item.replace(/^[✓○]\s*/, "")}</span>
                                  </li>
                                ))}
                              </ul>
                            </section>
                          )}

                          {data.meetingContext && data.meetingContext.split("\n").length > 3 && (
                            <section>
                              <SectionLabel
                                label="Full Context"
                                icon={<FileText className="size-3.5" />}
                                copyText={data.meetingContext}
                                copyLabel="context"
                              />
                              <p className="mt-3 whitespace-pre-wrap text-sm leading-relaxed">{data.meetingContext}</p>
                            </section>
                          )}

                          {data.currentState && data.currentState.length > 0 && (
                            <section>
                              <SectionLabel label="Current State" copyText={formatBulletList(data.currentState)} copyLabel="current state" />
                              <ul className="mt-3 space-y-2">
                                {data.currentState.map((item, i) => (
                                  <li key={i} className="flex items-start gap-2.5 text-sm leading-relaxed">
                                    <span className="mt-2 size-1.5 shrink-0 rounded-full bg-muted-foreground" />
                                    <span>{item}</span>
                                  </li>
                                ))}
                              </ul>
                            </section>
                          )}

                          {data.questions && data.questions.length > 0 && (
                            <section>
                              <SectionLabel
                                label="Questions to Surface"
                                icon={<HelpCircle className="size-3.5" />}
                                copyText={formatNumberedList(data.questions)}
                                copyLabel="questions"
                              />
                              <ol className="mt-3 space-y-2">
                                {data.questions.map((q, i) => (
                                  <li key={i} className="flex items-start gap-2.5 text-sm leading-relaxed">
                                    <span className="mt-0.5 text-xs font-medium text-muted-foreground w-4 shrink-0 text-right">{i + 1}.</span>
                                    <span>{q}</span>
                                  </li>
                                ))}
                              </ol>
                            </section>
                          )}

                          {data.keyPrinciples && data.keyPrinciples.length > 0 && (
                            <section>
                              <SectionLabel
                                label="Key Principles"
                                icon={<BookOpen className="size-3.5" />}
                                copyText={formatBulletList(data.keyPrinciples)}
                                copyLabel="principles"
                              />
                              <div className="mt-3 space-y-3">
                                {data.keyPrinciples.map((principle, i) => (
                                  <blockquote key={i} className="border-l-2 border-primary/30 pl-4 text-sm italic text-muted-foreground">
                                    {principle}
                                  </blockquote>
                                ))}
                              </div>
                            </section>
                          )}

                          {data.references && data.references.length > 0 && (
                            <section>
                              <SectionLabel label="References" />
                              <div className="mt-3 space-y-2">
                                {data.references.map((ref_, i) => (
                                  <ReferenceRow key={i} reference={ref_} />
                                ))}
                              </div>
                            </section>
                          )}
                        </CollapsibleContent>
                      </Collapsible>
                    </section>
                  )}

                  <div className="mt-12 flex items-center gap-3">
                    <div className="h-px flex-1 bg-border" />
                    <span className="text-[10px] font-semibold uppercase tracking-[0.2em] text-muted-foreground/40">
                      End of Brief
                    </span>
                    <div className="h-px flex-1 bg-border" />
                  </div>
                </div>

                <aside className="hidden lg:block">
                  <div className="sticky top-6 space-y-5">
                    <section className="rounded-xl border border-border/70 bg-card/60 p-4">
                      <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">Jump To</p>
                      <nav className="mt-3 space-y-2">
                        {reportNav.map((item) => (
                          <a
                            key={item.id}
                            href={`#${item.id}`}
                            className="block rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-primary/10 hover:text-foreground"
                          >
                            {item.label}
                          </a>
                        ))}
                      </nav>
                    </section>

                    {data.stakeholderSignals && (
                      <section className="rounded-xl border border-border/70 bg-card/60 p-4">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">Relationship Signals</p>
                        <div className="mt-3">
                          <RelationshipPills signals={data.stakeholderSignals} />
                        </div>
                      </section>
                    )}

                    {data.entityReadiness && data.entityReadiness.length > 0 && (
                      <section className="rounded-xl border border-primary/20 bg-primary/[0.05] p-4">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-primary/80">Before This Meeting</p>
                        <ul className="mt-3 space-y-2">
                          {data.entityReadiness.slice(0, 4).map((item, i) => (
                            <li key={i} className="flex items-start gap-2 text-sm">
                              <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary/60" />
                              <span>{item}</span>
                            </li>
                          ))}
                        </ul>
                      </section>
                    )}

                    {data.recentEmailSignals && data.recentEmailSignals.length > 0 && (
                      <section className="rounded-xl border border-amber-500/20 bg-amber-500/[0.08] p-4">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-amber-700 dark:text-amber-300">
                          Recent Email Signals
                        </p>
                        <ul className="mt-3 space-y-2.5">
                          {data.recentEmailSignals.slice(0, 4).map((signal, i) => (
                            <li key={`${signal.id ?? i}-${signal.signalType}`} className="text-sm">
                              <div className="flex items-center justify-between gap-2">
                                <span className="rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-amber-700 dark:text-amber-200">
                                  {signal.signalType}
                                </span>
                                {signal.detectedAt && (
                                  <span className="text-[10px] text-muted-foreground">
                                    {formatRelativeDateLong(signal.detectedAt)}
                                  </span>
                                )}
                              </div>
                              <p className="mt-1 leading-relaxed">{signal.signalText}</p>
                            </li>
                          ))}
                        </ul>
                      </section>
                    )}

                    {recentWinsForSidebar.length > 0 && (
                      <section className="rounded-xl border border-success/20 bg-success/[0.08] p-4">
                        <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-success">
                          Recent Wins
                        </p>
                        <ul className="mt-3 space-y-2.5">
                          {recentWinsForSidebar.slice(0, 4).map((win, i) => (
                            <li key={i} className="flex items-start gap-2 text-sm">
                              <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-success/70" />
                              <span className="leading-relaxed text-foreground">{win}</span>
                            </li>
                          ))}
                        </ul>
                      </section>
                    )}
                  </div>
                </aside>
              </div>
            </div>
          )}

          {/* Pre-meeting: outcomes below prep if they exist (I195) */}
          {!isPastMeeting && outcomes && (
            <>
              <Separator className="my-8" />
              <OutcomesSection outcomes={outcomes} onRefresh={loadMeetingIntelligence} />
            </>
          )}
        </div>
      </ScrollArea>
      <AgendaDraftDialog
        open={draft.open}
        onOpenChange={draft.setOpen}
        loading={draft.loading}
        subject={draft.subject}
        body={draft.body}
      />
    </main>
  );
}

/** Thin section label used throughout report sections. */
function SectionLabel({
  label,
  icon,
  className,
  copyText,
  copyLabel,
}: {
  label: string;
  icon?: React.ReactNode;
  className?: string;
  copyText?: string;
  copyLabel?: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <div className={cn(
        "flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground",
        className,
      )}>
        {icon}
        {label}
      </div>
      {copyText && (
        <CopyButton text={copyText} label={copyLabel} className="ml-auto" />
      )}
    </div>
  );
}

function RelationshipPills({ signals }: { signals: StakeholderSignals }) {
  const tempColor = {
    hot: "text-success",
    warm: "text-primary",
    cool: "text-muted-foreground",
    cold: "text-destructive",
  }[signals.temperature] ?? "text-muted-foreground";

  const lastMeetingText = signals.lastMeeting
    ? formatRelativeDateLong(signals.lastMeeting)
    : "No meetings recorded";

  return (
    <div className="flex flex-wrap gap-2">
      <Badge variant="outline" className={cn("font-normal capitalize", tempColor)}>
        {signals.temperature}
      </Badge>
      <Badge variant="outline" className="font-normal">
        Last: {lastMeetingText}
      </Badge>
      <Badge variant="outline" className="font-normal">
        {signals.meetingFrequency30d} meeting{signals.meetingFrequency30d !== 1 ? "s" : ""} / 30d
      </Badge>
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
}: {
  meetingId: string;
  initialNotes?: string;
  isEditable: boolean;
}) {
  const [notes, setNotes] = useState(initialNotes || "");
  const [saving, setSaving] = useState(false);
  const saveTimer = useRef<ReturnType<typeof setTimeout>>();

  // Don't render if no notes and not editable (past meeting)
  if (!isEditable && !notes) return null;

  function handleChange(value: string) {
    setNotes(value);

    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(async () => {
      setSaving(true);
      try {
        await invoke("update_meeting_user_notes", { meetingId, notes: value });
      } catch (err) {
        console.error("Save failed:", err);
      } finally {
        setSaving(false);
      }
    }, 1000);
  }

  return (
    <div className="rounded-lg border border-primary/10 bg-primary/[0.03] p-5">
      <div className="flex items-center gap-2 mb-3">
        <span className="text-[11px] font-semibold uppercase tracking-wider text-primary/70">My Notes</span>
        {saving && <span className="text-[10px] text-muted-foreground animate-pulse">Saving...</span>}
      </div>
      {isEditable ? (
        <textarea
          value={notes}
          onChange={(e) => handleChange(e.target.value)}
          className="w-full min-h-[80px] rounded-md border-0 bg-transparent p-0 text-sm leading-relaxed placeholder:text-muted-foreground/50 focus:outline-none focus:ring-0 resize-y"
          placeholder="Add your own notes for this meeting..."
        />
      ) : (
        <div className="whitespace-pre-wrap text-sm leading-relaxed">{notes}</div>
      )}
    </div>
  );
}

function UserAgendaEditor({
  meetingId,
  initialAgenda,
  isEditable,
}: {
  meetingId: string;
  initialAgenda?: string[];
  isEditable: boolean;
}) {
  const [agenda, setAgenda] = useState(initialAgenda || []);
  const [newItem, setNewItem] = useState("");
  const [saving, setSaving] = useState(false);

  // Don't render if no agenda and not editable
  if (!isEditable && agenda.length === 0) return null;

  async function saveAgenda(updatedAgenda: string[]) {
    setSaving(true);
    try {
      await invoke("update_meeting_user_agenda", { meetingId, agenda: updatedAgenda });
      setAgenda(updatedAgenda);
    } catch (err) {
      console.error("Save failed:", err);
    } finally {
      setSaving(false);
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
    <div className="rounded-lg border border-primary/10 bg-primary/[0.03] p-5">
      <div className="flex items-center gap-2 mb-3">
        <span className="text-[11px] font-semibold uppercase tracking-wider text-primary/70">My Agenda</span>
        {saving && <span className="text-[10px] text-muted-foreground animate-pulse">Saving...</span>}
      </div>
      {agenda.length > 0 && (
        <ol className="space-y-2 mb-4">
          {agenda.map((item, i) => (
            <li key={i} className="flex items-start gap-2.5">
              <span className="text-xs font-medium text-primary/50 w-4 shrink-0 text-right mt-0.5">{i + 1}.</span>
              <span className="flex-1 text-sm leading-relaxed">{item}</span>
              {isEditable && (
                <Button size="sm" variant="ghost" className="h-5 w-5 p-0 opacity-0 hover:opacity-100 transition-opacity" onClick={() => removeItem(i)}>
                  <span className="text-muted-foreground hover:text-foreground">&times;</span>
                </Button>
              )}
            </li>
          ))}
        </ol>
      )}
      {isEditable && (
        <div className="flex gap-2">
          <Input
            value={newItem}
            onChange={(e) => setNewItem(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addItem()}
            placeholder="Add agenda item..."
            className="h-8 border-0 bg-transparent text-sm shadow-none placeholder:text-muted-foreground/50 focus-visible:ring-0"
          />
          <Button size="sm" variant="ghost" className="h-8 text-primary hover:text-primary" onClick={addItem}>Add</Button>
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
}: {
  outcomes: MeetingOutcomeData;
  onRefresh: () => void;
}) {
  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold">Meeting Outcomes</h2>
      <MeetingOutcomes outcomes={outcomes} onRefresh={onRefresh} />
    </div>
  );
}

// =============================================================================
// Shared Components
// =============================================================================

function CopyAllButton({ data }: { data: FullMeetingPrep }) {
  const { copied, copy } = useCopyToClipboard();

  return (
    <Button
      variant="ghost"
      size="sm"
      className="text-muted-foreground hover:text-foreground shrink-0"
      onClick={() => copy(formatFullPrep(data))}
    >
      {copied ? (
        <Check className="size-3.5 text-success" />
      ) : (
        <Copy className="size-3.5" />
      )}
    </Button>
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
          icon={<Users className="size-3.5" />}
          copyText={formatAttendeeContext(attendeeContext)}
          copyLabel="people"
        />
        <div className="mt-3 space-y-3">
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
          icon={<Users className="size-3.5" />}
          copyText={formatAttendees(attendees)}
          copyLabel="attendees"
        />
        <div className="mt-3 space-y-3">
          {attendees.map((attendee, i) => (
            <div key={i} className="flex items-start gap-3">
              <div className="flex size-7 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
                {attendee.name.charAt(0)}
              </div>
              <div>
                <p className="text-sm font-medium">{attendee.name}</p>
                {attendee.role && (
                  <p className="text-xs text-muted-foreground">{attendee.role}</p>
                )}
                {attendee.focus && (
                  <p className="text-xs text-muted-foreground">{attendee.focus}</p>
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
  return (
    <div className="space-y-3">
      {people.map((person, i) => (
        <div key={i} className="flex items-start gap-3 rounded-lg border border-border/70 bg-card/50 p-3">
          <div className="flex size-7 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary shrink-0">
            {person.name.charAt(0)}
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="text-sm font-medium">{person.name}</p>
              {person.role && (
                <span className="text-xs text-muted-foreground">{sanitizeInlineText(person.role)}</span>
              )}
              {person.engagement && (
                <span
                  className={cn(
                    "text-[10px] font-medium capitalize",
                    person.engagement === "champion" && "text-success",
                    person.engagement === "detractor" && "text-destructive",
                    person.engagement === "neutral" && "text-muted-foreground",
                  )}
                >
                  {person.engagement}
                </span>
              )}
            </div>
            {person.assessment && (
              <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
                {truncateText(sanitizeInlineText(person.assessment), 180)}
              </p>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

function AttendeeRow({ person }: { person: AttendeeContext }) {
  const tempColor = {
    hot: "text-success",
    warm: "text-primary",
    cool: "text-muted-foreground",
    cold: "text-destructive",
  }[person.temperature ?? ""] ?? "text-muted-foreground";

  const isNew = person.meetingCount === 0;
  const isCold = person.temperature === "cold";
  const lastSeenText = person.lastSeen ? formatRelativeDateLong(person.lastSeen) : undefined;

  const inner = (
    <div className={cn(
      "flex items-start gap-3 rounded-md p-2 -mx-2",
      person.personId && "hover:bg-muted/50 cursor-pointer",
    )}>
      <div className={cn(
        "flex size-8 items-center justify-center rounded-full text-sm font-medium",
        isCold ? "bg-destructive/10 text-destructive" :
        isNew ? "bg-success/10 text-success" :
        "bg-primary/10 text-primary",
      )}>
        {person.name.charAt(0)}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="font-medium">{person.name}</p>
          {person.temperature && (
            <span className={cn("text-xs font-medium capitalize", tempColor)}>
              {person.temperature}
            </span>
          )}
          {isNew && (
            <span className="text-xs font-medium text-success">New contact</span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5">
          {person.role && (
            <p className="text-sm text-muted-foreground">{person.role}</p>
          )}
          {person.organization && (
            <p className="text-sm text-muted-foreground">{person.organization}</p>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-0.5 mt-0.5">
          {person.meetingCount != null && person.meetingCount > 0 && (
            <span className="text-xs text-muted-foreground">
              {person.meetingCount} meeting{person.meetingCount !== 1 ? "s" : ""}
            </span>
          )}
          {lastSeenText && (
            <span className={cn("text-xs", isCold ? "text-destructive" : "text-muted-foreground")}>
              Last seen {lastSeenText}
            </span>
          )}
        </div>
        {isCold && (
          <p className="mt-1 text-xs text-destructive">
            Cold — hasn't been seen in 60+ days
          </p>
        )}
        {person.notes && (
          <p className="mt-1 text-xs text-muted-foreground italic">{person.notes}</p>
        )}
      </div>
    </div>
  );

  if (person.personId) {
    return <Link to="/people/$personId" params={{ personId: person.personId }}>{inner}</Link>;
  }
  return inner;
}

function ActionItem({ action }: { action: ActionWithContext }) {
  return (
    <div
      className={cn(
        "rounded-md border p-3",
        action.isOverdue && "border-destructive bg-destructive/5"
      )}
    >
      <div className="flex items-start gap-2">
        {action.isOverdue ? (
          <AlertTriangle className="mt-0.5 size-4 text-destructive" />
        ) : (
          <CheckCircle className="mt-0.5 size-4 text-muted-foreground" />
        )}
        <div className="flex-1">
          <p className="font-medium">{action.title}</p>
          {action.dueDate && (
            <p
              className={cn(
                "text-sm",
                action.isOverdue ? "text-destructive" : "text-muted-foreground"
              )}
            >
              Due: {action.dueDate}
            </p>
          )}
          {action.context && (
            <p className="mt-1 text-sm text-muted-foreground">{action.context}</p>
          )}
        </div>
      </div>
    </div>
  );
}

function ReferenceRow({ reference }: { reference: SourceReference }) {
  return (
    <div className="flex items-center justify-between rounded-md bg-muted/50 p-2">
      <div>
        <p className="text-sm font-medium">{reference.label}</p>
        {reference.path && (
          <p className="font-mono text-xs text-muted-foreground">
            {reference.path}
          </p>
        )}
      </div>
      {reference.lastUpdated && (
        <span className="text-xs text-muted-foreground">
          {reference.lastUpdated}
        </span>
      )}
    </div>
  );
}

// =============================================================================
// Helpers
// =============================================================================

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

function formatQuickContext(items: [string, string][]): string {
  return items.map(([key, value]) => `${key}: ${value}`).join("\n");
}

function formatProposedAgenda(items: AgendaItem[]): string {
  return items
    .map((a, i) => {
      let line = `${i + 1}. ${cleanPrepLine(a.topic)}`;
      if (a.why) line += ` — ${cleanPrepLine(a.why)}`;
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
      return `- ${parts.join(" — ")}`;
    })
    .join("\n");
}

function formatAttendees(attendees: Stakeholder[]): string {
  return attendees
    .map((a) => {
      const parts = [a.name];
      if (a.role) parts.push(a.role);
      return `- ${parts.join(" — ")}`;
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

  const calendarNotes = normalizeCalendarNotes(data.calendarNotes);
  if (calendarNotes) {
    sections.push(`\n## Calendar Notes\n${calendarNotes}`);
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

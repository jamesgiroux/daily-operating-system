import * as React from "react";
import { Link } from "@tanstack/react-router";
import { emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  Building2,
  Check,
  ChevronDown,
  FileText,
  FolderKanban,
  Loader2,
  Paperclip,
  Trophy,
  X,
} from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useMeetingOutcomes } from "@/hooks/useMeetingOutcomes";
import { MeetingOutcomes } from "./MeetingOutcomes";
import { EntityPicker } from "@/components/ui/entity-picker";
import type { Meeting, MeetingType, CalendarEvent, LinkedEntity } from "@/types";
import { cn } from "@/lib/utils";

interface MeetingCardProps {
  meeting: Meeting;
  /** Live epoch ms from useCalendar — enables reactive isPast/isLive without per-card intervals. */
  now?: number;
  /** Current calendar event from useCalendar — used for live glow matching. */
  currentMeeting?: CalendarEvent;
}

const borderStyles: Partial<Record<MeetingType, string>> = {
  customer: "border-l-4 border-l-primary",
  qbr: "border-l-4 border-l-primary",
  partnership: "border-l-4 border-l-primary",
  external: "border-l-4 border-l-primary",
  personal: "border-l-4 border-l-success",
};

const badgeStyles: Partial<Record<MeetingType, string>> = {
  customer: "bg-primary/15 text-primary hover:bg-primary/20",
  qbr: "bg-primary/15 text-primary hover:bg-primary/20",
  partnership: "bg-primary/15 text-primary hover:bg-primary/20",
  external: "bg-primary/15 text-primary hover:bg-primary/20",
  personal: "bg-success/15 text-success hover:bg-success/20",
};

const badgeLabels: Partial<Record<MeetingType, string>> = {
  customer: "Customer",
  qbr: "QBR",
  partnership: "Partnership",
  external: "External",
  internal: "Internal",
  team_sync: "Team Sync",
  one_on_one: "1:1",
  all_hands: "All Hands",
  training: "Training",
  personal: "Personal",
};

export const dotColors: Partial<Record<MeetingType, string>> = {
  customer: "bg-primary",
  qbr: "bg-primary",
  partnership: "bg-primary",
  external: "bg-primary",
  personal: "bg-success",
};

// --- Display state types and pure function ---

export interface DisplayStateContext {
  isPast: boolean;
  outcomesStatus: "loaded" | "none" | "loading" | "unknown";
  isLive: boolean;
  hasInlinePrep: boolean;
  hasEnrichedPrep: boolean;
}

interface BadgeState {
  key: string;
  label: string;
  variant: "outline" | "secondary";
  className: string;
  icon?: "check";
}

interface ActionState {
  key: "view-prep" | "attach-transcript" | "capture-outcomes";
  label: string;
  linkTo?: string;
}

export interface MeetingDisplayState {
  primaryStatus:
    | "cancelled"
    | "live"
    | "processed"
    | "past-unprocessed"
    | "past-loading"
    | "new"
    | "has-prep"
    | "no-prep"
    | "default";
  card: { className: string; hoverEnabled: boolean };
  title: { lineThrough: boolean };
  badges: BadgeState[];
  actions: ActionState[];
  showExpander: boolean;
  dot: { bgClass: string; ringClass: string; animate: boolean };
}

export function computeMeetingDisplayState(
  meeting: Meeting,
  ctx: DisplayStateContext,
): MeetingDisplayState {
  const isCancelled = meeting.overlayStatus === "cancelled";
  const isNew = meeting.overlayStatus === "new";
  const hasPrepContext = meeting.hasPrep && !!meeting.id;

  // Dot styling (always computed — used by MeetingTimeline)
  const dotBg = isCancelled
    ? "bg-muted-foreground/30"
    : (dotColors[meeting.type] ?? "bg-muted-foreground/50");
  const dotRing =
    !isCancelled && ctx.isLive ? "ring-2 ring-primary/50" : "";
  const dotAnimate = !isCancelled && ctx.isLive;

  // --- Priority chain ---

  // 1. Cancelled gates everything
  if (isCancelled) {
    return {
      primaryStatus: "cancelled",
      card: {
        className: cn(
          "rounded-lg border bg-card shadow-sm transition-all duration-150",
          borderStyles[meeting.type],
          "opacity-50",
        ),
        hoverEnabled: false,
      },
      title: { lineThrough: true },
      badges: [
        {
          key: "cancelled",
          label: "Cancelled",
          variant: "outline",
          className: "text-destructive border-destructive/30",
        },
      ],
      actions: [],
      showExpander: false,
      dot: { bgClass: dotBg, ringClass: dotRing, animate: dotAnimate },
    };
  }

  const badges: BadgeState[] = [];
  const actions: ActionState[] = [];
  let primaryStatus: MeetingDisplayState["primaryStatus"] = "default";

  // Live ring is additive — computed via card className, not a primaryStatus
  const liveRing =
    ctx.isLive ? "animate-pulse-gold ring-2 ring-primary/50" : "";

  const cardClassName = cn(
    "rounded-lg border bg-card shadow-sm transition-all duration-150",
    "hover:-translate-y-0.5 hover:shadow-md",
    borderStyles[meeting.type],
    liveRing,
  );

  // 2. Past + outcomes loaded → "processed" badge (no actions)
  if (ctx.isPast && ctx.outcomesStatus === "loaded") {
    primaryStatus = "processed";
    badges.push({
      key: "processed",
      label: "Processed",
      variant: "outline",
      className: "text-success border-success/30",
      icon: "check",
    });
  }
  // 3. Past + outcomes loading → no badge, no buttons (prevents flash)
  else if (ctx.isPast && ctx.outcomesStatus === "loading") {
    primaryStatus = "past-loading";
  }
  // 4. Past + no outcomes → View Prep (if available) + Attach/Outcomes buttons
  else if (ctx.isPast && ctx.outcomesStatus === "none") {
    primaryStatus = "past-unprocessed";
    if (hasPrepContext) {
      actions.push({
        key: "view-prep",
        label: "View Prep",
        linkTo: meeting.id,
      });
    }
    actions.push(
      { key: "attach-transcript", label: "Attach" },
      { key: "capture-outcomes", label: "Outcomes" },
    );
  }
  // 5. New → "No prep available" badge
  else if (isNew) {
    primaryStatus = "new";
    badges.push({
      key: "new",
      label: "No prep available",
      variant: "outline",
      className: "text-muted-foreground",
    });
  }
  // 6. Has prep file → "View Prep" action + optional "Limited prep" badge
  else if (hasPrepContext) {
    primaryStatus = "has-prep";
    actions.push({
      key: "view-prep",
      label: "View Prep",
      linkTo: meeting.id,
    });
    if (!ctx.hasEnrichedPrep) {
      badges.push({
        key: "limited-prep",
        label: "Limited prep",
        variant: "outline",
        className: "text-destructive/70 border-destructive/30",
      });
    }
  }
  // 7. Customer without prep → "No prep" badge
  else if (!meeting.hasPrep && meeting.type === "customer") {
    primaryStatus = "no-prep";
    badges.push({
      key: "no-prep",
      label: "No prep",
      variant: "outline",
      className: "text-muted-foreground",
    });
  }

  // If live was the only signal, mark it
  if (primaryStatus === "default" && ctx.isLive) {
    primaryStatus = "live";
  }

  // Cancelable — secondary badge, additive after primary chain
  // (cancelled already returned early above, so no need to re-check)
  const CANCELABLE_TYPES: MeetingType[] = ["internal", "team_sync"];
  if (CANCELABLE_TYPES.includes(meeting.type) && !meeting.hasPrep) {
    badges.push({
      key: "cancelable",
      label: "Cancelable",
      variant: "outline",
      className: "text-muted-foreground",
    });
  }

  const showExpander =
    ctx.hasInlinePrep || ctx.outcomesStatus === "loaded";

  return {
    primaryStatus,
    card: { className: cardClassName, hoverEnabled: true },
    title: { lineThrough: false },
    badges,
    actions,
    showExpander,
    dot: { bgClass: dotBg, ringClass: dotRing, animate: dotAnimate },
  };
}

/** Parse a display time like "10:30 AM" to epoch ms (today). */
function parseDisplayTimeMs(timeStr: string | undefined): number | null {
  if (!timeStr) return null;
  const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
  if (!match) return null;
  let hours = parseInt(match[1], 10);
  const minutes = parseInt(match[2], 10);
  const period = match[3].toUpperCase();
  if (period === "PM" && hours !== 12) hours += 12;
  if (period === "AM" && hours === 12) hours = 0;
  const d = new Date();
  d.setHours(hours, minutes, 0, 0);
  return d.getTime();
}

function getMeetingEndMs(meeting: Meeting): number | null {
  return parseDisplayTimeMs(meeting.endTime) ?? parseDisplayTimeMs(meeting.time);
}

function getMeetingStartMs(meeting: Meeting): number | null {
  return parseDisplayTimeMs(meeting.time);
}

export function MeetingCard({ meeting, now: nowProp, currentMeeting: currentMeetingProp }: MeetingCardProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [attaching, setAttaching] = React.useState(false);

  // Use provided live clock or fall back to Date.now() (static — no interval)
  const now = nowProp ?? Date.now();

  // Compute live state from actual current time (not stale JSON flag)
  const isLive = React.useMemo(() => {
    // First: match against the live calendar's current meeting
    if (currentMeetingProp) {
      if (meeting.calendarEventId && meeting.calendarEventId === currentMeetingProp.id) return true;
      if (meeting.title === currentMeetingProp.title || meeting.id === currentMeetingProp.id) return true;
    }
    // Fallback: compare parsed display times against live clock
    const startMs = getMeetingStartMs(meeting);
    const endMs = getMeetingEndMs(meeting);
    if (startMs && endMs) return startMs <= now && now < endMs;
    return false;
  }, [currentMeetingProp, meeting, now]);

  const isPast = React.useMemo(() => {
    const endMs = getMeetingEndMs(meeting);
    return endMs ? now > endMs : false;
  }, [meeting, now]);

  // Local entity state for optimistic updates (multi-entity)
  const [localEntities, setLocalEntities] = React.useState<LinkedEntity[]>(
    meeting.linkedEntities ?? []
  );
  const [suggestedUnarchiveAccountId, setSuggestedUnarchiveAccountId] = React.useState<string | null>(
    meeting.suggestedUnarchiveAccountId ?? null,
  );
  const [restoringSuggestion, setRestoringSuggestion] = React.useState(false);
  const [suggestionError, setSuggestionError] = React.useState<string | null>(null);

  // Sync from props when meeting data refreshes (e.g., dashboard reload)
  React.useEffect(() => {
    setLocalEntities(meeting.linkedEntities ?? []);
  }, [meeting.linkedEntities]);

  React.useEffect(() => {
    setSuggestedUnarchiveAccountId(meeting.suggestedUnarchiveAccountId ?? null);
    setSuggestionError(null);
  }, [meeting.suggestedUnarchiveAccountId]);

  const handleAddEntity = React.useCallback(
    async (newId: string | null, name?: string) => {
      if (!newId || !name) return;
      // Skip if already linked
      if (localEntities.some((e) => e.id === newId)) return;

      const newEntity: LinkedEntity = { id: newId, name, entityType: "account" };
      // Optimistic add
      setLocalEntities((prev) => [...prev, newEntity]);

      try {
        await invoke("add_meeting_entity", {
          meetingId: meeting.id,
          entityId: newId,
          entityType: "account",
          meetingTitle: meeting.title,
          startTime: meeting.startIso ?? meeting.time,
          meetingTypeStr: meeting.type,
        });
        emit("entity-updated");
      } catch (err) {
        // Revert on failure
        setLocalEntities((prev) => prev.filter((e) => e.id !== newId));
        console.error("Failed to add meeting entity:", err);
      }
    },
    [meeting.id, meeting.title, meeting.startIso, meeting.time, meeting.type, localEntities]
  );

  const handleRemoveEntity = React.useCallback(
    async (entityId: string, entityType: string) => {
      // Optimistic remove
      setLocalEntities((prev) => prev.filter((e) => e.id !== entityId));

      try {
        await invoke("remove_meeting_entity", {
          meetingId: meeting.id,
          entityId,
          entityType,
        });
        emit("entity-updated");
      } catch (err) {
        // Revert on failure
        setLocalEntities(meeting.linkedEntities ?? []);
        console.error("Failed to remove meeting entity:", err);
      }
    },
    [meeting.id, meeting.linkedEntities]
  );

  const { outcomes, loading, refresh: refreshOutcomes } =
    useMeetingOutcomes(meeting.id);
  const outcomesStatus = loading ? "loading" as const : outcomes !== null ? "loaded" as const : "none" as const;

  // Auto-expand when outcomes arrive (e.g., from transcript-processed event)
  const prevOutcomes = React.useRef(outcomes);
  React.useEffect(() => {
    if (prevOutcomes.current === null && outcomes !== null) {
      setIsOpen(true);
    }
    prevOutcomes.current = outcomes;
  }, [outcomes]);

  const hasEnrichedPrep = !!(
    meeting.prep &&
    (meeting.prep.context || (meeting.prep.metrics && meeting.prep.metrics.length > 0))
  );

  const displayState = computeMeetingDisplayState(meeting, {
    isPast,
    outcomesStatus,
    isLive,
    hasInlinePrep: !!(meeting.prep && Object.keys(meeting.prep).length > 0),
    hasEnrichedPrep,
  });

  const handleAttachTranscript = React.useCallback(async () => {
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
    const toIso = (timeStr?: string): string => {
      if (!timeStr) return new Date().toISOString();
      const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
      if (!match) return new Date().toISOString();
      let h = parseInt(match[1], 10);
      const m = parseInt(match[2], 10);
      const period = match[3].toUpperCase();
      if (period === "PM" && h !== 12) h += 12;
      if (period === "AM" && h === 12) h = 0;
      const d = new Date();
      d.setHours(h, m, 0, 0);
      return d.toISOString();
    };

    const calendarEvent: CalendarEvent = {
      id: meeting.id,
      title: meeting.title,
      start: toIso(meeting.time),
      end: toIso(meeting.endTime),
      type: meeting.type,
      account: meeting.account,
      attendees: [],
      isAllDay: false,
    };

    try {
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
      await refreshOutcomes();
      setIsOpen(true);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("Failed to attach transcript:", msg);
      toast.error("Failed to attach transcript", { description: msg });
    } finally {
      setAttaching(false);
    }
  }, [meeting, refreshOutcomes]);

  const handleCaptureOutcomes = React.useCallback(() => {
    const toIso = (timeStr?: string): string => {
      if (!timeStr) return new Date().toISOString();
      const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
      if (!match) return new Date().toISOString();
      let h = parseInt(match[1], 10);
      const m = parseInt(match[2], 10);
      const period = match[3].toUpperCase();
      if (period === "PM" && h !== 12) h += 12;
      if (period === "AM" && h === 12) h = 0;
      const d = new Date();
      d.setHours(h, m, 0, 0);
      return d.toISOString();
    };

    const payload: CalendarEvent = {
      id: meeting.id,
      title: meeting.title,
      start: toIso(meeting.time),
      end: toIso(meeting.endTime),
      type: meeting.type,
      account: meeting.account,
      attendees: [],
      isAllDay: false,
    };
    emit("post-meeting-prompt", payload);
  }, [meeting]);

  return (
    <div className={displayState.card.className}>
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        <div className="p-5">
          {/* Row 1: Time + badges + actions */}
          <div className="flex items-center justify-between mb-1">
            <div className="flex items-center gap-2">
              <span className="font-mono text-xs text-muted-foreground/70">
                {meeting.time}
              </span>
              {meeting.endTime && (
                <>
                  <span className="text-muted-foreground/30">—</span>
                  <span className="font-mono text-xs text-muted-foreground/50">
                    {meeting.endTime}
                  </span>
                </>
              )}
              <Badge className={cn(badgeStyles[meeting.type], "text-[10px] px-1.5 py-0")} variant="secondary">
                {badgeLabels[meeting.type]}
              </Badge>
              {displayState.badges.map((badge) => (
                <Badge key={badge.key} variant={badge.variant} className={cn(badge.className, "text-[10px] px-1.5 py-0")}>
                  {badge.icon === "check" && <Check className="mr-0.5 size-2.5" />}
                  {badge.label}
                </Badge>
              ))}
            </div>

            <div className="flex items-center gap-1.5">
              {displayState.actions.map((action) => (
                <ActionButton
                  key={action.key}
                  action={action}
                  meeting={meeting}
                  attaching={attaching}
                  onAttach={handleAttachTranscript}
                  onCapture={handleCaptureOutcomes}
                />
              ))}

              {displayState.showExpander && (
                <CollapsibleTrigger asChild>
                  <button
                    className={cn(
                      "rounded-md p-1 transition-colors hover:bg-muted",
                      isOpen && "bg-muted"
                    )}
                  >
                    <ChevronDown
                      className={cn(
                        "size-3.5 text-muted-foreground transition-transform duration-200",
                        isOpen && "rotate-180"
                      )}
                    />
                  </button>
                </CollapsibleTrigger>
              )}
            </div>
          </div>

          {/* Row 2: Title */}
          <h3 className={cn("text-[15px] font-semibold leading-snug", displayState.title.lineThrough && "line-through")}>
            {meeting.title}
          </h3>

          {/* Row 3: Entity chips */}
          <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
            {localEntities.map((entity) => {
              const Icon = entity.entityType === "project" ? FolderKanban : Building2;
              return (
                <span
                  key={entity.id}
                  className="inline-flex items-center gap-1 rounded-md border bg-muted/50 px-2 py-0.5 text-xs"
                >
                  <Icon className="size-3 text-muted-foreground" />
                  {entity.name}
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemoveEntity(entity.id, entity.entityType);
                    }}
                    className="ml-0.5 text-muted-foreground hover:text-foreground"
                  >
                    <X className="size-3" />
                  </button>
                </span>
              );
            })}
            <EntityPicker
              value={null}
              onChange={handleAddEntity}
              entityType="account"
              placeholder="Link account..."
            />
          </div>

          {suggestedUnarchiveAccountId && (
            <div className="flex items-center gap-2 mt-1">
              <span className="text-xs text-primary/70">Matches archived account</span>
              <Button
                variant="ghost"
                size="sm"
                className="h-5 px-2 text-xs text-primary hover:text-primary"
                disabled={restoringSuggestion}
                onClick={async (e) => {
                  e.stopPropagation();
                  try {
                    setRestoringSuggestion(true);
                    setSuggestionError(null);
                    await invoke("archive_account", {
                      id: suggestedUnarchiveAccountId!,
                      archived: false,
                    });
                    await invoke("add_meeting_entity", {
                      meetingId: meeting.id,
                      entityId: suggestedUnarchiveAccountId!,
                      entityType: "account",
                      meetingTitle: meeting.title,
                      startTime: meeting.startIso ?? meeting.time,
                      meetingTypeStr: meeting.type,
                    });
                    setSuggestedUnarchiveAccountId(null);
                    emit("entity-updated");
                  } catch (err) {
                    const message = err instanceof Error ? err.message : String(err);
                    setSuggestionError(message);
                  } finally {
                    setRestoringSuggestion(false);
                  }
                }}
              >
                {restoringSuggestion ? "Restoring..." : "Restore"}
              </Button>
            </div>
          )}
          {suggestionError && (
            <p className="mt-1 text-xs text-destructive">{suggestionError}</p>
          )}

          {/* Row 4: Intelligence brief — surfaced on card, no expansion needed */}
          {meeting.prep?.context && (
            <p className="mt-2.5 text-[13px] leading-relaxed text-muted-foreground line-clamp-2">
              {meeting.prep.context}
            </p>
          )}

          {/* Row 5: Signal pills — top risk + top win visible at a glance */}
          {meeting.prep && (meeting.prep.risks?.length || meeting.prep.wins?.length) && (
            <div className="flex flex-wrap gap-1.5 mt-2">
              {meeting.prep.wins?.[0] && (
                <span className="inline-flex items-center gap-1 rounded-md bg-success/8 px-2 py-0.5 text-[11px] text-success">
                  <span className="size-1.5 rounded-full bg-success" />
                  <span className="line-clamp-1">{meeting.prep.wins[0]}</span>
                </span>
              )}
              {meeting.prep.risks?.[0] && (
                <span className="inline-flex items-center gap-1 rounded-md bg-destructive/8 px-2 py-0.5 text-[11px] text-destructive">
                  <span className="size-1.5 rounded-full bg-destructive" />
                  <span className="line-clamp-1">{meeting.prep.risks[0]}</span>
                </span>
              )}
            </div>
          )}
        </div>

        {displayState.showExpander && (
          <CollapsibleContent>
            <div className="border-t p-5 space-y-4">
              {/* Always show outcomes if they exist */}
              {outcomes !== null && (
                <MeetingOutcomes
                  outcomes={outcomes}
                  onRefresh={refreshOutcomes}
                />
              )}

              {/* Show prep: standalone if no outcomes, collapsible underneath if outcomes exist */}
              {meeting.prep && Object.keys(meeting.prep).length > 0 && (
                outcomes !== null ? (
                  <Collapsible defaultOpen={false}>
                    <CollapsibleTrigger className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground w-full">
                      <ChevronDown className="size-3" />
                      Pre-Meeting Context
                    </CollapsibleTrigger>
                    <CollapsibleContent className="mt-3">
                      <MeetingPrepContent prep={meeting.prep} />
                    </CollapsibleContent>
                  </Collapsible>
                ) : (
                  <MeetingPrepContent prep={meeting.prep} />
                )
              )}
            </div>
          </CollapsibleContent>
        )}
      </Collapsible>
    </div>
  );
}

function ActionButton({
  action,
  meeting,
  attaching,
  onAttach,
  onCapture,
}: {
  action: ActionState;
  meeting: Meeting;
  attaching: boolean;
  onAttach: () => void;
  onCapture: () => void;
}) {
  switch (action.key) {
    case "view-prep":
      return (
        <Button variant="ghost" size="sm" className="text-primary hover:text-primary" asChild>
          <Link to="/meeting/$meetingId" params={{ meetingId: action.linkTo! }}>
            {meeting.prepReviewed ? (
              <Check className="mr-1 size-3.5 text-success" />
            ) : (
              <FileText className="mr-1 size-3.5" />
            )}
            {action.label}
          </Link>
        </Button>
      );
    case "attach-transcript":
      return (
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-foreground"
          onClick={onAttach}
          disabled={attaching}
        >
          {attaching ? (
            <Loader2 className="mr-1 size-3.5 animate-spin" />
          ) : (
            <Paperclip className="mr-1 size-3.5" />
          )}
          {action.label}
        </Button>
      );
    case "capture-outcomes":
      return (
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-foreground"
          onClick={onCapture}
        >
          <Trophy className="mr-1 size-3.5" />
          {action.label}
        </Button>
      );
  }
}

function MeetingPrepContent({ prep }: { prep: NonNullable<Meeting["prep"]> }) {
  // Build unified sections that work for any meeting type
  // "At a Glance" - metrics for customer, key context for internal
  const atAGlance = prep.metrics?.slice(0, 4) ?? [];

  // "Discuss" - talking points, actions, or questions (whatever's available)
  const discuss = prep.actions ?? prep.questions ?? [];

  // "Watch" - risks or blockers
  const watch = prep.risks ?? [];

  const hasContent = prep.context || atAGlance.length > 0 || discuss.length > 0 || watch.length > 0;

  if (!hasContent) {
    return (
      <p className="text-sm text-muted-foreground italic">
        No prep summary available
      </p>
    );
  }

  return (
    <div className="space-y-4 text-sm">
      {/* Context - always first if available */}
      {prep.context && (
        <p className="text-muted-foreground">{prep.context}</p>
      )}

      {/* Universal grid with consistent sections */}
      <div className="grid gap-4 md:grid-cols-2">
        {/* At a Glance - key metrics or context points */}
        {atAGlance.length > 0 && (
          <PrepSection
            title="At a Glance"
            items={atAGlance}
            color="text-foreground"
          />
        )}

        {/* Discuss - talking points, actions, or questions */}
        {discuss.length > 0 && (
          <PrepSection
            title="Discuss"
            items={discuss.slice(0, 4)}
            color="text-primary"
          />
        )}

        {/* Watch - risks or blockers */}
        {watch.length > 0 && (
          <PrepSection
            title="Watch"
            items={watch.slice(0, 3)}
            color="text-destructive"
          />
        )}

        {/* Wins - only show if available (typically customer) */}
        {prep.wins && prep.wins.length > 0 && (
          <PrepSection
            title="Wins"
            items={prep.wins.slice(0, 3)}
            color="text-success"
          />
        )}
      </div>
    </div>
  );
}

function PrepSection({
  title,
  items,
  color,
}: {
  title: string;
  items: string[];
  color: string;
}) {
  return (
    <div className="space-y-1.5">
      <h4 className={cn("font-medium", color)}>{title}</h4>
      <ul className="space-y-1">
        {items.map((item, i) => (
          <li key={i} className="flex items-start gap-2 text-muted-foreground">
            <span className={cn("mt-1.5 size-1.5 shrink-0 rounded-full", color.replace("text-", "bg-"))} />
            {item}
          </li>
        ))}
      </ul>
    </div>
  );
}

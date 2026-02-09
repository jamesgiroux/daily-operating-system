import * as React from "react";
import { Link } from "@tanstack/react-router";
import { emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Building2,
  Check,
  ChevronDown,
  FileText,
  FolderKanban,
  Loader2,
  Paperclip,
  Trophy,
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
import type { Meeting, MeetingType, CalendarEvent } from "@/types";
import { cn } from "@/lib/utils";

interface MeetingCardProps {
  meeting: Meeting;
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
  const hasPrepFile = meeting.hasPrep && meeting.prepFile;

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

  // 2. Past + outcomes loaded → "processed"
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
    if (hasPrepFile) {
      actions.push({
        key: "view-prep",
        label: "View Prep",
        linkTo: meeting.prepFile!,
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
  else if (hasPrepFile) {
    primaryStatus = "has-prep";
    actions.push({
      key: "view-prep",
      label: "View Prep",
      linkTo: meeting.prepFile!,
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

/** Check if a meeting's end time has passed today. */
function isPastMeeting(meeting: Meeting): boolean {
  const timeStr = meeting.endTime || meeting.time;
  if (!timeStr) return false;

  const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
  if (!match) return false;

  let hours = parseInt(match[1], 10);
  const minutes = parseInt(match[2], 10);
  const period = match[3].toUpperCase();
  if (period === "PM" && hours !== 12) hours += 12;
  if (period === "AM" && hours === 12) hours = 0;

  const now = new Date();
  const end = new Date();
  end.setHours(hours, minutes, 0, 0);
  return now > end;
}

export function MeetingCard({ meeting }: MeetingCardProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [attaching, setAttaching] = React.useState(false);

  const { outcomes, loading, refresh: refreshOutcomes } =
    useMeetingOutcomes(meeting.id);
  const outcomesStatus = loading ? "loading" as const : outcomes !== null ? "loaded" as const : "none" as const;

  const hasEnrichedPrep = !!(
    meeting.prep &&
    (meeting.prep.context || (meeting.prep.metrics && meeting.prep.metrics.length > 0))
  );

  const displayState = computeMeetingDisplayState(meeting, {
    isPast: isPastMeeting(meeting),
    outcomesStatus,
    isLive: meeting.isCurrent ?? false,
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
      await invoke("attach_meeting_transcript", {
        filePath: selected,
        meeting: calendarEvent,
      });
      refreshOutcomes();
    } catch (err) {
      console.error("Failed to attach transcript:", err);
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
        <div className="flex items-start justify-between p-5">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-mono text-sm text-muted-foreground">
                {meeting.time}
              </span>
              {meeting.endTime && (
                <>
                  <span className="text-muted-foreground/50">—</span>
                  <span className="font-mono text-sm text-muted-foreground/70">
                    {meeting.endTime}
                  </span>
                </>
              )}
            </div>
            <h3 className={cn("font-semibold", displayState.title.lineThrough && "line-through")}>
              {meeting.title}
            </h3>
            {meeting.account && (
              <p className="text-sm text-primary">{meeting.account}</p>
            )}
            {meeting.linkedEntities && meeting.linkedEntities.length > 0 && (
              <div className="flex items-center gap-1.5 flex-wrap">
                {meeting.linkedEntities.map((entity) => (
                  <Link
                    key={entity.id}
                    to={entity.entityType === "project" ? "/projects/$projectId" : "/accounts/$accountId"}
                    params={entity.entityType === "project" ? { projectId: entity.id } : { accountId: entity.id }}
                    className="inline-flex items-center gap-1 rounded-md bg-muted px-1.5 py-0.5 text-xs text-muted-foreground hover:bg-muted/80 transition-colors"
                  >
                    {entity.entityType === "project" ? (
                      <FolderKanban className="size-3" />
                    ) : (
                      <Building2 className="size-3" />
                    )}
                    {entity.name}
                  </Link>
                ))}
              </div>
            )}
          </div>

          <div className="flex items-center gap-2">
            {displayState.badges.map((badge) => (
              <Badge key={badge.key} variant={badge.variant} className={badge.className}>
                {badge.icon === "check" && <Check className="mr-1 size-3" />}
                {badge.label}
              </Badge>
            ))}

            <Badge className={badgeStyles[meeting.type]} variant="secondary">
              {badgeLabels[meeting.type]}
            </Badge>

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
                    "rounded-md p-1.5 transition-colors hover:bg-muted",
                    isOpen && "bg-muted"
                  )}
                >
                  <ChevronDown
                    className={cn(
                      "size-4 text-muted-foreground transition-transform duration-200",
                      isOpen && "rotate-180"
                    )}
                  />
                </button>
              </CollapsibleTrigger>
            )}
          </div>
        </div>

        {displayState.showExpander && (
          <CollapsibleContent>
            <div className="border-t bg-muted/30 p-5">
              {outcomes !== null && (
                <MeetingOutcomes
                  outcomes={outcomes}
                  onRefresh={refreshOutcomes}
                />
              )}
              {outcomes === null && meeting.prep && Object.keys(meeting.prep).length > 0 && (
                <MeetingPrepContent prep={meeting.prep} />
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
          <Link to="/meeting/$prepFile" params={{ prepFile: action.linkTo! }}>
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

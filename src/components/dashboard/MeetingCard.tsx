import * as React from "react";
import { Link } from "@tanstack/react-router";
import { emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Check,
  ChevronDown,
  FileText,
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
  const hasInlinePrep = meeting.prep && Object.keys(meeting.prep).length > 0;
  const hasPrepFile = meeting.hasPrep && meeting.prepFile;
  const isPast = isPastMeeting(meeting);
  const isCancelled = meeting.overlayStatus === "cancelled";
  const isNew = meeting.overlayStatus === "new";

  // Load outcomes for past meetings
  const { outcomes, refresh: refreshOutcomes } =
    useMeetingOutcomes(meeting.id);
  const hasOutcomes = outcomes !== null;

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
    // Build ISO timestamps from display times
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
    // Build ISO timestamps from the meeting's display times (e.g., "09:00 AM").
    // These are used by the capture backend; fallback to current time if unparseable.
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
    <div
      className={cn(
        "rounded-lg border bg-card shadow-sm transition-all duration-150",
        !isCancelled && "hover:-translate-y-0.5 hover:shadow-md",
        borderStyles[meeting.type],
        meeting.isCurrent && !isCancelled && "animate-pulse-gold ring-2 ring-primary/50",
        isCancelled && "opacity-50"
      )}
    >
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
            <h3 className={cn("font-semibold", isCancelled && "line-through")}>{meeting.title}</h3>
            {meeting.account && (
              <p className="text-sm text-primary">{meeting.account}</p>
            )}
          </div>

          <div className="flex items-center gap-2">
            {isCancelled && (
              <Badge variant="outline" className="text-destructive border-destructive/30">
                Cancelled
              </Badge>
            )}

            {isNew && (
              <Badge variant="outline" className="text-muted-foreground">
                No prep available
              </Badge>
            )}

            <Badge className={badgeStyles[meeting.type]} variant="secondary">
              {badgeLabels[meeting.type]}
            </Badge>

            {/* View Prep button for meetings with prep files (hidden for cancelled) */}
            {hasPrepFile && !isCancelled && (
              <Button
                variant="ghost"
                size="sm"
                className="text-primary hover:text-primary"
                asChild
              >
                <Link
                  to="/meeting/$prepFile"
                  params={{ prepFile: meeting.prepFile! }}
                >
                  {meeting.prepReviewed ? (
                    <Check className="mr-1 size-3.5 text-success" />
                  ) : (
                    <FileText className="mr-1 size-3.5" />
                  )}
                  View Prep
                </Link>
              </Button>
            )}

            {/* No prep badge for customer meetings without prep */}
            {!meeting.hasPrep && !isNew && meeting.type === "customer" && (
              <Badge variant="outline" className="text-muted-foreground">
                No prep
              </Badge>
            )}

            {/* Past meeting actions: outcomes display or attach/capture (hidden for cancelled) */}
            {isPast && !isCancelled && (
              <>
                {hasOutcomes ? (
                  <Badge variant="outline" className="text-success border-success/30">
                    <Check className="mr-1 size-3" />
                    Processed
                  </Badge>
                ) : (
                  <>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-muted-foreground hover:text-foreground"
                      onClick={handleAttachTranscript}
                      disabled={attaching}
                    >
                      {attaching ? (
                        <Loader2 className="mr-1 size-3.5 animate-spin" />
                      ) : (
                        <Paperclip className="mr-1 size-3.5" />
                      )}
                      Attach
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-muted-foreground hover:text-foreground"
                      onClick={handleCaptureOutcomes}
                    >
                      <Trophy className="mr-1 size-3.5" />
                      Outcomes
                    </Button>
                  </>
                )}
              </>
            )}

            {/* Expand button for inline prep or outcomes */}
            {(hasInlinePrep || hasOutcomes) && (
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

        {(hasInlinePrep || hasOutcomes) && (
          <CollapsibleContent>
            <div className="border-t bg-muted/30 p-5">
              {hasOutcomes && (
                <MeetingOutcomes
                  outcomes={outcomes}
                  onRefresh={refreshOutcomes}
                />
              )}
              {hasInlinePrep && !hasOutcomes && (
                <MeetingPrepContent prep={meeting.prep!} />
              )}
            </div>
          </CollapsibleContent>
        )}
      </Collapsible>
    </div>
  );
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

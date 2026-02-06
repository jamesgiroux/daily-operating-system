import * as React from "react";
import { Link } from "@tanstack/react-router";
import { emit } from "@tauri-apps/api/event";
import { ChevronDown, FileText, Trophy } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
  const hasInlinePrep = meeting.prep && Object.keys(meeting.prep).length > 0;
  const hasPrepFile = meeting.hasPrep && meeting.prepFile;
  const isPast = isPastMeeting(meeting);

  const handleCaptureOutcomes = React.useCallback(() => {
    const payload: CalendarEvent = {
      id: meeting.id,
      title: meeting.title,
      start: "",
      end: "",
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
        "hover:-translate-y-0.5 hover:shadow-md",
        borderStyles[meeting.type],
        meeting.isCurrent && "animate-pulse-gold ring-2 ring-primary/50"
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
            <h3 className="font-semibold">{meeting.title}</h3>
            {meeting.account && (
              <p className="text-sm text-primary">{meeting.account}</p>
            )}
          </div>

          <div className="flex items-center gap-2">
            <Badge className={badgeStyles[meeting.type]} variant="secondary">
              {badgeLabels[meeting.type]}
            </Badge>

            {/* View Prep button for meetings with prep files */}
            {hasPrepFile && (
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
                  <FileText className="mr-1 size-3.5" />
                  View Prep
                </Link>
              </Button>
            )}

            {/* No prep badge for customer meetings without prep */}
            {!meeting.hasPrep && meeting.type === "customer" && (
              <Badge variant="outline" className="text-muted-foreground">
                No prep
              </Badge>
            )}

            {/* Outcomes button for past meetings */}
            {isPast && (
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground hover:text-foreground"
                onClick={handleCaptureOutcomes}
              >
                <Trophy className="mr-1 size-3.5" />
                Outcomes
              </Button>
            )}

            {/* Expand button for inline prep */}
            {hasInlinePrep && (
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

        {hasInlinePrep && (
          <CollapsibleContent>
            <div className="border-t bg-muted/30 p-5">
              <MeetingPrepContent prep={meeting.prep!} />
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

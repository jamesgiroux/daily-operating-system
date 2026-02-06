import { CalendarDays } from "lucide-react";
import { MeetingCard } from "./MeetingCard";
import type { Meeting, MeetingType } from "@/types";
import { useCalendar } from "@/hooks/useCalendar";
import { cn } from "@/lib/utils";

interface MeetingTimelineProps {
  meetings: Meeting[];
}

const dotColors: Partial<Record<MeetingType, string>> = {
  customer: "bg-primary",
  qbr: "bg-primary",
  partnership: "bg-primary",
  external: "bg-primary",
  personal: "bg-success",
};

export function MeetingTimeline({ meetings }: MeetingTimelineProps) {
  const { currentMeeting } = useCalendar();

  if (meetings.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed p-8 text-center">
        <CalendarDays className="mb-2 size-8 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">No meetings today</p>
      </div>
    );
  }

  // Determine which briefing meeting is "current" by checking live calendar
  function isLive(meeting: Meeting): boolean {
    if (!currentMeeting) return false;
    // Match by title + approximate time
    return (
      meeting.title === currentMeeting.title ||
      meeting.id === currentMeeting.id
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-lg font-semibold">
          <CalendarDays className="size-5" />
          Schedule
        </h2>
        <div className="flex items-center gap-3">
          {currentMeeting && (
            <span className="flex items-center gap-1.5 text-xs text-primary">
              <span className="relative flex size-2">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-primary opacity-75" />
                <span className="relative inline-flex size-2 rounded-full bg-primary" />
              </span>
              In meeting
            </span>
          )}
          <span className="text-sm text-muted-foreground">
            {meetings.length} meeting{meetings.length !== 1 ? "s" : ""}
          </span>
        </div>
      </div>

      <div className="relative">
        {/* Timeline line */}
        <div className="absolute left-[7px] top-6 bottom-6 w-px bg-border" />

        <div className="space-y-6">
          {meetings.map((meeting, index) => {
            const live = isLive(meeting);
            return (
              <div
                key={meeting.id}
                className={cn(
                  "relative flex gap-4 pl-6",
                  "animate-fade-in-up opacity-0",
                  index === 0 && "animate-delay-1",
                  index === 1 && "animate-delay-2",
                  index === 2 && "animate-delay-3",
                  index >= 3 && "animate-delay-4"
                )}
                style={{
                  animationDelay: index >= 3 ? `${0.1 + index * 0.05}s` : undefined,
                }}
              >
                {/* Timeline dot */}
                <div
                  className={cn(
                    "absolute left-0 top-5 size-3.5 rounded-full border-2 border-background",
                    dotColors[meeting.type] ?? "bg-muted-foreground/50",
                    (live || meeting.isCurrent) && "ring-2 ring-primary/50",
                    live && "animate-pulse"
                  )}
                />

                <div className="flex-1">
                  <MeetingCard meeting={meeting} />
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

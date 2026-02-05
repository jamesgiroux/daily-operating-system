import { CalendarDays } from "lucide-react";
import { MeetingCard } from "./MeetingCard";
import type { Meeting, MeetingType } from "@/types";
import { cn } from "@/lib/utils";

interface MeetingTimelineProps {
  meetings: Meeting[];
}

const dotColors: Record<MeetingType, string> = {
  customer: "bg-primary",
  internal: "bg-muted-foreground/50",
  personal: "bg-success",
};

export function MeetingTimeline({ meetings }: MeetingTimelineProps) {
  if (meetings.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed p-8 text-center">
        <CalendarDays className="mb-2 size-8 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">No meetings today</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-lg font-semibold">
          <CalendarDays className="size-5" />
          Schedule
        </h2>
        <span className="text-sm text-muted-foreground">
          {meetings.length} meeting{meetings.length !== 1 ? "s" : ""}
        </span>
      </div>

      <div className="relative">
        {/* Timeline line */}
        <div className="absolute left-[7px] top-6 bottom-6 w-px bg-border" />

        <div className="space-y-6">
          {meetings.map((meeting, index) => (
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
                  dotColors[meeting.type],
                  meeting.isCurrent && "ring-2 ring-primary/50"
                )}
              />

              <div className="flex-1">
                <MeetingCard meeting={meeting} />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

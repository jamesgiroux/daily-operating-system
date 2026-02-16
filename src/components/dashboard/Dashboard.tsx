import { Target } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { MeetingTimeline } from "./MeetingTimeline";
import { ActionList } from "./ActionList";
import { EmailList } from "./EmailList";
import { formatDayTime } from "@/lib/utils";
import type { DashboardData, DataFreshness } from "@/types";

interface DashboardProps {
  data: DashboardData;
  freshness: DataFreshness;
}

function getFormattedDate(): string {
  return new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  });
}

export function Dashboard({ data, freshness }: DashboardProps) {
  const emails = data.emails ?? [];
  const emailSync = data.emailSync;
  const formattedDate = getFormattedDate();

  return (
    <ScrollArea className="flex-1">
      <div className="px-8 pt-10 pb-8">
        <div className="mx-auto max-w-6xl">
          <div className="space-y-8">
            <div className="space-y-1">
              <h1 className="text-2xl font-semibold tracking-tight">
                {formattedDate}
              </h1>
              {freshness.freshness === "stale" && (
                <p className="text-xs text-muted-foreground">
                  Last updated {formatDayTime(freshness.generatedAt)}
                </p>
              )}
            </div>

            {data.overview.focus && (
              <div className="block rounded-lg bg-success/10 border border-success/15 px-4 py-3.5">
                <div className="flex items-center gap-2 mb-2">
                  <Target className="size-5 shrink-0 text-success" />
                  <span className="text-sm font-semibold text-success">Focus</span>
                </div>
                <p className="text-sm font-medium text-foreground leading-relaxed">{data.overview.focus}</p>
              </div>
            )}

            <MeetingTimeline meetings={data.meetings} />

            <div className="animate-fade-in-up opacity-0 animate-delay-3">
              <ActionList actions={data.actions} />
            </div>

            <div className="animate-fade-in-up opacity-0 animate-delay-4">
              <EmailList emails={emails} emailSync={emailSync} />
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}

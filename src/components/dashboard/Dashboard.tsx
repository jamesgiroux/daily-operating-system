import { Target, ChevronRight } from "lucide-react";
import { Link } from "@tanstack/react-router";
import { ScrollArea } from "@/components/ui/scroll-area";
import { IntelligenceCard } from "./IntelligenceCard";
import { MeetingTimeline } from "./MeetingTimeline";
import { ActionList } from "./ActionList";
import { EmailList } from "./EmailList";
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

function formatRelativeDate(isoString: string): string {
  try {
    const date = new Date(isoString);
    if (isNaN(date.getTime())) return "";
    return date.toLocaleDateString("en-US", {
      weekday: "long",
    }) + " at " + date.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

export function Dashboard({ data, freshness }: DashboardProps) {
  const emails = data.emails ?? [];
  const formattedDate = getFormattedDate();

  return (
    <ScrollArea className="flex-1">
      <div className="p-8">
        <div className="mx-auto max-w-6xl">
          <div className="grid gap-8 lg:grid-cols-[5fr_2fr]">
            {/* Left: Schedule */}
            <div className="min-w-0 space-y-6">
              <div className="space-y-1">
                <h1 className="text-2xl font-semibold tracking-tight">
                  {formattedDate}
                </h1>
                {freshness.freshness === "stale" && (
                  <p className="text-xs text-muted-foreground">
                    Last updated {formatRelativeDate(freshness.generatedAt)}
                  </p>
                )}
              </div>
              <MeetingTimeline meetings={data.meetings} />
            </div>

            {/* Right: Context sidebar */}
            <div className="min-w-0 space-y-5">
              {data.overview.focus && (
                <Link to="/focus" className="block rounded-lg bg-primary/5 border border-primary/10 px-3.5 py-3 transition-colors hover:bg-primary/10">
                  <div className="flex items-start gap-2.5">
                    <Target className="size-4 shrink-0 text-primary mt-0.5" />
                    <div className="min-w-0 flex-1">
                      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Focus</span>
                      <p className="mt-1 text-sm font-medium text-primary leading-relaxed">{data.overview.focus}</p>
                    </div>
                    <ChevronRight className="size-4 shrink-0 text-muted-foreground mt-0.5" />
                  </div>
                </Link>
              )}
              <IntelligenceCard />
              <div className="animate-fade-in-up opacity-0 animate-delay-3">
                <ActionList actions={data.actions} />
              </div>
              <div className="animate-fade-in-up opacity-0 animate-delay-4">
                <EmailList emails={emails} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}

import { Target, ChevronRight } from "lucide-react";
import { Link } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ReadinessStrip } from "./ReadinessStrip";
import type { DashboardData, DataFreshness, Meeting, Action } from "@/types";

interface OverviewProps {
  overview: DashboardData["overview"];
  meetings: Meeting[];
  actions: Action[];
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

export function Overview({ overview, meetings, actions, freshness }: OverviewProps) {
  const formattedDate = getFormattedDate();

  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <div className="space-y-3">
          <h1 className="text-3xl font-semibold tracking-tight">
            {formattedDate}
          </h1>
          {freshness.freshness === "stale" && (
            <p className="text-xs text-muted-foreground">
              Last updated {formatRelativeDate(freshness.generatedAt)}
            </p>
          )}
          {overview.focus && (
            <div className="flex items-center gap-2.5 rounded-lg bg-primary/5 border border-primary/10 px-3 py-2">
              <Target className="size-4 shrink-0 text-primary" />
              <div className="min-w-0">
                <span className="text-xs font-medium text-muted-foreground">Focus</span>
                <p className="text-sm font-medium text-primary truncate">{overview.focus}</p>
              </div>
              <Link to="/focus" className="ml-auto">
                <ChevronRight className="size-4 text-muted-foreground hover:text-foreground transition-colors" />
              </Link>
            </div>
          )}
        </div>

        {overview.summary && (
          <Card className="animate-fade-in-up opacity-0 animate-delay-2">
            <CardHeader className="pb-2">
              <CardTitle className="text-base font-medium text-muted-foreground">
                Today
              </CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm leading-relaxed text-muted-foreground">
                {overview.summary}
              </p>
            </CardContent>
          </Card>
        )}
      </div>
      <ReadinessStrip meetings={meetings} actions={actions} />
    </div>
  );
}

import { Target, ChevronRight } from "lucide-react";
import { Link } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { StatsRow } from "./StatsRow";
import type { DashboardData, DataFreshness } from "@/types";

interface OverviewProps {
  overview: DashboardData["overview"];
  stats: DashboardData["stats"];
  freshness: DataFreshness;
}

function getTimeBasedGreeting(): string {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 17) return "Good afternoon";
  return "Good evening";
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

export function Overview({ overview, stats, freshness }: OverviewProps) {
  const greeting = getTimeBasedGreeting();
  const formattedDate = getFormattedDate();

  return (
    <div className="space-y-6">
      <div className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-3">
          <h1 className="text-3xl font-semibold tracking-tight">
            {formattedDate}
          </h1>
          {freshness.freshness === "stale" && (
            <p className="text-xs text-muted-foreground">
              Last updated {formatRelativeDate(freshness.generatedAt)}
            </p>
          )}
          <p className="text-lg font-light text-muted-foreground">{greeting}</p>
          {overview.focus && (
            <Link
              to="/focus"
              className="group block space-y-1 text-sm rounded-md -mx-2 px-2 py-1.5 transition-colors hover:bg-muted/50"
            >
              <div className="flex items-center gap-2 text-muted-foreground">
                <Target className="size-4 text-primary" />
                <span className="font-medium">Focus</span>
                <ChevronRight className="size-3.5 opacity-0 -translate-x-1 transition-all group-hover:opacity-100 group-hover:translate-x-0" />
              </div>
              <p className="text-primary">{overview.focus}</p>
            </Link>
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
      <StatsRow stats={stats} />
    </div>
  );
}

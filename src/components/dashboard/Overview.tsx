import { Target } from "lucide-react";
import { StatsRow } from "./StatsRow";
import type { DashboardData } from "@/types";

interface OverviewProps {
  overview: DashboardData["overview"];
  stats: DashboardData["stats"];
}

/**
 * Returns a time-appropriate greeting based on the current hour
 */
function getTimeBasedGreeting(): string {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 17) return "Good afternoon";
  return "Good evening";
}

export function Overview({ overview, stats }: OverviewProps) {
  const greeting = getTimeBasedGreeting();

  return (
    <div className="space-y-6">
      <div className="space-y-3">
        <div className="flex items-baseline gap-3">
          <h1 className="text-3xl font-light italic">{overview.date}</h1>
        </div>
        <p className="text-lg font-light text-muted-foreground">
          {greeting} — {overview.summary}
        </p>
        {overview.focus && (
          <div className="flex items-center gap-2 text-sm">
            <Target className="size-4 text-primary" />
            <span className="font-medium">Focus:</span>
            <span className="text-primary">{overview.focus}</span>
          </div>
        )}
      </div>
      <StatsRow stats={stats} />
    </div>
  );
}

import { Target } from "lucide-react";
import { StatsRow } from "./StatsRow";
import type { DashboardData } from "@/types";

interface OverviewProps {
  overview: DashboardData["overview"];
  stats: DashboardData["stats"];
}

export function Overview({ overview, stats }: OverviewProps) {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <div className="flex items-baseline gap-2">
          <h2 className="text-2xl font-bold">{overview.greeting}</h2>
          <span className="text-muted-foreground">—</span>
          <span className="text-muted-foreground">{overview.date}</span>
        </div>
        <p className="text-muted-foreground">{overview.summary}</p>
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

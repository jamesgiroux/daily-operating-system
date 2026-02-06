import { Target } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { StatsRow } from "./StatsRow";
import type { DashboardData } from "@/types";

interface OverviewProps {
  overview: DashboardData["overview"];
  stats: DashboardData["stats"];
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

export function Overview({ overview, stats }: OverviewProps) {
  const greeting = getTimeBasedGreeting();
  const formattedDate = getFormattedDate();

  return (
    <div className="space-y-6">
      <div className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-3">
          <h1 className="text-3xl font-semibold tracking-tight">
            {formattedDate}
          </h1>
          <p className="text-lg font-light text-muted-foreground">{greeting}</p>
          {overview.focus && (
            <div className="space-y-1 text-sm">
              <div className="flex items-center gap-2 text-muted-foreground">
                <Target className="size-4 text-primary" />
                <span className="font-medium">Focus</span>
              </div>
              <p className="text-primary">{overview.focus}</p>
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
      <StatsRow stats={stats} />
    </div>
  );
}

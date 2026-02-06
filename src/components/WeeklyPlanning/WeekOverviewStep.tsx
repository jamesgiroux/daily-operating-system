import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { AlertTriangle } from "lucide-react";
import type { WeekOverview } from "@/types";

interface WeekOverviewStepProps {
  weekData: WeekOverview;
  onContinue: () => void;
  onSkip: () => void;
}

export function WeekOverviewStep({
  weekData,
  onContinue,
  onSkip,
}: WeekOverviewStepProps) {
  const totalMeetings = weekData.days.reduce(
    (sum, d) => sum + d.meetings.length,
    0
  );

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Your week at a glance</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          {totalMeetings} meetings, {weekData.actionSummary?.dueThisWeek ?? 0}{" "}
          actions due
        </p>
      </div>

      {/* 5-day grid */}
      <div className="grid grid-cols-5 gap-3">
        {weekData.days.map((day) => (
          <div key={day.date} className="rounded-lg border p-3 text-center">
            <p className="text-xs font-medium text-muted-foreground">
              {day.dayName}
            </p>
            <p className="mt-1 text-2xl font-semibold">
              {day.meetings.length}
            </p>
            <p className="text-xs text-muted-foreground">
              meeting{day.meetings.length !== 1 ? "s" : ""}
            </p>
          </div>
        ))}
      </div>

      {/* Alerts row */}
      <div className="flex flex-wrap gap-3">
        {weekData.actionSummary &&
          weekData.actionSummary.overdueCount > 0 && (
            <Badge variant="destructive" className="gap-1">
              <AlertTriangle className="size-3" />
              {weekData.actionSummary.overdueCount} overdue actions
            </Badge>
          )}
        {weekData.hygieneAlerts && weekData.hygieneAlerts.length > 0 && (
          <Badge
            variant="outline"
            className="gap-1 border-peach text-peach"
          >
            <AlertTriangle className="size-3" />
            {weekData.hygieneAlerts.length} hygiene alert
            {weekData.hygieneAlerts.length !== 1 ? "s" : ""}
          </Badge>
        )}
      </div>

      <div className="flex items-center gap-3">
        <Button onClick={onContinue}>Got it</Button>
        <Button variant="ghost" onClick={onSkip}>
          Skip
        </Button>
      </div>
    </div>
  );
}

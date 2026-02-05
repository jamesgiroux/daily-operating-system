import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { WeekOverview, WeekDay, WeekMeeting, PrepStatus, AlertSeverity } from "@/types";
import { cn } from "@/lib/utils";
import {
  AlertCircle,
  Calendar,
  CheckCircle,
  Clock,
  FileText,
  Users,
  AlertTriangle,
} from "lucide-react";

interface WeekResult {
  status: "success" | "not_found" | "error";
  data?: WeekOverview;
  message?: string;
}

const prepStatusConfig: Record<PrepStatus, { label: string; icon: typeof CheckCircle; color: string }> = {
  prep_needed: { label: "Prep needed", icon: FileText, color: "text-destructive" },
  agenda_needed: { label: "Agenda needed", icon: Calendar, color: "text-primary" },
  bring_updates: { label: "Bring updates", icon: Clock, color: "text-primary" },
  context_needed: { label: "Context needed", icon: Users, color: "text-muted-foreground" },
  prep_ready: { label: "Prep ready", icon: CheckCircle, color: "text-success" },
  draft_ready: { label: "Draft ready", icon: FileText, color: "text-success" },
  done: { label: "Done", icon: CheckCircle, color: "text-success" },
};

const severityStyles: Record<AlertSeverity, string> = {
  critical: "border-l-destructive bg-destructive/5",
  warning: "border-l-primary bg-primary/5",
  info: "border-l-muted-foreground",
};

export default function WeekPage() {
  const [data, setData] = useState<WeekOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadWeek() {
      try {
        const result = await invoke<WeekResult>("get_week_data");
        if (result.status === "success" && result.data) {
          setData(result.data);
        } else if (result.status === "not_found") {
          setError(result.message || "No week overview found");
        } else if (result.status === "error") {
          setError(result.message || "Failed to load week data");
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }
    loadWeek();
  }, []);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-64" />
          <Skeleton className="h-4 w-48" />
        </div>
        <div className="grid grid-cols-5 gap-4">
          {[1, 2, 3, 4, 5].map((i) => (
            <Skeleton key={i} className="h-96" />
          ))}
        </div>
      </main>
    );
  }

  if (error || !data) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="size-5" />
              <p>{error || "No week data available. Run /week to generate."}</p>
            </div>
          </CardContent>
        </Card>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          {/* Header */}
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">
              Week {data.weekNumber}
            </h1>
            <p className="text-sm text-muted-foreground">{data.dateRange}</p>
          </div>

          {/* Week calendar grid */}
          <div className="mb-8 grid grid-cols-5 gap-3">
            {data.days.map((day) => (
              <DayColumn key={day.dayName} day={day} />
            ))}
          </div>

          {/* Side panels */}
          <div className="grid gap-6 lg:grid-cols-2">
            {/* Action summary */}
            {data.actionSummary && (
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <AlertTriangle className="size-4" />
                    Action Summary
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Overdue</span>
                    <Badge variant="destructive">{data.actionSummary.overdueCount}</Badge>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Due this week</span>
                    <Badge variant="secondary">{data.actionSummary.dueThisWeek}</Badge>
                  </div>
                  {data.actionSummary.criticalItems.length > 0 && (
                    <div className="pt-2">
                      <p className="mb-2 text-sm font-medium text-destructive">
                        Critical Items:
                      </p>
                      <ul className="space-y-1">
                        {data.actionSummary.criticalItems.map((item, i) => (
                          <li key={i} className="text-sm text-muted-foreground">
                            • {item}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </CardContent>
              </Card>
            )}

            {/* Hygiene alerts */}
            {data.hygieneAlerts && data.hygieneAlerts.length > 0 && (
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <AlertCircle className="size-4 text-destructive" />
                    Hygiene Alerts
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  {data.hygieneAlerts.map((alert, i) => (
                    <div
                      key={i}
                      className={cn(
                        "rounded-md border-l-4 p-3",
                        severityStyles[alert.severity]
                      )}
                    >
                      <p className="font-medium">{alert.account}</p>
                      {alert.ring && (
                        <p className="text-xs text-muted-foreground">
                          Ring: {alert.ring}
                          {alert.arr && ` • ARR: ${alert.arr}`}
                        </p>
                      )}
                      <p className="mt-1 text-sm text-muted-foreground">
                        {alert.issue}
                      </p>
                    </div>
                  ))}
                </CardContent>
              </Card>
            )}
          </div>

          {/* Focus areas */}
          {data.focusAreas && data.focusAreas.length > 0 && (
            <Card className="mt-6">
              <CardHeader>
                <CardTitle className="text-base">Weekly Priorities</CardTitle>
              </CardHeader>
              <CardContent>
                <ol className="list-decimal list-inside space-y-2">
                  {data.focusAreas.map((area, i) => (
                    <li key={i} className="text-sm">
                      {area}
                    </li>
                  ))}
                </ol>
              </CardContent>
            </Card>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

function DayColumn({ day }: { day: WeekDay }) {
  const today = new Date().toLocaleDateString("en-US", { weekday: "short" });
  const isToday = day.dayName.toLowerCase().startsWith(today.toLowerCase());

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3",
        isToday && "ring-2 ring-primary"
      )}
    >
      <h3
        className={cn(
          "mb-3 text-sm font-semibold",
          isToday && "text-primary"
        )}
      >
        {day.dayName}
      </h3>
      <div className="space-y-2">
        {day.meetings.length === 0 ? (
          <p className="py-4 text-center text-xs text-muted-foreground">
            No meetings
          </p>
        ) : (
          day.meetings.map((meeting, i) => (
            <WeekMeetingCard key={i} meeting={meeting} />
          ))
        )}
      </div>
    </div>
  );
}

function WeekMeetingCard({ meeting }: { meeting: WeekMeeting }) {
  const config = prepStatusConfig[meeting.prepStatus];
  const Icon = config.icon;

  return (
    <div
      className={cn(
        "rounded-md border p-2 text-xs",
        meeting.type === "customer" && "border-l-2 border-l-primary"
      )}
    >
      <div className="mb-1 font-mono text-muted-foreground">{meeting.time}</div>
      <div className="line-clamp-2 font-medium">{meeting.title}</div>
      <div className={cn("mt-1 flex items-center gap-1", config.color)}>
        <Icon className="size-3" />
        <span>{config.label}</span>
      </div>
    </div>
  );
}

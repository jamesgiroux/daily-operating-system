import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { FocusData, FocusPriority, TimeBlock } from "@/types";
import { cn } from "@/lib/utils";
import {
  AlertCircle,
  Target,
  Clock,
  Zap,
  Sun,
  Moon,
  CheckCircle2,
} from "lucide-react";

interface FocusResult {
  status: "success" | "not_found" | "error";
  data?: FocusData;
  message?: string;
}

const priorityColors: Record<string, string> = {
  "priority 1": "border-l-destructive",
  "priority 2": "border-l-primary",
  "priority 3": "border-l-muted-foreground",
  "priority 4": "border-l-muted",
};

export default function FocusPage() {
  const [data, setData] = useState<FocusData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadFocus() {
      try {
        const result = await invoke<FocusResult>("get_focus_data");
        if (result.status === "success" && result.data) {
          setData(result.data);
        } else if (result.status === "not_found") {
          setError(result.message || "No focus data found");
        } else if (result.status === "error") {
          setError(result.message || "Failed to load focus data");
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }
    loadFocus();
  }, []);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-32" />
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
              <p>{error || "No focus data available."}</p>
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
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">Focus</h1>
            <p className="text-sm text-muted-foreground">
              Suggested priorities and time blocks for today
            </p>
          </div>

          <div className="grid gap-6 lg:grid-cols-3">
            {/* Priorities column */}
            <div className="lg:col-span-2 space-y-4">
              {data.priorities.map((priority, i) => (
                <PrioritySection key={i} priority={priority} />
              ))}

              {data.priorities.length === 0 && (
                <Card>
                  <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                    <CheckCircle2 className="mb-4 size-12 text-success" />
                    <p className="text-lg font-medium">All clear!</p>
                    <p className="text-sm text-muted-foreground">
                      No specific focus areas suggested today.
                    </p>
                  </CardContent>
                </Card>
              )}
            </div>

            {/* Sidebar */}
            <div className="space-y-6">
              {/* Time blocks */}
              {data.timeBlocks && data.timeBlocks.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <Clock className="size-4" />
                      Available Time
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {data.timeBlocks.map((block, i) => (
                      <TimeBlockItem key={i} block={block} />
                    ))}
                  </CardContent>
                </Card>
              )}

              {/* Energy notes */}
              {data.energyNotes && (data.energyNotes.morning || data.energyNotes.afternoon) && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <Zap className="size-4" />
                      Energy Tips
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {data.energyNotes.morning && (
                      <div className="flex items-start gap-2">
                        <Sun className="mt-0.5 size-4 text-primary" />
                        <div>
                          <p className="text-sm font-medium">Morning</p>
                          <p className="text-sm text-muted-foreground">
                            {data.energyNotes.morning}
                          </p>
                        </div>
                      </div>
                    )}
                    {data.energyNotes.afternoon && (
                      <div className="flex items-start gap-2">
                        <Moon className="mt-0.5 size-4 text-muted-foreground" />
                        <div>
                          <p className="text-sm font-medium">Afternoon</p>
                          <p className="text-sm text-muted-foreground">
                            {data.energyNotes.afternoon}
                          </p>
                        </div>
                      </div>
                    )}
                  </CardContent>
                </Card>
              )}

              {/* Quick wins */}
              {data.quickWins && data.quickWins.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <Zap className="size-4 text-success" />
                      Quick Wins
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {data.quickWins.map((win, i) => (
                        <li key={i} className="flex items-start gap-2 text-sm">
                          <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-success" />
                          {win}
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function PrioritySection({ priority }: { priority: FocusPriority }) {
  const borderColor = priorityColors[priority.level.toLowerCase()] || "border-l-muted";

  return (
    <Card className={cn("border-l-4", borderColor)}>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-base">
          <Target className="size-4" />
          {priority.level}
          {priority.label && (
            <span className="font-normal text-muted-foreground">
              : {priority.label}
            </span>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {priority.items.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No items in this priority level.
          </p>
        ) : (
          <ul className="space-y-2">
            {priority.items.map((item, i) => (
              <li key={i} className="flex items-start gap-2 text-sm">
                <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-muted-foreground" />
                {item}
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}

function TimeBlockItem({ block }: { block: TimeBlock }) {
  return (
    <div className="flex items-center justify-between rounded-md bg-muted/50 p-2">
      <div>
        <p className="font-mono text-sm">
          {block.start} - {block.end}
        </p>
        {block.suggestedUse && (
          <p className="text-xs text-muted-foreground">{block.suggestedUse}</p>
        )}
      </div>
      <span className="text-xs text-muted-foreground">
        {block.durationMinutes} min
      </span>
    </div>
  );
}

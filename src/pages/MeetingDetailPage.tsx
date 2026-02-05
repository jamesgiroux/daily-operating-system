import { useState, useEffect } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { FullMeetingPrep, Stakeholder, ActionWithContext, SourceReference } from "@/types";
import { cn } from "@/lib/utils";
import {
  AlertCircle,
  ArrowLeft,
  Clock,
  Users,
  FileText,
  HelpCircle,
  BookOpen,
  AlertTriangle,
  CheckCircle,
  TrendingUp,
  History,
  Target,
  MessageSquare,
} from "lucide-react";

interface MeetingPrepResult {
  status: "success" | "not_found" | "error";
  data?: FullMeetingPrep;
  message?: string;
}

export default function MeetingDetailPage() {
  const { prepFile } = useParams({ strict: false });
  const [data, setData] = useState<FullMeetingPrep | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showRaw, setShowRaw] = useState(false);

  useEffect(() => {
    async function loadPrep() {
      if (!prepFile) {
        setError("No prep file specified");
        setLoading(false);
        return;
      }

      try {
        const result = await invoke<MeetingPrepResult>("get_meeting_prep", {
          prepFile,
        });
        if (result.status === "success" && result.data) {
          setData(result.data);
        } else if (result.status === "not_found") {
          setError(result.message || "Prep file not found");
        } else if (result.status === "error") {
          setError(result.message || "Failed to load prep");
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }
    loadPrep();
  }, [prepFile]);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-10 w-3/4" />
        <Skeleton className="mb-6 h-4 w-48" />
        <div className="space-y-4">
          <Skeleton className="h-32" />
          <Skeleton className="h-48" />
          <Skeleton className="h-32" />
        </div>
      </main>
    );
  }

  if (error || !data) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Link to="/">
          <Button variant="ghost" size="sm" className="mb-4">
            <ArrowLeft className="mr-2 size-4" />
            Back to Dashboard
          </Button>
        </Link>
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="size-5" />
              <p>{error || "Meeting prep not available"}</p>
            </div>
            <p className="mt-2 text-sm text-muted-foreground">
              This meeting doesn't have a prep file yet. The system generates prep
              files for customer meetings when running /today.
            </p>
          </CardContent>
        </Card>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          {/* Back button */}
          <Link to="/">
            <Button variant="ghost" size="sm" className="mb-4">
              <ArrowLeft className="mr-2 size-4" />
              Back to Dashboard
            </Button>
          </Link>

          {/* Header */}
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">
              {data.title}
            </h1>
            {data.timeRange && (
              <div className="mt-1 flex items-center gap-2 text-muted-foreground">
                <Clock className="size-4" />
                <span>{data.timeRange}</span>
              </div>
            )}
          </div>

          {/* Toggle raw markdown */}
          <div className="mb-6">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowRaw(!showRaw)}
            >
              {showRaw ? "Show Formatted" : "Show Raw Markdown"}
            </Button>
          </div>

          {showRaw && data.rawMarkdown ? (
            <Card>
              <CardContent className="pt-6">
                <pre className="whitespace-pre-wrap text-sm font-mono">
                  {data.rawMarkdown}
                </pre>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-6">
              {/* Quick Context - metrics table */}
              {data.quickContext && data.quickContext.length > 0 && (
                <Card className="border-l-4 border-l-primary">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <TrendingUp className="size-4" />
                      Quick Context
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                      {data.quickContext.map(([metric, value], i) => (
                        <div key={i} className="rounded-md bg-muted/50 p-3">
                          <p className="text-xs font-medium text-muted-foreground">
                            {metric}
                          </p>
                          <p className="text-sm font-semibold">{value}</p>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Meeting context */}
              {data.meetingContext && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <FileText className="size-4" />
                      Context
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="whitespace-pre-wrap text-sm">{data.meetingContext}</p>
                  </CardContent>
                </Card>
              )}

              {/* Attendees */}
              {data.attendees && data.attendees.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <Users className="size-4" />
                      Key Attendees
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-3">
                      {data.attendees.map((attendee, i) => (
                        <StakeholderRow key={i} stakeholder={attendee} />
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Since Last Meeting */}
              {data.sinceLast && data.sinceLast.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <History className="size-4" />
                      Since Last Meeting
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {data.sinceLast.map((item, i) => (
                        <li key={i} className="flex items-start gap-2 text-sm">
                          <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" />
                          <span>{item}</span>
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              )}

              {/* Strategic Programs */}
              {data.strategicPrograms && data.strategicPrograms.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <Target className="size-4" />
                      Current Strategic Programs
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {data.strategicPrograms.map((item, i) => (
                        <li key={i} className="flex items-start gap-2 text-sm">
                          <span className={cn(
                            "mt-0.5",
                            item.startsWith("✓") ? "text-success" : "text-muted-foreground"
                          )}>
                            {item.startsWith("✓") ? "✓" : "○"}
                          </span>
                          <span>{item.replace(/^[✓○]\s*/, "")}</span>
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              )}

              {/* Current state */}
              {data.currentState && data.currentState.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">Current State</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {data.currentState.map((item, i) => (
                        <li key={i} className="flex items-start gap-2 text-sm">
                          <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-muted-foreground" />
                          <span>{item}</span>
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              )}

              {/* Risks */}
              {data.risks && data.risks.length > 0 && (
                <Card className="border-l-4 border-l-destructive">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <AlertTriangle className="size-4 text-destructive" />
                      Current Risks to Monitor
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {data.risks.map((risk, i) => (
                        <li key={i} className="flex items-start gap-2 text-sm">
                          <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-destructive" />
                          <span>{risk}</span>
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              )}

              {/* Suggested Talking Points */}
              {data.talkingPoints && data.talkingPoints.length > 0 && (
                <Card className="border-l-4 border-l-success">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <MessageSquare className="size-4 text-success" />
                      Suggested Talking Points
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ol className="list-decimal list-inside space-y-2">
                      {data.talkingPoints.map((point, i) => (
                        <li key={i} className="text-sm">
                          {point}
                        </li>
                      ))}
                    </ol>
                  </CardContent>
                </Card>
              )}

              {/* Open items / Actions */}
              {data.openItems && data.openItems.length > 0 && (
                <Card className="border-l-4 border-l-primary">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <CheckCircle className="size-4 text-primary" />
                      Open Items to Discuss
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-3">
                      {data.openItems.map((item, i) => (
                        <ActionItem key={i} action={item} />
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Questions */}
              {data.questions && data.questions.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <HelpCircle className="size-4" />
                      Questions to Surface
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <ol className="list-decimal list-inside space-y-2">
                      {data.questions.map((q, i) => (
                        <li key={i} className="text-sm">
                          {q}
                        </li>
                      ))}
                    </ol>
                  </CardContent>
                </Card>
              )}

              {/* Key principles */}
              {data.keyPrinciples && data.keyPrinciples.length > 0 && (
                <Card className="bg-muted/30">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <BookOpen className="size-4" />
                      Key Principles
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-3">
                      {data.keyPrinciples.map((principle, i) => (
                        <blockquote
                          key={i}
                          className="border-l-2 border-primary pl-4 text-sm italic"
                        >
                          {principle}
                        </blockquote>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* References */}
              {data.references && data.references.length > 0 && (
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base">References</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {data.references.map((ref, i) => (
                        <ReferenceRow key={i} reference={ref} />
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}
            </div>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

function StakeholderRow({ stakeholder }: { stakeholder: Stakeholder }) {
  return (
    <div className="flex items-start gap-3">
      <div className="flex size-8 items-center justify-center rounded-full bg-primary/10 text-primary">
        {stakeholder.name.charAt(0)}
      </div>
      <div>
        <p className="font-medium">{stakeholder.name}</p>
        {stakeholder.role && (
          <p className="text-sm text-muted-foreground">{stakeholder.role}</p>
        )}
        {stakeholder.focus && (
          <p className="text-sm text-muted-foreground">{stakeholder.focus}</p>
        )}
      </div>
    </div>
  );
}

function ActionItem({ action }: { action: ActionWithContext }) {
  return (
    <div
      className={cn(
        "rounded-md border p-3",
        action.isOverdue && "border-destructive bg-destructive/5"
      )}
    >
      <div className="flex items-start gap-2">
        {action.isOverdue ? (
          <AlertTriangle className="mt-0.5 size-4 text-destructive" />
        ) : (
          <CheckCircle className="mt-0.5 size-4 text-muted-foreground" />
        )}
        <div className="flex-1">
          <p className="font-medium">{action.title}</p>
          {action.dueDate && (
            <p
              className={cn(
                "text-sm",
                action.isOverdue ? "text-destructive" : "text-muted-foreground"
              )}
            >
              Due: {action.dueDate}
            </p>
          )}
          {action.context && (
            <p className="mt-1 text-sm text-muted-foreground">{action.context}</p>
          )}
        </div>
      </div>
    </div>
  );
}

function ReferenceRow({ reference }: { reference: SourceReference }) {
  return (
    <div className="flex items-center justify-between rounded-md bg-muted/50 p-2">
      <div>
        <p className="text-sm font-medium">{reference.label}</p>
        {reference.path && (
          <p className="font-mono text-xs text-muted-foreground">
            {reference.path}
          </p>
        )}
      </div>
      {reference.lastUpdated && (
        <span className="text-xs text-muted-foreground">
          {reference.lastUpdated}
        </span>
      )}
    </div>
  );
}

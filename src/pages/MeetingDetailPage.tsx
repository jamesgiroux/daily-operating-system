import { useState, useEffect } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { FullMeetingPrep, Stakeholder, StakeholderSignals, ActionWithContext, SourceReference } from "@/types";
import { cn } from "@/lib/utils";
import { CopyButton } from "@/components/ui/copy-button";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import {
  AlertCircle,
  ArrowLeft,
  Check,
  Clock,
  Copy,
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

          {/* Toggle raw markdown + Copy All */}
          <div className="mb-6 flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowRaw(!showRaw)}
            >
              {showRaw ? "Show Formatted" : "Show Raw Markdown"}
            </Button>
            <CopyAllButton data={data} />
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
                      <CopyButton text={formatQuickContext(data.quickContext!)} label="quick context" className="ml-auto" />
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

              {/* Relationship Context (I43) */}
              {data.stakeholderSignals && (
                <RelationshipContext signals={data.stakeholderSignals} />
              )}

              {/* Meeting context */}
              {data.meetingContext && (
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <FileText className="size-4" />
                      Context
                      <CopyButton text={data.meetingContext} label="context" className="ml-auto" />
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
                      <CopyButton text={formatAttendees(data.attendees)} label="attendees" className="ml-auto" />
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
                      <CopyButton text={formatBulletList(data.sinceLast)} label="since last meeting" className="ml-auto" />
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
                      <CopyButton text={formatBulletList(data.strategicPrograms)} label="programs" className="ml-auto" />
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
                    <CardTitle className="flex items-center gap-2 text-base">
                      Current State
                      <CopyButton text={formatBulletList(data.currentState)} label="current state" className="ml-auto" />
                    </CardTitle>
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
                      <CopyButton text={formatBulletList(data.risks)} label="risks" className="ml-auto" />
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
                      <CopyButton text={formatNumberedList(data.talkingPoints)} label="talking points" className="ml-auto" />
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
                      <CopyButton text={formatOpenItems(data.openItems)} label="open items" className="ml-auto" />
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
                      <CopyButton text={formatNumberedList(data.questions)} label="questions" className="ml-auto" />
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
                      <CopyButton text={formatBulletList(data.keyPrinciples)} label="principles" className="ml-auto" />
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

function CopyAllButton({ data }: { data: FullMeetingPrep }) {
  const { copied, copy } = useCopyToClipboard();

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={() => copy(formatFullPrep(data))}
    >
      {copied ? (
        <Check className="mr-2 size-3.5 text-success" />
      ) : (
        <Copy className="mr-2 size-3.5" />
      )}
      {copied ? "Copied!" : "Copy All"}
    </Button>
  );
}

function RelationshipContext({ signals }: { signals: StakeholderSignals }) {
  const tempColor = {
    hot: "text-success",
    warm: "text-primary",
    cool: "text-muted-foreground",
    cold: "text-destructive",
  }[signals.temperature] ?? "text-muted-foreground";

  const trendLabel = {
    increasing: "Increasing",
    stable: "Stable",
    decreasing: "Decreasing",
  }[signals.trend] ?? signals.trend;

  const lastMeetingText = signals.lastMeeting
    ? formatRelativeDate(signals.lastMeeting)
    : "No meetings recorded";

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <History className="size-4" />
          Relationship Context
          <CopyButton text={formatRelationshipContext(signals)} label="relationship context" className="ml-auto" />
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div className="rounded-md bg-muted/50 p-3">
            <p className="text-xs font-medium text-muted-foreground">Last Meeting</p>
            <p className={cn("text-sm font-semibold", tempColor)}>
              {lastMeetingText}
            </p>
          </div>
          <div className="rounded-md bg-muted/50 p-3">
            <p className="text-xs font-medium text-muted-foreground">Temperature</p>
            <p className={cn("text-sm font-semibold capitalize", tempColor)}>
              {signals.temperature}
            </p>
          </div>
          <div className="rounded-md bg-muted/50 p-3">
            <p className="text-xs font-medium text-muted-foreground">Last 30 Days</p>
            <p className="text-sm font-semibold">
              {signals.meetingFrequency30d} meeting{signals.meetingFrequency30d !== 1 ? "s" : ""}
            </p>
          </div>
          <div className="rounded-md bg-muted/50 p-3">
            <p className="text-xs font-medium text-muted-foreground">Trend</p>
            <p className="text-sm font-semibold">{trendLabel}</p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function formatRelativeDate(iso: string): string {
  const date = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays} days ago`;
  if (diffDays < 30) {
    const weeks = Math.floor(diffDays / 7);
    return `${weeks} week${weeks !== 1 ? "s" : ""} ago`;
  }
  const months = Math.floor(diffDays / 30);
  return `${months} month${months !== 1 ? "s" : ""} ago`;
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

// =============================================================================
// Copy-to-clipboard formatters
// Output is clean plaintext with light markdown — pastes well into Slack,
// email, and docs.
// =============================================================================

function formatBulletList(items: string[]): string {
  return items.map((item) => `- ${item}`).join("\n");
}

function formatNumberedList(items: string[]): string {
  return items.map((item, i) => `${i + 1}. ${item}`).join("\n");
}

function formatQuickContext(items: [string, string][]): string {
  return items.map(([key, value]) => `${key}: ${value}`).join("\n");
}

function formatRelationshipContext(signals: StakeholderSignals): string {
  const lastMeeting = signals.lastMeeting
    ? formatRelativeDate(signals.lastMeeting)
    : "No meetings recorded";
  return [
    `Last Meeting: ${lastMeeting}`,
    `Temperature: ${signals.temperature}`,
    `Last 30 Days: ${signals.meetingFrequency30d} meeting${signals.meetingFrequency30d !== 1 ? "s" : ""}`,
    `Trend: ${signals.trend}`,
  ].join("\n");
}

function formatAttendees(attendees: Stakeholder[]): string {
  return attendees
    .map((a) => {
      const parts = [a.name];
      if (a.role) parts.push(a.role);
      return `- ${parts.join(" — ")}`;
    })
    .join("\n");
}

function formatOpenItems(items: ActionWithContext[]): string {
  return items
    .map((item) => {
      let line = `- ${item.title}`;
      if (item.dueDate) line += ` (due: ${item.dueDate})`;
      if (item.isOverdue) line += " [OVERDUE]";
      return line;
    })
    .join("\n");
}

function formatFullPrep(data: FullMeetingPrep): string {
  const sections: string[] = [];

  // Header
  sections.push(`# ${data.title}`);
  if (data.timeRange) sections.push(data.timeRange);

  if (data.quickContext && data.quickContext.length > 0) {
    sections.push(`\n## Quick Context\n${formatQuickContext(data.quickContext)}`);
  }

  if (data.stakeholderSignals) {
    sections.push(`\n## Relationship Context\n${formatRelationshipContext(data.stakeholderSignals)}`);
  }

  if (data.meetingContext) {
    sections.push(`\n## Context\n${data.meetingContext}`);
  }

  if (data.attendees && data.attendees.length > 0) {
    sections.push(`\n## Key Attendees\n${formatAttendees(data.attendees)}`);
  }

  if (data.sinceLast && data.sinceLast.length > 0) {
    sections.push(`\n## Since Last Meeting\n${formatBulletList(data.sinceLast)}`);
  }

  if (data.strategicPrograms && data.strategicPrograms.length > 0) {
    sections.push(`\n## Current Strategic Programs\n${formatBulletList(data.strategicPrograms)}`);
  }

  if (data.currentState && data.currentState.length > 0) {
    sections.push(`\n## Current State\n${formatBulletList(data.currentState)}`);
  }

  if (data.risks && data.risks.length > 0) {
    sections.push(`\n## Risks\n${formatBulletList(data.risks)}`);
  }

  if (data.talkingPoints && data.talkingPoints.length > 0) {
    sections.push(`\n## Talking Points\n${formatNumberedList(data.talkingPoints)}`);
  }

  if (data.openItems && data.openItems.length > 0) {
    sections.push(`\n## Open Items\n${formatOpenItems(data.openItems)}`);
  }

  if (data.questions && data.questions.length > 0) {
    sections.push(`\n## Questions\n${formatNumberedList(data.questions)}`);
  }

  if (data.keyPrinciples && data.keyPrinciples.length > 0) {
    sections.push(`\n## Key Principles\n${formatBulletList(data.keyPrinciples)}`);
  }

  return sections.join("\n");
}

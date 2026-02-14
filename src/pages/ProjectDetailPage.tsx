import { useState, useEffect, useCallback } from "react";
import { useParams, Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { EmailSignalList } from "@/components/ui/email-signal-list";
import {
  StatusBadge,
  projectStatusStyles,
  progressStyles,
} from "@/components/ui/status-badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { PageError } from "@/components/PageState";
import { cn, formatShortDate } from "@/lib/utils";
import {
  Archive,
  ArrowLeft,
  Calendar,
  CheckCircle2,
  Loader2,
  Minus,
  Save,
  Sparkles,
  TrendingDown,
  TrendingUp,
  Users,
} from "lucide-react";
import type { ProjectDetail } from "@/types";

const statusOptions = ["active", "on_hold", "completed", "archived"];


const temperatureStyles: Record<string, string> = {
  hot: "bg-destructive/15 text-destructive",
  warm: "bg-primary/15 text-primary",
  cool: "bg-muted text-muted-foreground",
  cold: "bg-muted text-muted-foreground/60",
};

function TrendIcon({ trend }: { trend: string }) {
  switch (trend) {
    case "increasing":
      return <TrendingUp className="size-4 text-green-600" />;
    case "decreasing":
      return <TrendingDown className="size-4 text-destructive" />;
    default:
      return <Minus className="size-4 text-muted-foreground" />;
  }
}

export default function ProjectDetailPage() {
  const { projectId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Editable structured fields
  const [editName, setEditName] = useState<string>("");
  const [editStatus, setEditStatus] = useState<string>("");
  const [editMilestone, setEditMilestone] = useState<string>("");
  const [editOwner, setEditOwner] = useState<string>("");
  const [editTargetDate, setEditTargetDate] = useState<string>("");
  const [editNotes, setEditNotes] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [enriching, setEnriching] = useState(false);

  const load = useCallback(async () => {
    if (!projectId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ProjectDetail>("get_project_detail", {
        projectId,
      });
      setDetail(result);
      setEditName(result.name);
      setEditStatus(result.status ?? "active");
      setEditMilestone(result.milestone ?? "");
      setEditOwner(result.owner ?? "");
      setEditTargetDate(result.targetDate ?? "");
      setEditNotes(result.notes ?? "");
      setDirty(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    load();
  }, [load]);

  async function handleSave() {
    if (!detail) return;
    setSaving(true);

    try {
      const fieldUpdates: [string, string][] = [];
      if (editName !== detail.name)
        fieldUpdates.push(["name", editName]);
      if (editStatus !== (detail.status ?? ""))
        fieldUpdates.push(["status", editStatus]);
      if (editMilestone !== (detail.milestone ?? ""))
        fieldUpdates.push(["milestone", editMilestone]);
      if (editOwner !== (detail.owner ?? ""))
        fieldUpdates.push(["owner", editOwner]);
      if (editTargetDate !== (detail.targetDate ?? ""))
        fieldUpdates.push(["target_date", editTargetDate]);

      for (const [field, value] of fieldUpdates) {
        await invoke("update_project_field", {
          projectId: detail.id,
          field,
          value,
        });
      }

      if (editNotes !== (detail.notes ?? "")) {
        await invoke("update_project_notes", {
          projectId: detail.id,
          notes: editNotes,
        });
      }

      setDirty(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleEnrich() {
    if (!detail) return;
    setEnriching(true);
    try {
      await invoke("enrich_project", { projectId: detail.id });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setEnriching(false);
    }
  }

  async function handleArchive() {
    if (!detail) return;
    try {
      await invoke("archive_project", { id: detail.id, archived: true });
      navigate({ to: "/projects" });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnarchive() {
    if (!detail) return;
    try {
      await invoke("archive_project", { id: detail.id, archived: false });
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-10 w-64" />
        <div className="mt-6 grid gap-4 lg:grid-cols-2">
          <Skeleton className="h-48" />
          <Skeleton className="h-48" />
        </div>
      </main>
    );
  }

  if (error || !detail) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error ?? "Project not found"} onRetry={load} />
      </main>
    );
  }

  const signals = detail.signals;

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          {/* Back + header */}
          <Link
            to="/projects"
            className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
            Projects
          </Link>

          {detail.archived && (
            <div className="mb-4 rounded-lg border border-primary/30 bg-primary/5 px-4 py-3 flex items-center justify-between">
              <span className="text-sm text-charcoal/70">This project is archived and hidden from active views.</span>
              <Button variant="outline" size="sm" onClick={handleUnarchive}>
                Unarchive
              </Button>
            </div>
          )}

          <div className="mb-6 flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="flex size-12 items-center justify-center rounded-full bg-primary/10 text-lg font-semibold text-primary">
                {detail.name.charAt(0).toUpperCase()}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-2xl font-semibold tracking-tight">
                    {detail.name}
                  </h1>
                  <StatusBadge
                    value={detail.status}
                    styles={projectStatusStyles}
                    fallback={projectStatusStyles.active}
                  />
                  {signals && (
                    <Badge
                      className={cn(
                        "text-xs",
                        temperatureStyles[signals.temperature] ??
                          temperatureStyles.cool
                      )}
                    >
                      {signals.temperature}
                    </Badge>
                  )}
                </div>
                <span className="text-sm text-muted-foreground">
                  {detail.owner && <>Owner: {detail.owner}</>}
                  {detail.owner && detail.targetDate && <> &middot; </>}
                  {detail.targetDate && <>Target: {detail.targetDate}</>}
                </span>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {dirty && (
                <Button size="sm" onClick={handleSave} disabled={saving}>
                  <Save className="mr-1 size-4" />
                  {saving ? "Saving..." : "Save"}
                </Button>
              )}
              {!detail.archived && (
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button variant="ghost" size="sm" className="h-8 text-xs text-muted-foreground">
                      <Archive className="mr-1 size-3" />
                      Archive
                    </Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Archive Project</AlertDialogTitle>
                      <AlertDialogDescription>
                        Archive "{detail.name}"? It will be hidden from active views.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel>Cancel</AlertDialogCancel>
                      <AlertDialogAction onClick={handleArchive}>Archive</AlertDialogAction>
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
              )}
            </div>
          </div>

          <div className="grid gap-4 lg:grid-cols-2">
            {/* Quick View (editable structured fields) */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">
                  Quick View
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <label className="text-xs font-medium text-muted-foreground">
                    Name
                  </label>
                  <input
                    type="text"
                    value={editName}
                    onChange={(e) => {
                      setEditName(e.target.value);
                      setDirty(true);
                    }}
                    placeholder="Project name"
                    className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="text-xs font-medium text-muted-foreground">
                      Status
                    </label>
                    <select
                      value={editStatus}
                      onChange={(e) => {
                        setEditStatus(e.target.value);
                        setDirty(true);
                      }}
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                    >
                      {statusOptions.map((s) => (
                        <option key={s} value={s}>
                          {s.replace("_", " ")}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label className="text-xs font-medium text-muted-foreground">
                      Owner
                    </label>
                    <input
                      type="text"
                      value={editOwner}
                      onChange={(e) => {
                        setEditOwner(e.target.value);
                        setDirty(true);
                      }}
                      placeholder="Project owner"
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                    />
                  </div>
                  <div>
                    <label className="text-xs font-medium text-muted-foreground">
                      Current Milestone
                    </label>
                    <input
                      type="text"
                      value={editMilestone}
                      onChange={(e) => {
                        setEditMilestone(e.target.value);
                        setDirty(true);
                      }}
                      placeholder="Current milestone"
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                    />
                  </div>
                  <div>
                    <label className="text-xs font-medium text-muted-foreground">
                      Target Date
                    </label>
                    <input
                      type="date"
                      value={editTargetDate}
                      onChange={(e) => {
                        setEditTargetDate(e.target.value);
                        setDirty(true);
                      }}
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Activity Signals */}
            {signals && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Activity Signals
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <div className="text-2xl font-semibold">
                        {signals.meetingFrequency30d}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        meetings (30d)
                      </div>
                    </div>
                    <div>
                      <div className="text-2xl font-semibold">
                        {signals.meetingFrequency90d}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        meetings (90d)
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <TrendIcon trend={signals.trend} />
                      <div>
                        <div className="text-sm font-medium capitalize">
                          {signals.trend}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          trend
                        </div>
                      </div>
                    </div>
                    {signals.lastMeeting && (
                      <div>
                        <div className="text-sm font-medium">
                          {formatShortDate(signals.lastMeeting)}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          last meeting
                        </div>
                      </div>
                    )}
                    {signals.daysUntilTarget != null && (
                      <div>
                        <div className="text-sm font-medium">
                          {signals.daysUntilTarget}d
                        </div>
                        <div className="text-xs text-muted-foreground">
                          until target
                        </div>
                      </div>
                    )}
                    <div>
                      <div className="text-sm font-medium">
                        {signals.openActionCount}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        open actions
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Description + Enrich */}
            <Card>
              <CardHeader className="flex flex-row items-center justify-between pb-3">
                <CardTitle className="text-sm font-medium">
                  Description
                </CardTitle>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleEnrich}
                  disabled={enriching}
                  className="h-7 text-xs"
                >
                  {enriching ? (
                    <Loader2 className="mr-1 size-3 animate-spin" />
                  ) : (
                    <Sparkles className="mr-1 size-3" />
                  )}
                  {enriching
                    ? "Researching..."
                    : detail.description
                      ? "Refresh"
                      : "Enrich"}
                </Button>
              </CardHeader>
              <CardContent className="text-sm">
                {detail.description ? (
                  <p>{detail.description}</p>
                ) : (
                  <p className="text-muted-foreground">
                    No description yet. Click Enrich to research this project.
                  </p>
                )}
              </CardContent>
            </Card>

            {/* Notes (editable) */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Notes</CardTitle>
              </CardHeader>
              <CardContent>
                <textarea
                  value={editNotes}
                  onChange={(e) => {
                    setEditNotes(e.target.value);
                    setDirty(true);
                  }}
                  placeholder="Notes about this project..."
                  rows={4}
                  className="w-full resize-none rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                />
              </CardContent>
            </Card>

            {/* Milestones */}
            {detail.milestones.length > 0 && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Milestones
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2">
                    {detail.milestones.map((m, i) => (
                      <div key={i} className="flex items-center gap-2 text-sm">
                        <StatusBadge
                          value={m.status}
                          styles={progressStyles}
                          fallback={progressStyles.planned}
                        />
                        <span className="font-medium">{m.name}</span>
                        {m.targetDate && (
                          <span className="text-muted-foreground">
                            &mdash; {m.targetDate}
                          </span>
                        )}
                        {m.notes && (
                          <span className="text-muted-foreground">
                            ({m.notes})
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Open Actions */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">
                  Open Actions
                  {detail.openActions.length > 0 && (
                    <span className="ml-1 text-muted-foreground">
                      ({detail.openActions.length})
                    </span>
                  )}
                </CardTitle>
              </CardHeader>
              <CardContent>
                {detail.openActions.length > 0 ? (
                  <div className="space-y-2">
                    {detail.openActions.map((a) => (
                      <div
                        key={a.id}
                        className="flex items-center gap-2 text-sm"
                      >
                        <CheckCircle2 className="size-3.5 shrink-0 text-muted-foreground" />
                        <Badge variant="outline" className="text-xs shrink-0">
                          {a.priority}
                        </Badge>
                        <span className="truncate">{a.title}</span>
                        {a.dueDate && (
                          <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                            {a.dueDate}
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    No open actions.
                  </p>
                )}
              </CardContent>
            </Card>

            {/* Recent Meetings */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">
                  Recent Meetings
                </CardTitle>
              </CardHeader>
              <CardContent>
                {detail.recentMeetings.length > 0 ? (
                  <div className="space-y-2">
                    {detail.recentMeetings.map((m) => (
                      <div
                        key={m.id}
                        className="flex items-center gap-2 text-sm"
                      >
                        <Calendar className="size-3.5 shrink-0 text-muted-foreground" />
                        <span className="truncate">{m.title}</span>
                        <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                          {formatShortDate(m.startTime)}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    No meetings recorded yet.
                  </p>
                )}
              </CardContent>
            </Card>

            {/* Team */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Team</CardTitle>
              </CardHeader>
              <CardContent>
                {detail.linkedPeople.length > 0 ? (
                  <div className="space-y-2">
                    {detail.linkedPeople.map((p) => (
                      <Link
                        key={p.id}
                        to="/people/$personId"
                        params={{ personId: p.id }}
                        className="flex items-center gap-2 text-sm transition-colors hover:text-primary"
                      >
                        <Users className="size-3.5 shrink-0 text-muted-foreground" />
                        <span className="font-medium">{p.name}</span>
                        {p.role && (
                          <span className="text-muted-foreground">
                            {p.role}
                          </span>
                        )}
                      </Link>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    No people linked yet.
                  </p>
                )}
              </CardContent>
            </Card>

            {/* Recent Captures */}
            {detail.recentCaptures.length > 0 && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Recent Captures
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2">
                    {detail.recentCaptures.map((c) => (
                      <div
                        key={c.id}
                        className="flex items-start gap-2 text-sm"
                      >
                        <CaptureIcon type={c.captureType} />
                        <div className="min-w-0">
                          <span className="truncate">{c.content}</span>
                          <div className="text-xs text-muted-foreground">
                            {c.meetingTitle}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </CardContent>
              </Card>
            )}

            {detail.recentEmailSignals && detail.recentEmailSignals.length > 0 && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Email Timeline
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <EmailSignalList
                    signals={detail.recentEmailSignals}
                    limit={8}
                    dateFormat="absolute"
                    showMetadata
                  />
                </CardContent>
              </Card>
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}


function CaptureIcon({ type }: { type: string }) {
  const styles: Record<string, string> = {
    win: "text-green-600",
    risk: "text-destructive",
    decision: "text-primary",
  };
  const labels: Record<string, string> = {
    win: "W",
    risk: "R",
    decision: "D",
  };
  return (
    <span
      className={cn(
        "inline-flex size-5 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-bold",
        styles[type] ?? "text-muted-foreground"
      )}
    >
      {labels[type] ?? "?"}
    </span>
  );
}



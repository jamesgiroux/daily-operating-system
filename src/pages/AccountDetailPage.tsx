import { useState, useEffect, useCallback } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  StatusBadge,
  healthStyles,
  progressStyles,
} from "@/components/ui/status-badge";
import { PageError } from "@/components/PageState";
import { cn, formatArr, formatFileSize, formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  ArrowLeft,
  Calendar,
  CalendarClock,
  CheckCircle2,
  ExternalLink,
  File,
  FileText,
  Loader2,
  Pencil,
  RefreshCw,
  Save,
  Plus,
  Sparkles,
  Users,
  X,
} from "lucide-react";
import type {
  AccountDetail,
  AccountHealth,
  AccountChildSummary,
  ContentFile,
  ParentAggregate,
} from "@/types";

const healthOptions: AccountHealth[] = ["green", "yellow", "red"];

const temperatureStyles: Record<string, string> = {
  hot: "bg-destructive/15 text-destructive",
  warm: "bg-primary/15 text-primary",
  cool: "bg-muted text-muted-foreground",
  cold: "bg-muted text-muted-foreground/60",
};

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const [detail, setDetail] = useState<AccountDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);

  // Editable structured fields
  const [editHealth, setEditHealth] = useState<string>("");
  const [editLifecycle, setEditLifecycle] = useState<string>("");
  const [editArr, setEditArr] = useState<string>("");
  const [editNps, setEditNps] = useState<string>("");
  const [editCsm, setEditCsm] = useState<string>("");
  const [editChampion, setEditChampion] = useState<string>("");
  const [editRenewal, setEditRenewal] = useState<string>("");
  const [editNotes, setEditNotes] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [enriching, setEnriching] = useState(false);

  // I127: Inline action creation
  const [addingAction, setAddingAction] = useState(false);
  const [newActionTitle, setNewActionTitle] = useState("");
  const [creatingAction, setCreatingAction] = useState(false);

  // I124: Content index state
  const [files, setFiles] = useState<ContentFile[]>([]);
  const [indexing, setIndexing] = useState(false);
  const [newFileCount, setNewFileCount] = useState(0);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [indexFeedback, setIndexFeedback] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!accountId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AccountDetail>("get_account_detail", {
        accountId,
      });
      setDetail(result);
      setEditHealth(result.health ?? "");
      setEditLifecycle(result.lifecycle ?? "");
      setEditArr(result.arr?.toString() ?? "");
      setEditNps(result.nps?.toString() ?? "");
      setEditCsm(result.csm ?? "");
      setEditChampion(result.champion ?? "");
      setEditRenewal(result.renewalDate ?? "");
      setEditNotes(result.notes ?? "");
      setDirty(false);
      // I124: Load content files
      try {
        const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
          entityId: accountId,
        });
        setFiles(contentFiles);
      } catch {
        // Non-critical — don't block page load
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    load();
  }, [load]);

  // I125: Listen for content-changed events from watcher
  useEffect(() => {
    const unlisten = listen<{ entityIds: string[]; count: number }>(
      "content-changed",
      (event) => {
        if (accountId && event.payload.entityIds.includes(accountId)) {
          setNewFileCount(event.payload.count);
          setBannerDismissed(false);
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [accountId]);

  async function handleSave() {
    if (!detail) return;
    setSaving(true);

    try {
      const fieldUpdates: [string, string][] = [];
      if (editHealth !== (detail.health ?? ""))
        fieldUpdates.push(["health", editHealth]);
      if (editLifecycle !== (detail.lifecycle ?? ""))
        fieldUpdates.push(["lifecycle", editLifecycle]);
      if (editArr !== (detail.arr?.toString() ?? ""))
        fieldUpdates.push(["arr", editArr]);
      if (editNps !== (detail.nps?.toString() ?? ""))
        fieldUpdates.push(["nps", editNps]);
      if (editCsm !== (detail.csm ?? ""))
        fieldUpdates.push(["csm", editCsm]);
      if (editChampion !== (detail.champion ?? ""))
        fieldUpdates.push(["champion", editChampion]);
      if (editRenewal !== (detail.renewalDate ?? ""))
        fieldUpdates.push(["contract_end", editRenewal]);

      for (const [field, value] of fieldUpdates) {
        await invoke("update_account_field", {
          accountId: detail.id,
          field,
          value,
        });
      }

      if (editNotes !== (detail.notes ?? "")) {
        await invoke("update_account_notes", {
          accountId: detail.id,
          notes: editNotes,
        });
      }

      setDirty(false);
      setEditing(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function handleCancelEdit() {
    if (!detail) return;
    setEditHealth(detail.health ?? "");
    setEditLifecycle(detail.lifecycle ?? "");
    setEditArr(detail.arr?.toString() ?? "");
    setEditNps(detail.nps?.toString() ?? "");
    setEditCsm(detail.csm ?? "");
    setEditChampion(detail.champion ?? "");
    setEditRenewal(detail.renewalDate ?? "");
    setDirty(false);
    setEditing(false);
  }

  async function handleIndexFiles() {
    if (!detail) return;
    setIndexing(true);
    setIndexFeedback(null);
    try {
      const updated = await invoke<ContentFile[]>("index_entity_files", {
        entityId: detail.id,
      });
      const diff = updated.length - files.length;
      setFiles(updated);
      setNewFileCount(0);
      setBannerDismissed(true);
      // Show brief feedback
      if (diff > 0) {
        setIndexFeedback(`${diff} new file${diff !== 1 ? "s" : ""} found`);
      } else {
        setIndexFeedback("Up to date");
      }
      setTimeout(() => setIndexFeedback(null), 3000);
    } catch (e) {
      setError(String(e));
    } finally {
      setIndexing(false);
    }
  }

  async function handleEnrich() {
    if (!detail) return;
    setEnriching(true);
    try {
      await invoke("enrich_account", { accountId: detail.id });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setEnriching(false);
    }
  }

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-8">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-12 w-64" />
        <Skeleton className="mt-4 h-20 w-full" />
        <div className="mt-8 grid gap-6 lg:grid-cols-[3fr_2fr]">
          <div className="space-y-6">
            <Skeleton className="h-40" />
            <Skeleton className="h-32" />
          </div>
          <div className="space-y-6">
            <Skeleton className="h-48" />
            <Skeleton className="h-32" />
          </div>
        </div>
      </main>
    );
  }

  if (error || !detail) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error ?? "Account not found"} onRetry={load} />
      </main>
    );
  }

  const signals = detail.signals;

  // Build metrics array — only include items with data
  const metrics: { label: string; value: string; color?: string }[] = [];
  if (detail.arr != null) {
    metrics.push({ label: "ARR", value: `$${formatArr(detail.arr)}` });
  }
  if (signals) {
    const trendLabel =
      signals.trend === "increasing"
        ? " \u2191"
        : signals.trend === "decreasing"
          ? " \u2193"
          : "";
    metrics.push({
      label: "Meetings (30d)",
      value: `${signals.meetingFrequency30d}${trendLabel}`,
    });
    if (signals.meetingFrequency90d > 0) {
      metrics.push({
        label: "Meetings (90d)",
        value: String(signals.meetingFrequency90d),
      });
    }
  }
  if (signals?.temperature) {
    metrics.push({
      label: "Engagement",
      value: signals.temperature,
      color: temperatureStyles[signals.temperature],
    });
  }
  if (detail.renewalDate) {
    metrics.push({
      label: "Renewal",
      value: formatRenewalCountdown(detail.renewalDate),
    });
  }
  if (detail.nps != null) {
    metrics.push({ label: "NPS", value: String(detail.nps) });
  }
  if (signals?.lastMeeting) {
    metrics.push({
      label: "Last Meeting",
      value: formatDate(signals.lastMeeting),
    });
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="space-y-8 p-8">
          {/* Back link / Breadcrumb */}
          <div className="flex items-center gap-1 text-sm text-muted-foreground">
            <Link
              to="/accounts"
              className="inline-flex items-center gap-1 transition-colors hover:text-foreground"
            >
              <ArrowLeft className="size-4" />
              Accounts
            </Link>
            {detail.parentId && detail.parentName && (
              <>
                <span className="mx-1">/</span>
                <Link
                  to="/accounts/$accountId"
                  params={{ accountId: detail.parentId }}
                  className="transition-colors hover:text-foreground"
                >
                  {detail.parentName}
                </Link>
              </>
            )}
          </div>

          {/* Hero Section */}
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <div className="flex size-14 items-center justify-center rounded-full bg-primary/10 text-xl font-semibold text-primary">
                {detail.name.charAt(0).toUpperCase()}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-3xl font-semibold tracking-tight">
                    {detail.name}
                  </h1>
                  {detail.health && (
                    <StatusBadge value={detail.health} styles={healthStyles} />
                  )}
                  {detail.lifecycle && (
                    <Badge variant="outline" className="text-xs capitalize">
                      {detail.lifecycle}
                    </Badge>
                  )}
                  {signals?.temperature && (
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
                {detail.csm && (
                  <span className="text-sm text-muted-foreground">
                    CSM: {detail.csm}
                  </span>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2">
              {dirty && (
                <Button size="sm" onClick={handleSave} disabled={saving}>
                  <Save className="mr-1 size-4" />
                  {saving ? "Saving..." : "Save"}
                </Button>
              )}
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleEnrich}
                      disabled={enriching}
                      className="h-8 text-xs"
                    >
                      {enriching ? (
                        <Loader2 className="mr-1 size-3 animate-spin" />
                      ) : (
                        <Sparkles className="mr-1 size-3" />
                      )}
                      {enriching
                        ? "Researching..."
                        : detail.companyOverview
                          ? "Refresh"
                          : "Enrich"}
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    {detail.companyOverview
                      ? files.length > 0
                        ? `Refresh using AI — includes ${files.length} workspace file${files.length !== 1 ? "s" : ""}`
                        : "Refresh company data using AI"
                      : files.length > 0
                        ? `Research this company using AI — includes ${files.length} workspace file${files.length !== 1 ? "s" : ""}`
                        : "Research this company using AI"}
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          {/* Metrics Row */}
          {metrics.length > 0 && (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
              {metrics.map((m) => (
                <div
                  key={m.label}
                  className="rounded-lg border bg-card px-4 py-3"
                >
                  <div
                    className={cn(
                      "text-xl font-semibold",
                      m.color && "capitalize"
                    )}
                  >
                    {m.value}
                  </div>
                  <div className="text-xs text-muted-foreground">{m.label}</div>
                </div>
              ))}
            </div>
          )}

          {/* I114: Portfolio Aggregate for parent accounts */}
          {detail.parentAggregate && (
            <PortfolioRow aggregate={detail.parentAggregate} />
          )}

          {/* Asymmetric Grid: Main (3fr) + Sidebar (2fr) */}
          <div className="grid gap-6 lg:grid-cols-[3fr_2fr]">
            {/* ═══ Main Column ═══ */}
            <div className="space-y-6">
              {/* Upcoming Meetings */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Upcoming Meetings
                    {detail.upcomingMeetings.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({detail.upcomingMeetings.length})
                      </span>
                    )}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {detail.upcomingMeetings.length > 0 ? (
                    <div className="space-y-2">
                      {detail.upcomingMeetings.map((m) => (
                        <div
                          key={m.id}
                          className="flex items-center gap-3 rounded-lg border px-4 py-3"
                        >
                          <Badge variant="outline" className="shrink-0 text-xs">
                            {formatMeetingType(m.meetingType)}
                          </Badge>
                          <span className="flex-1 truncate font-medium">
                            {m.title}
                          </span>
                          <span className="shrink-0 text-sm text-muted-foreground">
                            {formatRelativeDate(m.startTime)}
                          </span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <EmptyState
                      icon={CalendarClock}
                      message="No upcoming meetings scheduled"
                    />
                  )}
                </CardContent>
              </Card>

              {/* Business Units (I114 — parent accounts only) */}
              {detail.children.length > 0 && (
                <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Business Units
                      <span className="ml-1 text-muted-foreground">
                        ({detail.children.length})
                      </span>
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {detail.children.map((child) => (
                        <ChildAccountRow key={child.id} child={child} />
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Open Actions */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Open Actions
                    {detail.openActions.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({detail.openActions.length})
                      </span>
                    )}
                  </CardTitle>
                  <div className="flex items-center gap-2">
                    {!addingAction && (
                      <button
                        onClick={() => setAddingAction(true)}
                        className="inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                      >
                        <Plus className="size-3" />
                        Add action
                      </button>
                    )}
                    {detail.openActions.length > 0 && (
                      <Link
                        to="/actions"
                        search={{ search: detail.id }}
                        className="inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                      >
                        View all
                        <ExternalLink className="size-3" />
                      </Link>
                    )}
                  </div>
                </CardHeader>
                <CardContent>
                  {addingAction && (
                    <div className="mb-3 flex items-center gap-2">
                      <input
                        type="text"
                        autoFocus
                        value={newActionTitle}
                        onChange={(e) => setNewActionTitle(e.target.value)}
                        onKeyDown={async (e) => {
                          if (e.key === "Enter" && newActionTitle.trim()) {
                            setCreatingAction(true);
                            try {
                              await invoke("create_action", {
                                title: newActionTitle.trim(),
                                accountId: detail.id,
                              });
                              setNewActionTitle("");
                              setAddingAction(false);
                              load();
                            } finally {
                              setCreatingAction(false);
                            }
                          }
                          if (e.key === "Escape") {
                            setAddingAction(false);
                            setNewActionTitle("");
                          }
                        }}
                        placeholder="Action title... (Enter to create)"
                        disabled={creatingAction}
                        className="flex-1 rounded-md border bg-background px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring"
                      />
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 text-xs"
                        onClick={() => {
                          setAddingAction(false);
                          setNewActionTitle("");
                        }}
                      >
                        Cancel
                      </Button>
                    </div>
                  )}
                  {detail.openActions.length > 0 ? (
                    <div className="space-y-2">
                      {detail.openActions.map((a) => (
                        <Link
                          key={a.id}
                          to="/actions/$actionId"
                          params={{ actionId: a.id }}
                          className="flex items-center gap-2 rounded-md px-1 py-0.5 text-sm transition-colors hover:bg-muted"
                        >
                          <CheckCircle2 className="size-3.5 shrink-0 text-muted-foreground" />
                          <Badge variant="outline" className="shrink-0 text-xs">
                            {a.priority}
                          </Badge>
                          <span className="truncate">{a.title}</span>
                          {a.dueDate && (
                            <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                              {a.dueDate}
                            </span>
                          )}
                        </Link>
                      ))}
                    </div>
                  ) : (
                    !addingAction && (
                      <EmptyState
                        icon={CheckCircle2}
                        message="No open actions"
                      />
                    )
                  )}
                </CardContent>
              </Card>

              {/* Company Overview */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Company Overview
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  {detail.companyOverview ? (
                    <>
                      {detail.companyOverview.description && (
                        <p>{detail.companyOverview.description}</p>
                      )}
                      <div className="flex flex-wrap gap-x-4 gap-y-1 text-muted-foreground">
                        {detail.companyOverview.industry && (
                          <span>
                            Industry: {detail.companyOverview.industry}
                          </span>
                        )}
                        {detail.companyOverview.size && (
                          <span>Size: {detail.companyOverview.size}</span>
                        )}
                        {detail.companyOverview.headquarters && (
                          <span>
                            HQ: {detail.companyOverview.headquarters}
                          </span>
                        )}
                      </div>
                    </>
                  ) : (
                    <p className="text-muted-foreground">
                      No company data yet. Click Enrich to research this
                      company.
                    </p>
                  )}
                </CardContent>
              </Card>

              {/* Recent Meetings */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Recent Meetings
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {detail.recentMeetings.length > 0 ? (
                    <div className="space-y-2">
                      {detail.recentMeetings.map((m) => (
                        <Link
                          key={m.id}
                          to="/meeting/history/$meetingId"
                          params={{ meetingId: m.id }}
                          className="flex items-center gap-3 rounded-lg border px-4 py-3 transition-colors hover:bg-muted"
                        >
                          <Badge variant="outline" className="shrink-0 text-xs">
                            {formatMeetingType(m.meetingType)}
                          </Badge>
                          <span className="flex-1 truncate font-medium">
                            {m.title}
                          </span>
                          <span className="shrink-0 text-sm text-muted-foreground">
                            {formatRelativeDate(m.startTime)}
                          </span>
                        </Link>
                      ))}
                    </div>
                  ) : (
                    <EmptyState
                      icon={Calendar}
                      message="No meetings recorded yet"
                    />
                  )}
                </CardContent>
              </Card>

              {/* Recent Captures */}
              {detail.recentCaptures.length > 0 && (
                <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Recent Captures
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {detail.recentCaptures.map((c) => (
                        <Link
                          key={c.id}
                          to="/meeting/history/$meetingId"
                          params={{ meetingId: c.meetingId }}
                          className="flex items-start gap-2 rounded-md px-1 py-0.5 text-sm transition-colors hover:bg-muted"
                        >
                          <CaptureIcon type={c.captureType} />
                          <div className="min-w-0">
                            <span className="truncate">{c.content}</span>
                            <div className="text-xs text-muted-foreground">
                              {c.meetingTitle}
                            </div>
                          </div>
                        </Link>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Files (I124) */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Files
                    {files.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({files.length})
                      </span>
                    )}
                  </CardTitle>
                  <div className="flex items-center gap-2">
                    {indexFeedback && (
                      <span className="text-xs text-muted-foreground animate-in fade-in">
                        {indexFeedback}
                      </span>
                    )}
                    <TooltipProvider>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="size-7"
                            onClick={handleIndexFiles}
                            disabled={indexing}
                          >
                            <RefreshCw
                              className={cn(
                                "size-4",
                                indexing && "animate-spin"
                              )}
                            />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>
                          Re-scan directory for files
                        </TooltipContent>
                      </Tooltip>
                    </TooltipProvider>
                  </div>
                </CardHeader>
                <CardContent>
                  {/* I125: New files banner */}
                  {newFileCount > 0 && !bannerDismissed && (
                    <div className="mb-3 flex items-center gap-2 rounded-md bg-primary/10 px-3 py-2 text-sm">
                      <FileText className="size-4 shrink-0 text-primary" />
                      <span className="flex-1">
                        {newFileCount} new file{newFileCount !== 1 ? "s" : ""}{" "}
                        detected
                      </span>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 text-xs"
                        onClick={handleIndexFiles}
                        disabled={indexing}
                      >
                        Index now
                      </Button>
                      <button
                        onClick={() => setBannerDismissed(true)}
                        className="text-muted-foreground hover:text-foreground"
                      >
                        <X className="size-3.5" />
                      </button>
                    </div>
                  )}

                  {files.length > 0 ? (
                    <div className="space-y-1">
                      {files.map((f) => (
                        <div
                          key={f.id}
                          className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-muted cursor-default"
                          onClick={() =>
                            invoke("reveal_in_finder", {
                              path: f.absolutePath,
                            })
                          }
                        >
                          <File className="size-3.5 shrink-0 text-muted-foreground" />
                          <span className="flex-1 truncate">{f.filename}</span>
                          <span className="shrink-0 font-mono text-xs text-muted-foreground">
                            {formatFileSize(f.fileSize)}
                          </span>
                          <span className="shrink-0 text-xs text-muted-foreground">
                            {formatRelativeDateShort(f.modifiedAt)}
                          </span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <EmptyState icon={FileText} message="No files indexed" />
                  )}
                </CardContent>
              </Card>

              {/* Strategic Programs */}
              {detail.strategicPrograms.length > 0 && (
                <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Strategic Programs
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {detail.strategicPrograms.map((p, i) => (
                        <div
                          key={i}
                          className="flex items-center gap-2 text-sm"
                        >
                          <StatusBadge
                            value={p.status}
                            styles={progressStyles}
                            fallback={progressStyles.planned}
                          />
                          <span className="font-medium">{p.name}</span>
                          {p.notes && (
                            <span className="text-muted-foreground">
                              &mdash; {p.notes}
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}
            </div>

            {/* ═══ Sidebar Column ═══ */}
            <div className="space-y-6">
              {/* Account Details (read-first with edit toggle) */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Account Details
                  </CardTitle>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="size-7"
                    onClick={() =>
                      editing ? handleCancelEdit() : setEditing(true)
                    }
                  >
                    {editing ? (
                      <X className="size-4" />
                    ) : (
                      <Pencil className="size-4" />
                    )}
                  </Button>
                </CardHeader>
                <CardContent>
                  {editing ? (
                    <AccountDetailsEditForm
                      editHealth={editHealth}
                      setEditHealth={setEditHealth}
                      editLifecycle={editLifecycle}
                      setEditLifecycle={setEditLifecycle}
                      editArr={editArr}
                      setEditArr={setEditArr}
                      editNps={editNps}
                      setEditNps={setEditNps}
                      editCsm={editCsm}
                      setEditCsm={setEditCsm}
                      editChampion={editChampion}
                      setEditChampion={setEditChampion}
                      editRenewal={editRenewal}
                      setEditRenewal={setEditRenewal}
                      setDirty={setDirty}
                      onSave={handleSave}
                      onCancel={handleCancelEdit}
                      saving={saving}
                      dirty={dirty}
                    />
                  ) : (
                    <AccountDetailsReadView detail={detail} />
                  )}
                </CardContent>
              </Card>

              {/* Notes */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Notes
                  </CardTitle>
                  {dirty && (
                    <Button
                      size="sm"
                      onClick={handleSave}
                      disabled={saving}
                      className="h-7 text-xs"
                    >
                      <Save className="mr-1 size-3" />
                      {saving ? "Saving..." : "Save"}
                    </Button>
                  )}
                </CardHeader>
                <CardContent>
                  <textarea
                    value={editNotes}
                    onChange={(e) => {
                      setEditNotes(e.target.value);
                      setDirty(true);
                    }}
                    placeholder="Notes about this account..."
                    rows={4}
                    className="w-full resize-none rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  />
                </CardContent>
              </Card>

              {/* Stakeholder Map */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Stakeholder Map
                    {detail.linkedPeople.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({detail.linkedPeople.length})
                      </span>
                    )}
                  </CardTitle>
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
                    <EmptyState
                      icon={Users}
                      message="No people linked yet"
                    />
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

// ─── Sub-components ─────────────────────────────────────────────────────────

function EmptyState({
  icon: Icon,
  message,
}: {
  icon: React.ElementType;
  message: string;
}) {
  return (
    <div className="flex flex-col items-center py-6 text-center">
      <Icon className="mb-2 size-8 text-muted-foreground/40" />
      <p className="text-sm text-muted-foreground">{message}</p>
    </div>
  );
}

function AccountDetailsReadView({ detail }: { detail: AccountDetail }) {
  const healthDotStyles: Record<string, string> = {
    green: "bg-green-500",
    yellow: "bg-yellow-500",
    red: "bg-destructive",
  };

  const fields: { label: string; value: React.ReactNode }[] = [];

  if (detail.health) {
    fields.push({
      label: "Health",
      value: (
        <span className="flex items-center gap-2">
          <span
            className={cn(
              "inline-block size-2.5 rounded-full",
              healthDotStyles[detail.health] ?? "bg-muted-foreground"
            )}
          />
          <span className="capitalize">{detail.health}</span>
        </span>
      ),
    });
  }

  if (detail.lifecycle) {
    fields.push({
      label: "Lifecycle",
      value: <span className="capitalize">{detail.lifecycle}</span>,
    });
  }

  if (detail.arr != null) {
    fields.push({ label: "ARR", value: `$${formatArr(detail.arr)}` });
  }

  if (detail.nps != null) {
    fields.push({ label: "NPS", value: String(detail.nps) });
  }

  if (detail.csm) {
    fields.push({ label: "CSM", value: detail.csm });
  }

  if (detail.champion) {
    fields.push({ label: "Champion", value: detail.champion });
  }

  if (detail.renewalDate) {
    fields.push({
      label: "Renewal",
      value: formatDate(detail.renewalDate),
    });
  }

  if (detail.contractStart) {
    fields.push({
      label: "Contract Start",
      value: formatDate(detail.contractStart),
    });
  }

  if (fields.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No details set. Click the pencil icon to add account information.
      </p>
    );
  }

  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2.5 text-sm">
      {fields.map((f) => (
        <div key={f.label} className="contents">
          <dt className="text-muted-foreground">{f.label}</dt>
          <dd>{f.value}</dd>
        </div>
      ))}
    </dl>
  );
}

function AccountDetailsEditForm({
  editHealth,
  setEditHealth,
  editLifecycle,
  setEditLifecycle,
  editArr,
  setEditArr,
  editNps,
  setEditNps,
  editCsm,
  setEditCsm,
  editChampion,
  setEditChampion,
  editRenewal,
  setEditRenewal,
  setDirty,
  onSave,
  onCancel,
  saving,
  dirty,
}: {
  editHealth: string;
  setEditHealth: (v: string) => void;
  editLifecycle: string;
  setEditLifecycle: (v: string) => void;
  editArr: string;
  setEditArr: (v: string) => void;
  editNps: string;
  setEditNps: (v: string) => void;
  editCsm: string;
  setEditCsm: (v: string) => void;
  editChampion: string;
  setEditChampion: (v: string) => void;
  editRenewal: string;
  setEditRenewal: (v: string) => void;
  setDirty: (v: boolean) => void;
  onSave: () => void;
  onCancel: () => void;
  saving: boolean;
  dirty: boolean;
}) {
  const inputClass =
    "mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring";

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="text-xs font-medium text-muted-foreground">
            Health
          </label>
          <select
            value={editHealth}
            onChange={(e) => {
              setEditHealth(e.target.value);
              setDirty(true);
            }}
            className={inputClass}
          >
            <option value="">Not set</option>
            {healthOptions.map((h) => (
              <option key={h} value={h}>
                {h}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">
            Lifecycle
          </label>
          <select
            value={editLifecycle}
            onChange={(e) => {
              setEditLifecycle(e.target.value);
              setDirty(true);
            }}
            className={inputClass}
          >
            <option value="">Not set</option>
            {[
              "onboarding",
              "ramping",
              "steady-state",
              "at-risk",
              "churned",
            ].map((s) => (
              <option key={s} value={s} className="capitalize">
                {s}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">
            ARR
          </label>
          <input
            type="number"
            value={editArr}
            onChange={(e) => {
              setEditArr(e.target.value);
              setDirty(true);
            }}
            placeholder="Annual revenue"
            className={inputClass}
          />
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">
            NPS
          </label>
          <input
            type="number"
            value={editNps}
            onChange={(e) => {
              setEditNps(e.target.value);
              setDirty(true);
            }}
            placeholder="NPS score"
            className={inputClass}
          />
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">
            CSM
          </label>
          <input
            type="text"
            value={editCsm}
            onChange={(e) => {
              setEditCsm(e.target.value);
              setDirty(true);
            }}
            placeholder="CSM name"
            className={inputClass}
          />
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">
            Champion
          </label>
          <input
            type="text"
            value={editChampion}
            onChange={(e) => {
              setEditChampion(e.target.value);
              setDirty(true);
            }}
            placeholder="Champion name"
            className={inputClass}
          />
        </div>
        <div className="col-span-2">
          <label className="text-xs font-medium text-muted-foreground">
            Renewal Date
          </label>
          <input
            type="date"
            value={editRenewal}
            onChange={(e) => {
              setEditRenewal(e.target.value);
              setDirty(true);
            }}
            className={inputClass}
          />
        </div>
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button size="sm" onClick={onSave} disabled={saving || !dirty}>
          <Save className="mr-1 size-4" />
          {saving ? "Saving..." : "Save"}
        </Button>
      </div>
    </div>
  );
}

function PortfolioRow({ aggregate }: { aggregate: ParentAggregate }) {
  const healthDot: Record<string, string> = {
    green: "bg-green-500",
    yellow: "bg-yellow-500",
    red: "bg-destructive",
  };

  const items: { label: string; value: string }[] = [
    { label: "Business Units", value: String(aggregate.buCount) },
  ];
  if (aggregate.totalArr != null) {
    items.push({ label: "Total ARR", value: `$${formatArr(aggregate.totalArr)}` });
  }
  if (aggregate.worstHealth) {
    items.push({ label: "Worst Health", value: aggregate.worstHealth });
  }
  if (aggregate.nearestRenewal) {
    items.push({
      label: "Nearest Renewal",
      value: formatRenewalCountdown(aggregate.nearestRenewal),
    });
  }

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
      {items.map((item) => (
        <div
          key={item.label}
          className="rounded-lg border border-dashed bg-muted/30 px-4 py-3"
        >
          <div className="flex items-center gap-2">
            {item.label === "Worst Health" && aggregate.worstHealth && (
              <span
                className={cn(
                  "size-2.5 rounded-full",
                  healthDot[aggregate.worstHealth] ?? "bg-muted-foreground/30"
                )}
              />
            )}
            <span className="text-xl font-semibold capitalize">
              {item.value}
            </span>
          </div>
          <div className="text-xs text-muted-foreground">{item.label}</div>
        </div>
      ))}
    </div>
  );
}

function ChildAccountRow({ child }: { child: AccountChildSummary }) {
  const healthDot: Record<string, string> = {
    green: "bg-green-500",
    yellow: "bg-yellow-500",
    red: "bg-destructive",
  };

  return (
    <Link
      to="/accounts/$accountId"
      params={{ accountId: child.id }}
      className="flex items-center gap-3 rounded-lg border px-4 py-3 transition-colors hover:bg-muted"
    >
      {child.health && (
        <span
          className={cn(
            "size-2.5 shrink-0 rounded-full",
            healthDot[child.health] ?? "bg-muted-foreground/30"
          )}
        />
      )}
      <span className="flex-1 truncate font-medium">{child.name}</span>
      {child.arr != null && (
        <span className="shrink-0 font-mono text-sm text-muted-foreground">
          ${formatArr(child.arr)}
        </span>
      )}
      {child.openActionCount > 0 && (
        <Badge variant="outline" className="shrink-0 text-xs">
          {child.openActionCount} action{child.openActionCount !== 1 ? "s" : ""}
        </Badge>
      )}
    </Link>
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

// ─── Formatters ─────────────────────────────────────────────────────────────

function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr.split("T")[0] ?? dateStr;
  }
}


function formatRelativeDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = date.getTime() - now.getTime();
    const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) {
      return date.toLocaleTimeString(undefined, {
        hour: "numeric",
        minute: "2-digit",
      });
    }
    if (diffDays === 1) {
      return `Tomorrow ${date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
    }
    if (diffDays === -1) return "Yesterday";
    if (diffDays < -1) return `${Math.abs(diffDays)} days ago`;
    if (diffDays <= 7) return `In ${diffDays} days`;
    return formatDate(dateStr);
  } catch {
    return dateStr.split("T")[0] ?? dateStr;
  }
}

function formatMeetingType(meetingType: string): string {
  const labels: Record<string, string> = {
    customer: "Customer",
    qbr: "QBR",
    training: "Training",
    internal: "Internal",
    team_sync: "Team Sync",
    one_on_one: "1:1",
    partnership: "Partner",
    all_hands: "All Hands",
    external: "External",
    personal: "Personal",
  };
  return labels[meetingType] ?? meetingType;
}

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round(
      (renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
    );
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    if (diffDays <= 90) return `${diffDays} days`;
    return renewal.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

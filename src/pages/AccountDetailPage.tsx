import { useState, useEffect, useCallback, useRef } from "react";
import { useParams, Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  StatusBadge,
  healthStyles,
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { PageError } from "@/components/PageState";
import { cn, formatArr, formatFileSize, formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  AlertTriangle,
  Archive,
  ArrowLeft,
  CalendarClock,
  CheckCircle2,
  ChevronDown,
  ExternalLink,
  File,
  FileText,
  Loader2,
  Pencil,
  RefreshCw,
  Save,
  Plus,
  Sparkles,
  Trophy,
  Users,
  X,
  HelpCircle,
  Target,
} from "lucide-react";
import { Input } from "@/components/ui/input";
import type {
  AccountDetail,
  AccountEvent,
  AccountHealth,
  AccountChildSummary,
  ContentFile,
  IntelRisk,
  IntelWin,
  MeetingPreview,
  StakeholderInsight,
  StrategicProgram,
  ParentAggregate,
} from "@/types";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

const healthOptions: AccountHealth[] = ["green", "yellow", "red"];

const temperatureStyles: Record<string, string> = {
  hot: "bg-destructive/15 text-destructive",
  warm: "bg-primary/15 text-primary",
  cool: "bg-muted text-muted-foreground",
  cold: "bg-muted text-muted-foreground/60",
};

/** Render inline text with [N] citation markers as superscript. */
function renderCitations(text: string): ReactNode[] {
  return text.split(/(\[\d+\])/).map((part, i) => {
    if (/^\[\d+\]$/.test(part)) {
      return (
        <sup key={i} className="text-muted-foreground text-[0.65em] ml-px font-medium">
          {part}
        </sup>
      );
    }
    return <span key={i}>{part}</span>;
  });
}

/** Render an executive assessment with paragraph spacing, superscript citations, and a sources footer. */
function AssessmentBody({ text }: { text: string }) {
  // Split body from SOURCES: block
  const sourcesMatch = text.match(/\nSOURCES:\s*\n/i);
  const body = sourcesMatch ? text.slice(0, sourcesMatch.index!) : text;
  const sourcesRaw = sourcesMatch
    ? text.slice(sourcesMatch.index! + sourcesMatch[0].length).trim()
    : null;

  const paragraphs = body.split(/\n+/).filter((p) => p.trim());

  return (
    <>
      <div className="flex flex-col gap-6">
        {paragraphs.map((para, i) => (
          <p key={i} className="text-sm leading-[1.75] text-foreground m-0">
            {renderCitations(para.trim())}
          </p>
        ))}
      </div>
      {sourcesRaw && (() => {
        const sourceLines = sourcesRaw.split("\n").filter((l) => l.trim());
        const visible = sourceLines.slice(0, 2);
        const overflow = sourceLines.slice(2);
        return (
          <div className="text-[11px] text-muted-foreground/70 border-t border-border/40 pt-3 mt-4 space-y-0.5">
            {visible.map((line, i) => (
              <p key={i} className="m-0">{line.trim()}</p>
            ))}
            {overflow.length > 0 && (
              <Collapsible>
                <CollapsibleContent>
                  <div className="space-y-0.5">
                    {overflow.map((line, i) => (
                      <p key={i + 2} className="m-0">{line.trim()}</p>
                    ))}
                  </div>
                </CollapsibleContent>
                <CollapsibleTrigger asChild>
                  <button className="text-muted-foreground/50 hover:text-muted-foreground transition-colors mt-1">
                    +{overflow.length} more sources
                  </button>
                </CollapsibleTrigger>
              </Collapsible>
            )}
          </div>
        );
      })()}
    </>
  );
}

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [detail, setDetail] = useState<AccountDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);

  // Editable structured fields
  const [editName, setEditName] = useState<string>("");
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
  const [enrichSeconds, setEnrichSeconds] = useState(0);

  // I127: Inline action creation
  const [addingAction, setAddingAction] = useState(false);
  const [newActionTitle, setNewActionTitle] = useState("");
  const [creatingAction, setCreatingAction] = useState(false);
  const [createChildOpen, setCreateChildOpen] = useState(false);
  const [childName, setChildName] = useState("");
  const [childDescription, setChildDescription] = useState("");
  const [childOwnerId, setChildOwnerId] = useState("");
  const [creatingChild, setCreatingChild] = useState(false);

  // I124: Content index state
  const [files, setFiles] = useState<ContentFile[]>([]);
  const [indexing, setIndexing] = useState(false);
  const [newFileCount, setNewFileCount] = useState(0);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [indexFeedback, setIndexFeedback] = useState<string | null>(null);

  // Evidence section collapse state
  const [recentMeetingsExpanded, setRecentMeetingsExpanded] = useState(false);

  // I163: Strategic programs inline editing
  const [programs, setPrograms] = useState<StrategicProgram[]>([]);
  const programsSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // I143: Lifecycle events
  const [events, setEvents] = useState<AccountEvent[]>([]);
  const [showEventForm, setShowEventForm] = useState(false);
  const [newEventType, setNewEventType] = useState("renewal");
  const [newEventDate, setNewEventDate] = useState("");
  const [newArrImpact, setNewArrImpact] = useState("");
  const [newEventNotes, setNewEventNotes] = useState("");

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (programsSaveTimer.current) clearTimeout(programsSaveTimer.current);
    };
  }, []);

  const intelligence = detail?.intelligence ?? null;

  const load = useCallback(async () => {
    if (!accountId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AccountDetail>("get_account_detail", {
        accountId,
      });
      setDetail(result);
      setEditName(result.name);
      setEditHealth(result.health ?? "");
      setEditLifecycle(result.lifecycle ?? "");
      setEditArr(result.arr?.toString() ?? "");
      setEditNps(result.nps?.toString() ?? "");
      setEditCsm(result.csm ?? "");
      setEditChampion(result.champion ?? "");
      setEditRenewal(result.renewalDate ?? "");
      setEditNotes(result.notes ?? "");
      setPrograms(result.strategicPrograms);
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
      // I143: Load lifecycle events
      try {
        const accountEvents = await invoke<AccountEvent[]>("get_account_events", {
          accountId,
        });
        setEvents(accountEvents);
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

  // Listen for intelligence-updated events
  useEffect(() => {
    const unlisten = listen<{ entityId: string }>(
      "intelligence-updated",
      (event) => {
        if (accountId && event.payload.entityId === accountId) {
          load();
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [accountId, load]);

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

  // Timer for enrichment progress
  useEffect(() => {
    if (!enriching) {
      setEnrichSeconds(0);
      return;
    }
    const interval = setInterval(() => {
      setEnrichSeconds((s) => s + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [enriching]);

  async function handleSave() {
    if (!detail) return;
    setSaving(true);

    try {
      const fieldUpdates: [string, string][] = [];
      if (editName !== detail.name)
        fieldUpdates.push(["name", editName]);
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
    setEditName(detail.name);
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

  async function handleCreateChild() {
    if (!detail || !childName.trim()) return;
    setCreatingChild(true);
    try {
      const result = await invoke<{ id: string }>("create_child_account", {
        parentId: detail.id,
        name: childName.trim(),
        description: childDescription.trim() || null,
        ownerPersonId: childOwnerId || null,
      });
      setCreateChildOpen(false);
      setChildName("");
      setChildDescription("");
      setChildOwnerId("");
      await load();
      navigate({ to: "/accounts/$accountId", params: { accountId: result.id } });
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingChild(false);
    }
  }

  // I163: Debounced save for strategic programs
  const savePrograms = useCallback(
    async (updated: StrategicProgram[]) => {
      if (!detail) return;
      if (programsSaveTimer.current) clearTimeout(programsSaveTimer.current);
      programsSaveTimer.current = setTimeout(async () => {
        try {
          await invoke("update_account_programs", {
            accountId: detail.id,
            programsJson: JSON.stringify(updated),
          });
        } catch (e) {
          console.error("Failed to save programs:", e);
        }
      }, 400);
    },
    [detail]
  );

  function handleProgramUpdate(index: number, updated: StrategicProgram) {
    const next = [...programs];
    next[index] = updated;
    setPrograms(next);
    savePrograms(next);
  }

  function handleProgramDelete(index: number) {
    const next = programs.filter((_, i) => i !== index);
    setPrograms(next);
    savePrograms(next);
  }

  function handleAddProgram() {
    const next = [...programs, { name: "", status: "Active", notes: "" }];
    setPrograms(next);
    // Don't save yet — let the user fill in the name first
  }

  // I143: Record a lifecycle event
  async function handleRecordEvent() {
    if (!detail || !newEventDate) return;
    try {
      await invoke("record_account_event", {
        accountId: detail.id,
        eventType: newEventType,
        eventDate: newEventDate,
        arrImpact: newArrImpact ? parseFloat(newArrImpact) : null,
        notes: newEventNotes || null,
      });
      const updated = await invoke<AccountEvent[]>("get_account_events", {
        accountId: detail.id,
      });
      setEvents(updated);
      setShowEventForm(false);
      setNewEventType("renewal");
      setNewEventDate("");
      setNewArrImpact("");
      setNewEventNotes("");
    } catch (err) {
      console.error("Failed to record event:", err);
    }
  }

  async function handleArchive() {
    if (!detail) return;
    try {
      await invoke("archive_account", { id: detail.id, archived: true });
      navigate({ to: "/accounts" });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnarchive() {
    if (!detail) return;
    try {
      await invoke("archive_account", { id: detail.id, archived: false });
      await load();
    } catch (e) {
      setError(String(e));
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

          {/* Archived Banner */}
          {detail.archived && (
            <div className="rounded-lg border border-primary/30 bg-primary/5 px-4 py-3 flex items-center justify-between">
              <span className="text-sm text-charcoal/70">This account is archived and hidden from active views.</span>
              <Button variant="outline" size="sm" onClick={handleUnarchive}>
                Unarchive
              </Button>
            </div>
          )}

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
                  {detail.isInternal && (
                    <Badge variant="outline" className="text-xs border-primary/30 text-primary">
                      Internal
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
              {!detail.archived && (
                <Dialog open={createChildOpen} onOpenChange={setCreateChildOpen}>
                  <DialogTrigger asChild>
                    <Button variant="outline" size="sm" className="h-8 text-xs">
                      <Plus className="mr-1 size-3" />
                      {detail.isInternal ? "New Team" : "New BU"}
                    </Button>
                  </DialogTrigger>
                  <DialogContent className="sm:max-w-md">
                    <DialogHeader>
                      <DialogTitle>{detail.isInternal ? "Create Team" : "Create Business Unit"}</DialogTitle>
                      <DialogDescription>
                        Add a child entity under {detail.name}.
                      </DialogDescription>
                    </DialogHeader>
                    <div className="space-y-3">
                      <div className="space-y-1">
                        <label className="text-xs text-muted-foreground">Name</label>
                        <Input
                          value={childName}
                          onChange={(e) => setChildName(e.target.value)}
                          placeholder={detail.isInternal ? "Team name" : "BU name"}
                        />
                        <div className="flex flex-wrap gap-1 pt-1">
                          {(detail.isInternal
                            ? ["Leadership Team", "Product Team", "Operations Team"]
                            : ["Enterprise", "SMB", "Strategic"])
                            .map((suggestion) => (
                              <button
                                key={suggestion}
                                type="button"
                                onClick={() => setChildName(suggestion)}
                                className="rounded-full border px-2 py-0.5 text-[10px] text-muted-foreground hover:text-foreground"
                              >
                                {suggestion}
                              </button>
                            ))}
                        </div>
                      </div>
                      <div className="space-y-1">
                        <label className="text-xs text-muted-foreground">Description</label>
                        <textarea
                          value={childDescription}
                          onChange={(e) => setChildDescription(e.target.value)}
                          className="min-h-[80px] w-full rounded-md border bg-background px-3 py-2 text-sm"
                          placeholder="Optional notes for this team or business unit"
                        />
                      </div>
                      <div className="space-y-1">
                        <label className="text-xs text-muted-foreground">Owner</label>
                        <select
                          value={childOwnerId}
                          onChange={(e) => setChildOwnerId(e.target.value)}
                          className="h-9 w-full rounded-md border bg-background px-2 text-sm"
                        >
                          <option value="">No owner</option>
                          {detail.linkedPeople.map((person) => (
                            <option key={person.id} value={person.id}>
                              {person.name || person.email}
                            </option>
                          ))}
                        </select>
                      </div>
                    </div>
                    <div className="flex justify-end gap-2">
                      <Button variant="ghost" onClick={() => setCreateChildOpen(false)}>
                        Cancel
                      </Button>
                      <Button onClick={handleCreateChild} disabled={creatingChild || !childName.trim()}>
                        {creatingChild ? "Creating..." : "Create"}
                      </Button>
                    </div>
                  </DialogContent>
                </Dialog>
              )}
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
                        ? `Building... ${enrichSeconds}s`
                        : intelligence
                          ? "Refresh Intelligence"
                          : "Build Intelligence"}
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    {intelligence
                      ? `Re-synthesize from ${files.length} file${files.length !== 1 ? "s" : ""}, meetings, and captures`
                      : "Synthesize intelligence from workspace files, meetings, and web search"}
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
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
                      <AlertDialogTitle>Archive Account</AlertDialogTitle>
                      <AlertDialogDescription>
                        {detail.isParent
                          ? `Archive "${detail.name}" and its ${detail.childCount} business unit${detail.childCount !== 1 ? "s" : ""}? They will be hidden from active views.`
                          : `Archive "${detail.name}"? It will be hidden from active views.`}
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

          {/* ═══ Executive Assessment (intelligence hero) ═══ */}
          {intelligence?.executiveAssessment ? (
            <div className="space-y-1">
              <AssessmentBody text={intelligence.executiveAssessment} />
              <p className="text-xs text-muted-foreground">
                Intelligence last updated{" "}
                {formatRelativeDateShort(intelligence.enrichedAt)}
                {intelligence.sourceFileCount > 0 &&
                  ` from ${intelligence.sourceFileCount} file${intelligence.sourceFileCount !== 1 ? "s" : ""}`}
              </p>
            </div>
          ) : (
            !intelligence && (
              <div className="rounded-lg border border-dashed bg-muted/30 px-6 py-8 text-center">
                <Sparkles className="mx-auto mb-3 size-8 text-muted-foreground/40" />
                <p className="mb-1 text-sm font-medium text-muted-foreground">
                  No intelligence yet
                </p>
                <p className="mb-4 text-xs text-muted-foreground/80">
                  Build intelligence to synthesize your workspace files,
                  meetings, and captures into an actionable assessment.
                </p>
                <Button
                  size="sm"
                  onClick={handleEnrich}
                  disabled={enriching}
                >
                  {enriching ? (
                    <Loader2 className="mr-1 size-3 animate-spin" />
                  ) : (
                    <Sparkles className="mr-1 size-3" />
                  )}
                  {enriching ? `Building... ${enrichSeconds}s` : "Build Intelligence"}
                </Button>
              </div>
            )
          )}

          {/* Asymmetric Grid: Main (3fr) + Sidebar (2fr) */}
          <div className="grid gap-6 lg:grid-cols-[3fr_2fr]">
            {/* ═══ Main Column ═══ */}
            <div className="min-w-0 space-y-6">
              {/* Strategic Attention: Risks + Wins + Unknowns (capped at 3, expandable) */}
              {intelligence && (() => {
                const risks = intelligence.risks ?? [];
                const wins = intelligence.recentWins ?? [];
                const unknowns = intelligence.currentState?.unknowns ?? [];
                const allItems = [
                  ...risks.map((r, i) => ({ type: "risk" as const, key: `risk-${i}`, item: r })),
                  ...wins.map((w, i) => ({ type: "win" as const, key: `win-${i}`, item: w })),
                  ...unknowns.map((u, i) => ({ type: "unknown" as const, key: `unknown-${i}`, item: u })),
                ];
                if (allItems.length === 0) return null;
                const visible = allItems.slice(0, 3);
                const overflow = allItems.slice(3);
                return (
                  <Card>
                    <CardHeader className="pb-3">
                      <CardTitle className="text-base font-semibold">
                        Strategic Attention
                        <span className="ml-1 text-muted-foreground">
                          ({allItems.length})
                        </span>
                      </CardTitle>
                      <p className="text-xs text-muted-foreground">
                        Synthesized from intelligence, meetings, and signals
                      </p>
                    </CardHeader>
                    <CardContent>
                      <Collapsible>
                        <div className="space-y-3">
                          {visible.map((entry) =>
                            entry.type === "risk" ? (
                              <AttentionRisk key={entry.key} risk={entry.item as IntelRisk} />
                            ) : entry.type === "win" ? (
                              <AttentionWin key={entry.key} win={entry.item as IntelWin} />
                            ) : (
                              <div
                                key={entry.key}
                                className="flex items-start gap-3 rounded-lg bg-primary/8 px-4 py-3"
                              >
                                <HelpCircle className="mt-0.5 size-4 shrink-0 text-primary" />
                                <span className="text-sm">{entry.item as string}</span>
                              </div>
                            )
                          )}
                        </div>
                        {overflow.length > 0 && (
                          <>
                            <CollapsibleContent>
                              <div className="space-y-3 mt-3 pt-3 border-t">
                                {overflow.map((entry) =>
                                  entry.type === "risk" ? (
                                    <AttentionRisk key={entry.key} risk={entry.item as IntelRisk} />
                                  ) : entry.type === "win" ? (
                                    <AttentionWin key={entry.key} win={entry.item as IntelWin} />
                                  ) : (
                                    <div
                                      key={entry.key}
                                      className="flex items-start gap-3 rounded-lg bg-primary/8 px-4 py-3"
                                    >
                                      <HelpCircle className="mt-0.5 size-4 shrink-0 text-primary" />
                                      <span className="text-sm">{entry.item as string}</span>
                                    </div>
                                  )
                                )}
                              </div>
                            </CollapsibleContent>
                            <CollapsibleTrigger asChild>
                              <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors mt-3 pt-3 border-t w-full justify-center">
                                <ChevronDown className="size-4" />
                                Show {overflow.length} more items
                              </button>
                            </CollapsibleTrigger>
                          </>
                        )}
                      </Collapsible>
                    </CardContent>
                  </Card>
                );
              })()}

              {/* Meeting Readiness */}
              {intelligence?.nextMeetingReadiness && intelligence.nextMeetingReadiness.prepItems.length > 0 && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      <Target className="mr-1.5 inline-block size-4" />
                      Next Meeting Readiness
                    </CardTitle>
                    {intelligence.nextMeetingReadiness.meetingTitle && (
                      <p className="text-sm text-muted-foreground">
                        {intelligence.nextMeetingReadiness.meetingTitle}
                        {intelligence.nextMeetingReadiness.meetingDate &&
                          ` — ${formatDate(intelligence.nextMeetingReadiness.meetingDate)}`}
                      </p>
                    )}
                  </CardHeader>
                  <CardContent>
                    <ul className="space-y-2">
                      {intelligence.nextMeetingReadiness.prepItems.map((item, i) => (
                        <li key={i} className="flex items-start gap-2 text-sm">
                          <span className="mt-1 size-1.5 shrink-0 rounded-full bg-primary" />
                          {item}
                        </li>
                      ))}
                    </ul>
                  </CardContent>
                </Card>
              )}

              {/* Upcoming Meetings */}
              <Card>
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

              {/* Recent Meetings — show 3, expand for all (ADR-0063) */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Recent Meetings
                    {detail.recentMeetings.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({detail.recentMeetings.length})
                      </span>
                    )}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {detail.recentMeetings.length > 0 ? (
                    <Collapsible open={recentMeetingsExpanded} onOpenChange={setRecentMeetingsExpanded}>
                      <div className="space-y-3">
                        {detail.recentMeetings.slice(0, 3).map((m) => (
                          <MeetingPreviewCard key={m.id} meeting={m} />
                        ))}
                      </div>
                      {detail.recentMeetings.length > 3 && (
                        <>
                          <CollapsibleContent>
                            <div className="space-y-3 mt-3 pt-3 border-t">
                              {detail.recentMeetings.slice(3).map((m) => (
                                <MeetingPreviewCard key={m.id} meeting={m} />
                              ))}
                            </div>
                          </CollapsibleContent>
                          <CollapsibleTrigger asChild>
                            <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors mt-3 pt-3 border-t w-full justify-center">
                              <ChevronDown className={cn("size-4 transition-transform", recentMeetingsExpanded && "rotate-180")} />
                              {recentMeetingsExpanded
                                ? "Show fewer"
                                : `Show ${detail.recentMeetings.length - 3} more meetings`}
                            </button>
                          </CollapsibleTrigger>
                        </>
                      )}
                    </Collapsible>
                  ) : (
                    <EmptyState
                      icon={CalendarClock}
                      message="No past meetings recorded"
                    />
                  )}
                </CardContent>
              </Card>

              {/* Open Actions (Commitments) */}
              <Card>
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Commitments
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
                                request: {
                                  title: newActionTitle.trim(),
                                  accountId: detail.id,
                                },
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

              {/* Stakeholder Intelligence */}
              {(intelligence?.stakeholderInsights?.length ?? 0) > 0 && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Stakeholder Intelligence
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-4">
                      {intelligence!.stakeholderInsights.map((s, i) => (
                        <StakeholderCard key={i} insight={s} linkedPeople={detail.linkedPeople} />
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Current State — What's working / Not working */}
              {intelligence?.currentState && (intelligence.currentState.working?.length || intelligence.currentState.notWorking?.length) && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Current State
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="grid gap-4 sm:grid-cols-2">
                      {intelligence.currentState.working?.length ? (
                        <div>
                          <h4 className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                            Working
                          </h4>
                          <ul className="space-y-1.5">
                            {intelligence.currentState.working.map((item, i) => (
                              <li
                                key={i}
                                className="flex items-start gap-2 text-sm"
                              >
                                <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-green-500" />
                                {item}
                              </li>
                            ))}
                          </ul>
                        </div>
                      ) : null}
                      {intelligence.currentState.notWorking?.length ? (
                        <div>
                          <h4 className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                            Not Working
                          </h4>
                          <ul className="space-y-1.5">
                            {intelligence.currentState.notWorking.map(
                              (item, i) => (
                                <li
                                  key={i}
                                  className="flex items-start gap-2 text-sm"
                                >
                                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-destructive" />
                                  {item}
                                </li>
                              )
                            )}
                          </ul>
                        </div>
                      ) : null}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Business Units (I114 — parent accounts only) */}
              {detail.children.length > 0 && (
                <Card>
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

              {/* Meeting Outcomes — captures from transcripts, capped at 3 */}
              {detail.recentCaptures.length > 0 && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Meeting Outcomes
                      <span className="ml-1 text-muted-foreground">
                        ({detail.recentCaptures.length})
                      </span>
                    </CardTitle>
                    <p className="text-xs text-muted-foreground">
                      Captured from recent meeting transcripts
                    </p>
                  </CardHeader>
                  <CardContent>
                    <Collapsible>
                      <div className="space-y-2">
                        {detail.recentCaptures.slice(0, 3).map((c) => (
                          <Link
                            key={c.id}
                            to="/meeting/$meetingId"
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
                      {detail.recentCaptures.length > 3 && (
                        <>
                          <CollapsibleContent>
                            <div className="space-y-2 mt-2 pt-2 border-t">
                              {detail.recentCaptures.slice(3).map((c) => (
                                <Link
                                  key={c.id}
                                  to="/meeting/$meetingId"
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
                          </CollapsibleContent>
                          <CollapsibleTrigger asChild>
                            <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors mt-2 pt-2 border-t w-full justify-center">
                              <ChevronDown className="size-4" />
                              Show {detail.recentCaptures.length - 3} more outcomes
                            </button>
                          </CollapsibleTrigger>
                        </>
                      )}
                    </Collapsible>
                  </CardContent>
                </Card>
              )}

              {detail.recentEmailSignals && detail.recentEmailSignals.length > 0 && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Email Timeline
                      <span className="ml-1 text-muted-foreground">
                        ({detail.recentEmailSignals.length})
                      </span>
                    </CardTitle>
                    <p className="text-xs text-muted-foreground">
                      Signals extracted from recent inbound email.
                    </p>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {detail.recentEmailSignals.slice(0, 8).map((signal, idx) => (
                        <div
                          key={`${signal.id ?? idx}-${signal.signalType}`}
                          className="rounded-md border border-border/70 bg-card/50 px-3 py-2"
                        >
                          <div className="flex items-center justify-between gap-2">
                            <Badge variant="outline" className="text-[10px] uppercase tracking-wide">
                              {signal.signalType}
                            </Badge>
                            <span className="text-[10px] text-muted-foreground">
                              {signal.detectedAt
                                ? formatRelativeDateShort(signal.detectedAt)
                                : ""}
                            </span>
                          </div>
                          <p className="mt-1 text-sm leading-relaxed">{signal.signalText}</p>
                          <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
                            {signal.urgency && <span>Urgency: {signal.urgency}</span>}
                            {signal.sentiment && <span>Sentiment: {signal.sentiment}</span>}
                            {signal.confidence != null && (
                              <span>Confidence: {Math.round(signal.confidence * 100)}%</span>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              )}

              {/* Value Delivered (from intelligence) */}
              {(intelligence?.valueDelivered?.length ?? 0) > 0 && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Value Delivered
                      <span className="ml-1 text-muted-foreground">
                        ({intelligence!.valueDelivered.length})
                      </span>
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="space-y-2">
                      {intelligence!.valueDelivered.map((v, i) => (
                        <div
                          key={i}
                          className="flex items-start gap-2 text-sm"
                        >
                          {v.date && (
                            <span className="shrink-0 font-mono text-xs text-muted-foreground">
                              {v.date}
                            </span>
                          )}
                          <span>{v.statement}</span>
                          {v.source && (
                            <span className="shrink-0 text-xs text-muted-foreground">
                              ({v.source})
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
            <div className="min-w-0 space-y-6">
              {/* Account Details (read-first with edit toggle) */}
              <Card>
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
                      editName={editName}
                      setEditName={setEditName}
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
              <Card>
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

              {/* I163: Strategic Programs (inline editable) */}
              <Card>
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Strategic Programs
                    {programs.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({programs.length})
                      </span>
                    )}
                  </CardTitle>
                  <button
                    onClick={handleAddProgram}
                    className="inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                  >
                    <Plus className="size-3" />
                    Add
                  </button>
                </CardHeader>
                <CardContent>
                  {programs.length > 0 ? (
                    <div className="space-y-1">
                      {programs.map((p, i) => (
                        <ProgramRow
                          key={i}
                          program={p}
                          autoFocusName={!p.name}
                          onUpdate={(updated) => handleProgramUpdate(i, updated)}
                          onDelete={() => handleProgramDelete(i)}
                        />
                      ))}
                    </div>
                  ) : (
                    <div className="text-sm text-muted-foreground space-y-2">
                      <p className="font-medium">Track strategic initiatives that cross multiple deals or drive account-level outcomes.</p>
                      <p className="text-xs">Examples: QBR prep, EBR planning, executive alignment campaigns, product adoption programs, expansion plays, renewal strategy</p>
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* I143: Lifecycle Events */}
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base font-semibold">
                      Lifecycle Events
                      {events.length > 0 && (
                        <span className="ml-1 text-muted-foreground">
                          ({events.length})
                        </span>
                      )}
                    </CardTitle>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 text-xs"
                      onClick={() => setShowEventForm(true)}
                    >
                      + Record
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  {showEventForm && (
                    <div className="space-y-3 border rounded-lg p-3 mb-3">
                      <select
                        value={newEventType}
                        onChange={(e) => setNewEventType(e.target.value)}
                        className="h-8 w-full rounded border bg-background px-2 text-sm"
                      >
                        <option value="renewal">Renewal</option>
                        <option value="expansion">Expansion</option>
                        <option value="churn">Churn</option>
                        <option value="downgrade">Downgrade</option>
                      </select>
                      <Input
                        type="date"
                        value={newEventDate}
                        onChange={(e) => setNewEventDate(e.target.value)}
                        className="h-8"
                      />
                      <Input
                        type="number"
                        placeholder="ARR impact"
                        value={newArrImpact}
                        onChange={(e) => setNewArrImpact(e.target.value)}
                        className="h-8"
                      />
                      <Input
                        placeholder="Notes (optional)"
                        value={newEventNotes}
                        onChange={(e) => setNewEventNotes(e.target.value)}
                        className="h-8"
                      />
                      <div className="flex gap-2">
                        <Button size="sm" onClick={handleRecordEvent} disabled={!newEventDate}>
                          Save
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => setShowEventForm(false)}
                        >
                          Cancel
                        </Button>
                      </div>
                    </div>
                  )}
                  {events.length === 0 ? (
                    <p className="text-sm text-muted-foreground">
                      No lifecycle events recorded
                    </p>
                  ) : (
                    <div className="space-y-2">
                      {events.map((event) => (
                        <div
                          key={event.id}
                          className="flex items-center justify-between py-1.5 text-sm"
                        >
                          <div className="flex items-center gap-2">
                            <Badge
                              variant="outline"
                              className={eventBadgeClass(event.eventType)}
                            >
                              {event.eventType}
                            </Badge>
                            <span className="text-muted-foreground">
                              {formatDate(event.eventDate)}
                            </span>
                          </div>
                          {event.arrImpact != null && (
                            <span
                              className={cn(
                                "font-mono text-xs",
                                event.arrImpact >= 0
                                  ? "text-green-600"
                                  : "text-destructive"
                              )}
                            >
                              {event.arrImpact >= 0 ? "+" : ""}
                              {formatCurrency(event.arrImpact)}
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* Company Context (demoted from main, from intelligence) */}
              {(intelligence?.companyContext || detail.companyOverview) && (
                <Card>
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Company Context
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-2 text-sm">
                    {intelligence?.companyContext ? (
                      <>
                        {intelligence.companyContext.description && (
                          <p>{intelligence.companyContext.description}</p>
                        )}
                        {intelligence.companyContext.additionalContext && (
                          <p className="text-muted-foreground">
                            {intelligence.companyContext.additionalContext}
                          </p>
                        )}
                        <div className="flex flex-wrap gap-x-4 gap-y-1 text-muted-foreground">
                          {intelligence.companyContext.industry && (
                            <span>
                              Industry: {intelligence.companyContext.industry}
                            </span>
                          )}
                          {intelligence.companyContext.size && (
                            <span>
                              Size: {intelligence.companyContext.size}
                            </span>
                          )}
                          {intelligence.companyContext.headquarters && (
                            <span>
                              HQ: {intelligence.companyContext.headquarters}
                            </span>
                          )}
                        </div>
                      </>
                    ) : detail.companyOverview ? (
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
                    ) : null}
                  </CardContent>
                </Card>
              )}

              {/* Files (moved from Evidence section to sidebar) */}
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
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
                              className="size-6"
                              onClick={handleIndexFiles}
                              disabled={indexing}
                            >
                              <RefreshCw
                                className={cn(
                                  "size-3",
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
                  </div>
                </CardHeader>
                <CardContent>
                  {/* I125: New files banner */}
                  {newFileCount > 0 && !bannerDismissed && (
                    <div className="mb-3 flex items-center gap-2 rounded-md bg-primary/10 px-3 py-2 text-sm">
                      <FileText className="size-4 shrink-0 text-primary" />
                      <span className="flex-1">
                        {newFileCount} new file
                        {newFileCount !== 1 ? "s" : ""} detected
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
                    <Collapsible>
                      <div className="space-y-1">
                        {files.slice(0, 10).map((f) => (
                          <FileRow key={f.id} file={f} />
                        ))}
                      </div>
                      {files.length > 10 && (
                        <>
                          <CollapsibleContent>
                            <div className="space-y-1 mt-1">
                              {files.slice(10).map((f) => (
                                <FileRow key={f.id} file={f} />
                              ))}
                            </div>
                          </CollapsibleContent>
                          <CollapsibleTrigger asChild>
                            <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors mt-2 pt-2 border-t w-full justify-center">
                              <ChevronDown className="size-4" />
                              Show {files.length - 10} more files
                            </button>
                          </CollapsibleTrigger>
                        </>
                      )}
                    </Collapsible>
                  ) : (
                    <EmptyState
                      icon={FileText}
                      message="No files indexed"
                    />
                  )}
                </CardContent>
              </Card>

              {/* Stakeholder Map (when no intelligence stakeholders) */}
              {(!intelligence?.stakeholderInsights?.length) && (
                <Card>
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
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

// ─── Meeting Preview Card (ADR-0063) ─────────────────────────────────────────

function MeetingPreviewCard({ meeting }: { meeting: MeetingPreview }) {
  const prep = meeting.prepContext;
  const riskCount = prep?.entityRisks?.length ?? 0;
  const actionCount = prep?.openItems?.length ?? 0;
  const questionCount = prep?.questions?.length ?? 0;

  return (
    <Link
      to="/meeting/$meetingId"
      params={{ meetingId: meeting.id }}
      className="block rounded-lg border transition-colors hover:bg-muted"
    >
      {/* Header row */}
      <div className="flex items-center gap-3 px-4 py-3">
        <Badge variant="outline" className="shrink-0 text-xs">
          {formatMeetingType(meeting.meetingType)}
        </Badge>
        <span className="flex-1 truncate font-medium">{meeting.title}</span>
        <span className="shrink-0 text-sm text-muted-foreground">
          {formatRelativeDate(meeting.startTime)}
        </span>
      </div>

      {/* Prep context preview (only if exists) */}
      {prep && (prep.intelligenceSummary || prep.proposedAgenda?.length) && (
        <div className="border-t px-4 py-2.5 space-y-1.5">
          {prep.intelligenceSummary && (
            <p className="text-xs text-muted-foreground line-clamp-2">
              {prep.intelligenceSummary}
            </p>
          )}
          {prep.proposedAgenda && prep.proposedAgenda.length > 0 && (
            <div className="flex gap-1.5 text-xs text-muted-foreground">
              <span className="shrink-0">Agenda:</span>
              <span className="truncate">
                {prep.proposedAgenda
                  .slice(0, 2)
                  .map((a) => a.topic)
                  .join(" · ")}
                {prep.proposedAgenda.length > 2 &&
                  ` +${prep.proposedAgenda.length - 2} more`}
              </span>
            </div>
          )}
          {(riskCount > 0 || actionCount > 0 || questionCount > 0) && (
            <div className="flex items-center gap-3 text-xs text-muted-foreground">
              {riskCount > 0 && (
                <span>
                  {riskCount} risk{riskCount !== 1 ? "s" : ""}
                </span>
              )}
              {actionCount > 0 && (
                <span>
                  {actionCount} action{actionCount !== 1 ? "s" : ""}
                </span>
              )}
              {questionCount > 0 && (
                <span>
                  {questionCount} question{questionCount !== 1 ? "s" : ""}
                </span>
              )}
              <span className="ml-auto text-primary">View full prep &rarr;</span>
            </div>
          )}
        </div>
      )}
    </Link>
  );
}

// ─── Intelligence Sub-components ─────────────────────────────────────────────

function AttentionRisk({ risk }: { risk: IntelRisk }) {
  const urgencyStyles: Record<string, string> = {
    critical: "bg-destructive/10 text-destructive",
    watch: "bg-[hsl(var(--peach))]/10 text-[hsl(var(--peach))]",
    low: "bg-muted text-muted-foreground",
  };

  return (
    <div className="flex items-start gap-3 rounded-lg bg-destructive/8 px-4 py-3">
      <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
      <div className="min-w-0 flex-1">
        <span className="text-sm">{risk.text}</span>
        <div className="mt-1 flex items-center gap-2">
          {risk.urgency && (
            <Badge
              className={cn(
                "text-[10px]",
                urgencyStyles[risk.urgency] ?? urgencyStyles.low
              )}
            >
              {risk.urgency}
            </Badge>
          )}
          {risk.source && (
            <span className="text-xs text-muted-foreground">
              {risk.source}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

function AttentionWin({ win }: { win: IntelWin }) {
  return (
    <div className="flex items-start gap-3 rounded-lg bg-green-500/8 px-4 py-3">
      <Trophy className="mt-0.5 size-4 shrink-0 text-green-600" />
      <div className="min-w-0 flex-1">
        <span className="text-sm">{win.text}</span>
        {(win.impact || win.source) && (
          <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
            {win.impact && <span>{win.impact}</span>}
            {win.source && <span>{win.source}</span>}
          </div>
        )}
      </div>
    </div>
  );
}

function StakeholderCard({
  insight,
  linkedPeople,
}: {
  insight: StakeholderInsight;
  linkedPeople: { id: string; name: string; role?: string }[];
}) {
  // Try to match to a linked person for navigation
  const matchedPerson = linkedPeople.find(
    (p) => p.name.toLowerCase() === insight.name.toLowerCase()
  );

  const engagementStyles: Record<string, string> = {
    high: "text-green-600",
    medium: "text-primary",
    low: "text-muted-foreground",
    champion: "text-green-600",
    neutral: "text-muted-foreground",
    detractor: "text-destructive",
  };

  const content = (
    <div className="rounded-lg border px-4 py-3">
      <div className="flex items-center gap-2">
        <span className="font-medium text-sm">{insight.name}</span>
        {insight.role && (
          <span className="text-xs text-muted-foreground">{insight.role}</span>
        )}
        {insight.engagement && (
          <Badge
            variant="outline"
            className={cn(
              "ml-auto text-[10px]",
              engagementStyles[insight.engagement.toLowerCase()] ?? ""
            )}
          >
            {insight.engagement}
          </Badge>
        )}
      </div>
      {insight.assessment && (
        <p className="mt-1 text-sm text-muted-foreground">
          {insight.assessment}
        </p>
      )}
    </div>
  );

  if (matchedPerson) {
    return (
      <Link
        to="/people/$personId"
        params={{ personId: matchedPerson.id }}
        className="block transition-colors hover:bg-muted/30 rounded-lg"
      >
        {content}
      </Link>
    );
  }

  return content;
}

// ─── I163: Strategic Program Inline Editing ──────────────────────────────────

const programStatusOptions = ["Active", "Planning", "On Hold", "Complete"] as const;

function ProgramRow({
  program,
  autoFocusName,
  onUpdate,
  onDelete,
}: {
  program: StrategicProgram;
  autoFocusName?: boolean;
  onUpdate: (updated: StrategicProgram) => void;
  onDelete: () => void;
}) {
  const [editingField, setEditingField] = useState<"name" | "notes" | null>(
    autoFocusName ? "name" : null
  );

  return (
    <div className="flex items-center gap-2 rounded-md px-1 py-1.5 group">
      {/* Name — click to edit */}
      {editingField === "name" ? (
        <Input
          autoFocus
          defaultValue={program.name}
          placeholder="Program name"
          onBlur={(e) => {
            const val = e.target.value.trim();
            if (val) {
              onUpdate({ ...program, name: val });
            } else if (!program.name) {
              // Empty new program — remove it
              onDelete();
            }
            setEditingField(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
            if (e.key === "Escape") {
              e.currentTarget.value = program.name;
              setEditingField(null);
            }
          }}
          className="h-7 text-sm flex-1"
        />
      ) : (
        <span
          className="flex-1 truncate text-sm cursor-pointer hover:underline decoration-muted-foreground/40"
          onClick={() => setEditingField("name")}
        >
          {program.name || "Untitled program"}
        </span>
      )}

      {/* Status dropdown */}
      <select
        value={program.status}
        onChange={(e) => onUpdate({ ...program, status: e.target.value })}
        className="h-7 shrink-0 rounded-md border bg-background px-2 text-xs outline-none focus:ring-1 focus:ring-ring"
      >
        {programStatusOptions.map((s) => (
          <option key={s} value={s}>
            {s}
          </option>
        ))}
      </select>

      {/* Notes — click to edit */}
      {editingField === "notes" ? (
        <Input
          autoFocus
          defaultValue={program.notes ?? ""}
          placeholder="Notes (optional)"
          onBlur={(e) => {
            onUpdate({ ...program, notes: e.target.value.trim() || undefined });
            setEditingField(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
            if (e.key === "Escape") {
              e.currentTarget.value = program.notes ?? "";
              setEditingField(null);
            }
          }}
          className="h-7 text-xs flex-1"
        />
      ) : (
        <span
          className="truncate text-xs text-muted-foreground cursor-pointer hover:underline decoration-muted-foreground/40 max-w-[120px]"
          onClick={() => setEditingField("notes")}
          title={program.notes ?? "Add notes"}
        >
          {program.notes || "notes"}
        </span>
      )}

      {/* Delete — visible on hover */}
      <Button
        variant="ghost"
        size="icon"
        className="h-6 w-6 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={onDelete}
      >
        <X className="size-3" />
      </Button>
    </div>
  );
}

// ─── Existing Sub-components ─────────────────────────────────────────────────

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
  editName,
  setEditName,
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
  editName: string;
  setEditName: (v: string) => void;
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
          placeholder="Account name"
          className={inputClass}
        />
      </div>
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

function FileRow({ file }: { file: ContentFile }) {
  return (
    <div
      className="flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-muted"
      onClick={() => invoke("reveal_in_finder", { path: file.absolutePath })}
    >
      <File className="size-3.5 shrink-0 text-muted-foreground" />
      <span className="flex-1 truncate">{file.filename}</span>
      <span className="shrink-0 font-mono text-xs text-muted-foreground">
        {formatFileSize(file.fileSize)}
      </span>
      <span className="shrink-0 text-xs text-muted-foreground">
        {formatRelativeDateShort(file.modifiedAt)}
      </span>
    </div>
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

function eventBadgeClass(type: string): string {
  switch (type) {
    case "renewal":
    case "expansion":
      return "text-green-600 border-green-600/30";
    case "churn":
    case "downgrade":
      return "text-destructive border-destructive/30";
    default:
      return "";
  }
}

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: 0,
  }).format(amount);
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
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}

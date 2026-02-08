import { useState, useEffect, useCallback } from "react";
import { useParams, Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PageError } from "@/components/PageState";
import { cn } from "@/lib/utils";
import {
  ArrowLeft,
  Calendar,
  CheckCircle2,
  Minus,
  Save,
  TrendingDown,
  TrendingUp,
  Users,
} from "lucide-react";
import type { AccountDetail, AccountHealth } from "@/types";

const healthOptions: AccountHealth[] = ["green", "yellow", "red"];

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

export default function AccountDetailPage() {
  const { accountId } = useParams({ strict: false });
  const [detail, setDetail] = useState<AccountDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Editable structured fields
  const [editHealth, setEditHealth] = useState<string>("");
  const [editRing, setEditRing] = useState<string>("");
  const [editArr, setEditArr] = useState<string>("");
  const [editNps, setEditNps] = useState<string>("");
  const [editCsm, setEditCsm] = useState<string>("");
  const [editChampion, setEditChampion] = useState<string>("");
  const [editRenewal, setEditRenewal] = useState<string>("");
  const [editNotes, setEditNotes] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

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
      setEditRing(result.ring?.toString() ?? "");
      setEditArr(result.arr?.toString() ?? "");
      setEditNps(result.nps?.toString() ?? "");
      setEditCsm(result.csm ?? "");
      setEditChampion(result.champion ?? "");
      setEditRenewal(result.renewalDate ?? "");
      setEditNotes(result.notes ?? "");
      setDirty(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    load();
  }, [load]);

  async function handleSave() {
    if (!detail) return;
    setSaving(true);

    try {
      // Save structured field changes
      const fieldUpdates: [string, string][] = [];
      if (editHealth !== (detail.health ?? ""))
        fieldUpdates.push(["health", editHealth]);
      if (editRing !== (detail.ring?.toString() ?? ""))
        fieldUpdates.push(["ring", editRing]);
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

      // Save notes if changed
      if (editNotes !== (detail.notes ?? "")) {
        await invoke("update_account_notes", {
          accountId: detail.id,
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
        <PageError message={error ?? "Account not found"} onRetry={load} />
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
            to="/accounts"
            className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
            Accounts
          </Link>

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
                  {detail.health && (
                    <HealthBadge health={detail.health as AccountHealth} />
                  )}
                  {detail.ring && (
                    <Badge variant="outline" className="text-xs">
                      Ring {detail.ring}
                    </Badge>
                  )}
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
                {detail.arr != null && (
                  <span className="text-sm text-muted-foreground">
                    ARR: ${formatArr(detail.arr)}
                    {detail.renewalDate && (
                      <> &middot; Renews {detail.renewalDate}</>
                    )}
                  </span>
                )}
              </div>
            </div>

            {dirty && (
              <Button size="sm" onClick={handleSave} disabled={saving}>
                <Save className="mr-1 size-4" />
                {saving ? "Saving..." : "Save"}
              </Button>
            )}
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
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
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
                      Ring
                    </label>
                    <select
                      value={editRing}
                      onChange={(e) => {
                        setEditRing(e.target.value);
                        setDirty(true);
                      }}
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                    >
                      <option value="">Not set</option>
                      {[1, 2, 3, 4].map((r) => (
                        <option key={r} value={r}>
                          Ring {r}
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
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
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
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
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
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
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
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
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
                      className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Engagement Signals */}
            {signals && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Engagement Signals
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
                          {formatDate(signals.lastMeeting)}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          last meeting
                        </div>
                      </div>
                    )}
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Company Overview (read-only, I74 adds refresh) */}
            {detail.companyOverview && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Company Overview
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2 text-sm">
                  {detail.companyOverview.description && (
                    <p>{detail.companyOverview.description}</p>
                  )}
                  <div className="flex flex-wrap gap-x-4 gap-y-1 text-muted-foreground">
                    {detail.companyOverview.industry && (
                      <span>Industry: {detail.companyOverview.industry}</span>
                    )}
                    {detail.companyOverview.size && (
                      <span>Size: {detail.companyOverview.size}</span>
                    )}
                    {detail.companyOverview.headquarters && (
                      <span>HQ: {detail.companyOverview.headquarters}</span>
                    )}
                  </div>
                </CardContent>
              </Card>
            )}

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
                  placeholder="Notes about this account..."
                  rows={4}
                  className="w-full resize-none rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                />
              </CardContent>
            </Card>

            {/* Strategic Programs */}
            {detail.strategicPrograms.length > 0 && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">
                    Strategic Programs
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2">
                    {detail.strategicPrograms.map((p, i) => (
                      <div key={i} className="flex items-center gap-2 text-sm">
                        <ProgramStatusBadge status={p.status} />
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
                          {formatDate(m.startTime)}
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

            {/* Stakeholder Map */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">
                  Stakeholder Map
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
                        className="flex items-center gap-2 text-sm hover:text-primary transition-colors"
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
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function HealthBadge({ health }: { health: AccountHealth }) {
  const styles: Record<AccountHealth, string> = {
    green:
      "bg-green-100 text-green-800 border-green-300 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700",
    yellow:
      "bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/30 dark:text-yellow-400 dark:border-yellow-700",
    red: "bg-red-100 text-red-800 border-red-300 dark:bg-red-900/30 dark:text-red-400 dark:border-red-700",
  };
  return (
    <Badge variant="outline" className={cn("text-xs", styles[health])}>
      {health}
    </Badge>
  );
}

function ProgramStatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    completed: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
    in_progress: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400",
    planned: "bg-muted text-muted-foreground",
  };
  return (
    <Badge
      variant="outline"
      className={cn("text-xs", styles[status] ?? styles.planned)}
    >
      {status.replace("_", " ")}
    </Badge>
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

function formatArr(arr: number): string {
  if (arr >= 1_000_000) return `${(arr / 1_000_000).toFixed(1)}M`;
  if (arr >= 1_000) return `${(arr / 1_000).toFixed(0)}K`;
  return arr.toFixed(0);
}

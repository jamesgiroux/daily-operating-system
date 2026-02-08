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
  Building2,
  Calendar,
  Mail,
  Save,
  TrendingDown,
  TrendingUp,
  Minus,
  X,
} from "lucide-react";
import type { PersonDetail } from "@/types";

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

export default function PersonDetailPage() {
  const { personId } = useParams({ strict: false });
  const [detail, setDetail] = useState<PersonDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Editable fields
  const [editRole, setEditRole] = useState("");
  const [editOrg, setEditOrg] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    if (!personId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<PersonDetail>("get_person_detail", {
        personId,
      });
      setDetail(result);
      setEditRole(result.role ?? "");
      setEditOrg(result.organization ?? "");
      setEditNotes(result.notes ?? "");
      setDirty(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [personId]);

  useEffect(() => {
    load();
  }, [load]);

  async function saveField(field: string, value: string) {
    if (!detail) return;
    try {
      setSaving(true);
      await invoke("update_person", {
        personId: detail.id,
        field,
        value,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleSave() {
    if (!detail) return;
    const updates: [string, string][] = [];
    if (editRole !== (detail.role ?? "")) updates.push(["role", editRole]);
    if (editOrg !== (detail.organization ?? "")) updates.push(["organization", editOrg]);
    if (editNotes !== (detail.notes ?? "")) updates.push(["notes", editNotes]);

    for (const [field, value] of updates) {
      await saveField(field, value);
    }
    setDirty(false);
    await load(); // Refresh to get updated data
  }

  async function handleUnlink(entityId: string) {
    if (!detail) return;
    try {
      await invoke("unlink_person_entity", {
        personId: detail.id,
        entityId,
      });
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
        <PageError message={error ?? "Person not found"} onRetry={load} />
      </main>
    );
  }

  const signals = detail.signals;

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          {/* Back + header */}
          <Link to="/people" className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors">
            <ArrowLeft className="size-4" />
            People
          </Link>

          <div className="mb-6 flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="flex size-12 items-center justify-center rounded-full bg-primary/10 text-lg font-semibold text-primary">
                {detail.name.charAt(0).toUpperCase()}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-2xl font-semibold tracking-tight">{detail.name}</h1>
                  <Badge
                    variant="outline"
                    className={cn(
                      "text-xs",
                      detail.relationship === "internal"
                        ? "bg-muted text-muted-foreground"
                        : "bg-primary/10 text-primary"
                    )}
                  >
                    {detail.relationship}
                  </Badge>
                  {signals && (
                    <Badge className={cn("text-xs", temperatureStyles[signals.temperature] ?? temperatureStyles.cool)}>
                      {signals.temperature}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Mail className="size-3" />
                  {detail.email}
                </div>
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
            {/* Editable info card */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Details</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <label className="text-xs font-medium text-muted-foreground">Organization</label>
                  <input
                    type="text"
                    value={editOrg}
                    onChange={(e) => {
                      setEditOrg(e.target.value);
                      setDirty(true);
                    }}
                    placeholder="Organization"
                    className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>
                <div>
                  <label className="text-xs font-medium text-muted-foreground">Role</label>
                  <input
                    type="text"
                    value={editRole}
                    onChange={(e) => {
                      setEditRole(e.target.value);
                      setDirty(true);
                    }}
                    placeholder="Role / Title"
                    className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>
                <div>
                  <label className="text-xs font-medium text-muted-foreground">Notes</label>
                  <textarea
                    value={editNotes}
                    onChange={(e) => {
                      setEditNotes(e.target.value);
                      setDirty(true);
                    }}
                    placeholder="Notes about this person..."
                    rows={4}
                    className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring resize-none"
                  />
                </div>
              </CardContent>
            </Card>

            {/* Signals card */}
            {signals && (
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm font-medium">Meeting Signals</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <div className="text-2xl font-semibold">{signals.meetingFrequency30d}</div>
                      <div className="text-xs text-muted-foreground">meetings (30d)</div>
                    </div>
                    <div>
                      <div className="text-2xl font-semibold">{signals.meetingFrequency90d}</div>
                      <div className="text-xs text-muted-foreground">meetings (90d)</div>
                    </div>
                    <div className="flex items-center gap-2">
                      <TrendIcon trend={signals.trend} />
                      <div>
                        <div className="text-sm font-medium capitalize">{signals.trend}</div>
                        <div className="text-xs text-muted-foreground">trend</div>
                      </div>
                    </div>
                    {signals.lastMeeting && (
                      <div>
                        <div className="text-sm font-medium">
                          {formatDate(signals.lastMeeting)}
                        </div>
                        <div className="text-xs text-muted-foreground">last meeting</div>
                      </div>
                    )}
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Linked entities */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Linked Entities</CardTitle>
              </CardHeader>
              <CardContent>
                {detail.entities && detail.entities.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {detail.entities.map((e) => (
                      <Badge key={e.id} variant="secondary" className="gap-1 pr-1">
                        <Building2 className="size-3" />
                        {e.name}
                        <button
                          onClick={() => handleUnlink(e.id)}
                          className="ml-1 rounded-full p-0.5 hover:bg-muted-foreground/20"
                        >
                          <X className="size-3" />
                        </button>
                      </Badge>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">No linked accounts or projects.</p>
                )}
              </CardContent>
            </Card>

            {/* Recent meetings */}
            <Card>
              <CardHeader className="pb-3">
                <CardTitle className="text-sm font-medium">Recent Meetings</CardTitle>
              </CardHeader>
              <CardContent>
                {detail.recentMeetings && detail.recentMeetings.length > 0 ? (
                  <div className="space-y-2">
                    {detail.recentMeetings.map((m) => (
                      <div key={m.id} className="flex items-center gap-2 text-sm">
                        <Calendar className="size-3.5 shrink-0 text-muted-foreground" />
                        <span className="truncate">{m.title}</span>
                        <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                          {formatDate(m.startTime)}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">No meetings recorded yet.</p>
                )}
              </CardContent>
            </Card>
          </div>
        </div>
      </ScrollArea>
    </main>
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

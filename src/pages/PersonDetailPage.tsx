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
  Pencil,
  Save,
  X,
} from "lucide-react";
import type { PersonDetail } from "@/types";

const temperatureStyles: Record<string, { dot: string; badge: string }> = {
  hot: { dot: "bg-success", badge: "bg-success/15 text-success" },
  warm: { dot: "bg-primary", badge: "bg-primary/15 text-primary" },
  cool: { dot: "bg-muted-foreground/40", badge: "bg-muted text-muted-foreground" },
  cold: { dot: "bg-destructive", badge: "bg-destructive/15 text-destructive" },
};

export default function PersonDetailPage() {
  const { personId } = useParams({ strict: false });
  const [detail, setDetail] = useState<PersonDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);

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
    setEditing(false);
    await load();
  }

  function handleCancelEdit() {
    if (!detail) return;
    setEditRole(detail.role ?? "");
    setEditOrg(detail.organization ?? "");
    setEditNotes(detail.notes ?? "");
    setDirty(false);
    setEditing(false);
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
        <PageError message={error ?? "Person not found"} onRetry={load} />
      </main>
    );
  }

  const signals = detail.signals;
  const tempStyle = temperatureStyles[signals?.temperature ?? ""] ?? temperatureStyles.cool;

  // Build metrics — only include items with data
  const metrics: { label: string; value: string }[] = [];
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
    if (signals.temperature) {
      metrics.push({ label: "Engagement", value: signals.temperature });
    }
    if (signals.lastMeeting) {
      metrics.push({ label: "Last Meeting", value: formatDate(signals.lastMeeting) });
    }
  }
  if (detail.meetingCount > 0) {
    metrics.push({ label: "Total Meetings", value: String(detail.meetingCount) });
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="space-y-8 p-8">
          {/* Back link */}
          <Link
            to="/people"
            className="inline-flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
            People
          </Link>

          {/* Hero Section */}
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-4">
              <div className="flex size-14 items-center justify-center rounded-full bg-muted text-xl font-semibold text-muted-foreground">
                {detail.name.charAt(0).toUpperCase()}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-3xl font-semibold tracking-tight">
                    {detail.name}
                  </h1>
                  <Badge
                    variant="outline"
                    className={cn(
                      "text-xs",
                      detail.relationship === "internal"
                        ? "bg-muted text-muted-foreground border-muted-foreground/30"
                        : detail.relationship === "external"
                          ? "bg-primary/10 text-primary border-primary/30"
                          : ""
                    )}
                  >
                    {detail.relationship}
                  </Badge>
                  {signals?.temperature && (
                    <Badge className={cn("text-xs capitalize", tempStyle.badge)}>
                      {signals.temperature}
                    </Badge>
                  )}
                </div>
                <div className="flex items-center gap-4 text-sm text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Mail className="size-3" />
                    {detail.email}
                  </span>
                  {detail.organization && (
                    <span className="flex items-center gap-1">
                      <Building2 className="size-3" />
                      {detail.organization}
                      {detail.role && ` \u00B7 ${detail.role}`}
                    </span>
                  )}
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

          {/* Metrics Row */}
          {metrics.length > 0 && (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
              {metrics.map((m) => (
                <div key={m.label} className="rounded-lg border bg-card px-4 py-3">
                  <div className="text-xl font-semibold capitalize">{m.value}</div>
                  <div className="text-xs text-muted-foreground">{m.label}</div>
                </div>
              ))}
            </div>
          )}

          {/* Asymmetric Grid: Main (3fr) + Sidebar (2fr) */}
          <div className="grid gap-6 lg:grid-cols-[3fr_2fr]">
            {/* Main Column */}
            <div className="space-y-6">
              {/* Recent Meetings */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Recent Meetings
                    {detail.recentMeetings && detail.recentMeetings.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({detail.recentMeetings.length})
                      </span>
                    )}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {detail.recentMeetings && detail.recentMeetings.length > 0 ? (
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
                            {formatDate(m.startTime)}
                          </span>
                        </Link>
                      ))}
                    </div>
                  ) : (
                    <EmptyState icon={Calendar} message="No meetings recorded yet" />
                  )}
                </CardContent>
              </Card>

              {/* Linked Entities */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="pb-3">
                  <CardTitle className="text-base font-semibold">
                    Linked Entities
                    {detail.entities && detail.entities.length > 0 && (
                      <span className="ml-1 text-muted-foreground">
                        ({detail.entities.length})
                      </span>
                    )}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {detail.entities && detail.entities.length > 0 ? (
                    <div className="space-y-2">
                      {detail.entities.map((e) => (
                        <div
                          key={e.id}
                          className="flex items-center justify-between gap-2 text-sm"
                        >
                          <Link
                            to="/accounts/$accountId"
                            params={{ accountId: e.id }}
                            className="flex items-center gap-2 transition-colors hover:text-primary"
                          >
                            <Building2 className="size-3.5 shrink-0 text-muted-foreground" />
                            <span className="font-medium">{e.name}</span>
                            <Badge variant="outline" className="text-xs capitalize">
                              {e.entityType}
                            </Badge>
                          </Link>
                          <button
                            onClick={() => handleUnlink(e.id)}
                            className="rounded-full p-1 text-muted-foreground hover:bg-muted-foreground/20 hover:text-foreground"
                          >
                            <X className="size-3" />
                          </button>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <EmptyState icon={Building2} message="No linked accounts or projects" />
                  )}
                </CardContent>
              </Card>
            </div>

            {/* Sidebar Column */}
            <div className="space-y-6">
              {/* Person Details (read-first with edit toggle) */}
              <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                <CardHeader className="flex flex-row items-center justify-between pb-3">
                  <CardTitle className="text-base font-semibold">
                    Details
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
                    <PersonDetailsEditForm
                      editOrg={editOrg}
                      setEditOrg={setEditOrg}
                      editRole={editRole}
                      setEditRole={setEditRole}
                      editNotes={editNotes}
                      setEditNotes={setEditNotes}
                      setDirty={setDirty}
                      onSave={handleSave}
                      onCancel={handleCancelEdit}
                      saving={saving}
                      dirty={dirty}
                    />
                  ) : (
                    <PersonDetailsReadView detail={detail} />
                  )}
                </CardContent>
              </Card>

              {/* Notes (always visible for quick access) */}
              {!editing && (
                <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md">
                  <CardHeader className="pb-3">
                    <CardTitle className="text-base font-semibold">
                      Notes
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    {detail.notes ? (
                      <p className="whitespace-pre-wrap text-sm">{detail.notes}</p>
                    ) : (
                      <p className="text-sm text-muted-foreground">
                        No notes yet. Click the pencil icon above to add notes.
                      </p>
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

function PersonDetailsReadView({ detail }: { detail: PersonDetail }) {
  const fields: { label: string; value: React.ReactNode }[] = [];

  if (detail.organization) {
    fields.push({ label: "Organization", value: detail.organization });
  }
  if (detail.role) {
    fields.push({ label: "Role", value: detail.role });
  }
  if (detail.email) {
    fields.push({ label: "Email", value: detail.email });
  }
  if (detail.firstSeen) {
    fields.push({ label: "First Seen", value: formatDate(detail.firstSeen) });
  }
  if (detail.lastSeen) {
    fields.push({ label: "Last Seen", value: formatDate(detail.lastSeen) });
  }

  if (fields.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No details set. Click the pencil icon to add information.
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

function PersonDetailsEditForm({
  editOrg,
  setEditOrg,
  editRole,
  setEditRole,
  editNotes,
  setEditNotes,
  setDirty,
  onSave,
  onCancel,
  saving,
  dirty,
}: {
  editOrg: string;
  setEditOrg: (v: string) => void;
  editRole: string;
  setEditRole: (v: string) => void;
  editNotes: string;
  setEditNotes: (v: string) => void;
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
          Organization
        </label>
        <input
          type="text"
          value={editOrg}
          onChange={(e) => {
            setEditOrg(e.target.value);
            setDirty(true);
          }}
          placeholder="Organization"
          className={inputClass}
        />
      </div>
      <div>
        <label className="text-xs font-medium text-muted-foreground">
          Role
        </label>
        <input
          type="text"
          value={editRole}
          onChange={(e) => {
            setEditRole(e.target.value);
            setDirty(true);
          }}
          placeholder="Role / Title"
          className={inputClass}
        />
      </div>
      <div>
        <label className="text-xs font-medium text-muted-foreground">
          Notes
        </label>
        <textarea
          value={editNotes}
          onChange={(e) => {
            setEditNotes(e.target.value);
            setDirty(true);
          }}
          placeholder="Notes about this person..."
          rows={4}
          className={cn(inputClass, "resize-none")}
        />
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

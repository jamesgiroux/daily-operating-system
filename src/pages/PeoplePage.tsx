import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PageError, PageEmpty } from "@/components/PageState";
import { cn } from "@/lib/utils";
import { RefreshCw, Search, Users } from "lucide-react";
import type { Person, PersonRelationship } from "@/types";

type RelationshipTab = "all" | "external" | "internal";

const tabs: { key: RelationshipTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "external", label: "External" },
  { key: "internal", label: "Internal" },
];

export default function PeoplePage() {
  const [people, setPeople] = useState<Person[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<RelationshipTab>("all");
  const [searchQuery, setSearchQuery] = useState("");

  const loadPeople = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const filter = tab === "all" ? undefined : tab;
      const result = await invoke<Person[]>("get_people", {
        relationship: filter ?? null,
      });
      setPeople(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [tab]);

  useEffect(() => {
    loadPeople();
  }, [loadPeople]);

  useEffect(() => {
    const unlisten = listen("people-updated", () => loadPeople());
    return () => {
      unlisten.then((f) => f());
    };
  }, [loadPeople]);

  const filtered = searchQuery
    ? people.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          p.email.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.organization ?? "").toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.role ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : people;

  const tabCounts: Record<RelationshipTab, number> = {
    all: people.length,
    external: people.filter((p) => p.relationship === "external").length,
    internal: people.filter((p) => p.relationship === "internal").length,
  };

  if (loading && people.length === 0) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-20 w-full" />
          ))}
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} onRetry={loadPeople} />
      </main>
    );
  }

  if (people.length === 0) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageEmpty
          icon={Users}
          title="No people discovered yet"
          message="People are discovered automatically from your calendar. Connect Google in Settings to get started."
        />
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6 flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">
                People
                <span className="ml-2 text-base font-normal text-muted-foreground">
                  {filtered.length}
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                People discovered from your calendar and meetings
              </p>
            </div>
            <Button variant="ghost" size="icon" className="size-8" onClick={loadPeople}>
              <RefreshCw className="size-4" />
            </Button>
          </div>

          {/* Search */}
          <div className="relative mb-4">
            <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search people..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full rounded-md border bg-background py-2 pl-10 pr-4 text-sm outline-none focus:ring-1 focus:ring-ring"
            />
          </div>

          {/* Relationship tabs */}
          <div className="mb-6 flex gap-2">
            {tabs.map((t) => (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={cn(
                  "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                  tab === t.key
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted hover:bg-muted/80"
                )}
              >
                {t.label}
                {tabCounts[t.key] > 0 && (
                  <span
                    className={cn(
                      "ml-1.5 inline-flex size-5 items-center justify-center rounded-full text-xs",
                      tab === t.key
                        ? "bg-primary-foreground/20 text-primary-foreground"
                        : "bg-muted-foreground/15 text-muted-foreground"
                    )}
                  >
                    {tabCounts[t.key]}
                  </span>
                )}
              </button>
            ))}
          </div>

          {/* People list */}
          <div className="space-y-2">
            {filtered.length === 0 ? (
              <Card>
                <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                  <Users className="mb-4 size-12 text-muted-foreground/40" />
                  <p className="text-lg font-medium">No matches</p>
                  <p className="text-sm text-muted-foreground">
                    Try a different search or filter.
                  </p>
                </CardContent>
              </Card>
            ) : (
              filtered.map((person) => (
                <PersonRow key={person.id} person={person} />
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function PersonRow({ person }: { person: Person }) {
  const lastSeenLabel = person.lastSeen ? formatRelativeDate(person.lastSeen) : null;

  return (
    <Link to="/people/$personId" params={{ personId: person.id }}>
      <Card className="transition-all hover:-translate-y-0.5 hover:shadow-md cursor-pointer">
        <CardContent className="flex items-center gap-4 p-4">
          {/* Avatar initial */}
          <div className="flex size-10 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">
            {person.name.charAt(0).toUpperCase()}
          </div>

          {/* Name + org + role */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-medium truncate">{person.name}</span>
              <RelationshipBadge relationship={person.relationship} />
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              {person.organization && <span>{person.organization}</span>}
              {person.organization && person.role && (
                <span className="text-muted-foreground/40">·</span>
              )}
              {person.role && <span>{person.role}</span>}
            </div>
          </div>

          {/* Meetings count */}
          <div className="text-right shrink-0">
            <div className="text-sm font-medium">{person.meetingCount}</div>
            <div className="text-xs text-muted-foreground">meetings</div>
          </div>

          {/* Last seen */}
          {lastSeenLabel && (
            <div className="text-right shrink-0 w-20">
              <div className="text-xs text-muted-foreground">{lastSeenLabel}</div>
            </div>
          )}
        </CardContent>
      </Card>
    </Link>
  );
}

function RelationshipBadge({ relationship }: { relationship: PersonRelationship }) {
  if (relationship === "unknown") return null;
  return (
    <Badge
      variant="outline"
      className={cn(
        "text-xs",
        relationship === "internal"
          ? "bg-muted text-muted-foreground border-muted-foreground/30"
          : "bg-primary/10 text-primary border-primary/30"
      )}
    >
      {relationship}
    </Badge>
  );
}

function formatRelativeDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.floor(
      (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
    );

    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays}d ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
    return `${Math.floor(diffDays / 30)}mo ago`;
  } catch {
    return "";
  }
}

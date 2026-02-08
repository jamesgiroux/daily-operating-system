import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { ListRow, ListColumn } from "@/components/ui/list-row";
import { PageError, PageEmpty } from "@/components/PageState";
import { cn, formatRelativeDate } from "@/lib/utils";
import { RefreshCw, Users } from "lucide-react";
import type { PersonListItem, PersonRelationship } from "@/types";

type RelationshipTab = "all" | "external" | "internal" | "unknown";

const tabs: { key: RelationshipTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "external", label: "External" },
  { key: "internal", label: "Internal" },
  { key: "unknown", label: "Unknown" },
];

const tempOrder: Record<string, number> = {
  hot: 0,
  warm: 1,
  cool: 2,
  cold: 3,
};

export default function PeoplePage() {
  const [people, setPeople] = useState<PersonListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<RelationshipTab>("all");
  const [searchQuery, setSearchQuery] = useState("");

  const loadPeople = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const filter = tab === "all" ? undefined : tab;
      const result = await invoke<PersonListItem[]>("get_people", {
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

  // Sort by temperature (hot first), then by last-seen (most recent first)
  const sorted = useMemo(() => {
    return [...filtered].sort((a, b) => {
      const ta = tempOrder[a.temperature] ?? 4;
      const tb = tempOrder[b.temperature] ?? 4;
      if (ta !== tb) return ta - tb;
      const aDate = a.lastSeen ? new Date(a.lastSeen).getTime() : 0;
      const bDate = b.lastSeen ? new Date(b.lastSeen).getTime() : 0;
      return bDate - aDate;
    });
  }, [filtered]);

  const tabCounts: Record<RelationshipTab, number> = {
    all: people.length,
    external: people.filter((p) => p.relationship === "external").length,
    internal: people.filter((p) => p.relationship === "internal").length,
    unknown: people.filter((p) => p.relationship === "unknown").length,
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
            <Skeleton key={i} className="h-12 w-full" />
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

  const showRelationship = tab === "all";

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

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search people..."
            className="mb-4"
          />

          <TabFilter
            tabs={tabs}
            active={tab}
            onChange={setTab}
            counts={tabCounts}
            className="mb-6"
          />

          {/* People list */}
          <div>
            {sorted.length === 0 ? (
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
              sorted.map((person) => (
                <PersonRow
                  key={person.id}
                  person={person}
                  showRelationship={showRelationship}
                />
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function PersonRow({
  person,
  showRelationship,
}: {
  person: PersonListItem;
  showRelationship: boolean;
}) {
  const tempDot: Record<string, string> = {
    hot: "bg-success",
    warm: "bg-primary",
    cool: "bg-muted-foreground/40",
    cold: "bg-destructive",
  };

  const trendArrow =
    person.trend === "increasing" ? (
      <span className="text-xs text-success">{"\u25B2"}</span>
    ) : person.trend === "decreasing" ? (
      <span className="text-xs text-destructive">{"\u25BC"}</span>
    ) : null;

  const lastSeen = person.lastSeen ? formatRelativeDate(person.lastSeen) : null;

  return (
    <ListRow
      to="/people/$personId"
      params={{ personId: person.id }}
      signalColor={tempDot[person.temperature] ?? "bg-muted-foreground/30"}
      name={person.name}
      badges={
        <>
          {trendArrow}
          {showRelationship && person.relationship !== "unknown" && (
            <RelationshipBadge relationship={person.relationship} />
          )}
        </>
      }
      subtitle={
        <>
          {person.organization}
          {person.organization && person.role && " \u00B7 "}
          {person.role}
        </>
      }
      columns={
        lastSeen ? (
          <ListColumn value={lastSeen} label="last seen" className="w-16" />
        ) : undefined
      }
    />
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

import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useNavigate, useSearch, Link } from "@tanstack/react-router";
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
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { ListRow, ListColumn } from "@/components/ui/list-row";
import { PageError, PageEmpty, SectionEmpty } from "@/components/PageState";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import { cn, formatRelativeDate } from "@/lib/utils";
import { Plus, RefreshCw, Users } from "lucide-react";
import type { PersonListItem, PersonRelationship, DuplicateCandidate } from "@/types";

type ArchiveTab = "active" | "archived";
type RelationshipTab = "all" | "external" | "internal" | "unknown";
type HygieneFilter = "unnamed" | "duplicates";

const archiveTabs: { key: ArchiveTab; label: string }[] = [
  { key: "active", label: "Active" },
  { key: "archived", label: "Archived" },
];

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

function parseRelationshipTab(value: unknown): RelationshipTab {
  if (value === "external" || value === "internal" || value === "unknown") {
    return value;
  }
  return "all";
}

function parseHygieneFilter(value: unknown): HygieneFilter | undefined {
  if (value === "unnamed" || value === "duplicates") {
    return value;
  }
  return undefined;
}

function isLikelyUnnamedPerson(person: PersonListItem): boolean {
  const name = person.name.toLowerCase();
  return !name.includes(" ") || name.includes("@");
}

export default function PeoplePage() {
  const { personality } = usePersonality();
  const search = useSearch({ from: "/people" });
  const navigate = useNavigate();
  const initialRelationshipTab = parseRelationshipTab(search.relationship);
  const activeHygieneFilter = parseHygieneFilter(search.hygiene);
  const [people, setPeople] = useState<PersonListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedPeople, setArchivedPeople] = useState<PersonListItem[]>([]);
  const [tab, setTab] = useState<RelationshipTab>(initialRelationshipTab);
  const [searchQuery, setSearchQuery] = useState("");
  const [showAddForm, setShowAddForm] = useState(false);
  const [newEmail, setNewEmail] = useState("");
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [duplicates, setDuplicates] = useState<DuplicateCandidate[]>([]);
  const [showDuplicates, setShowDuplicates] = useState(activeHygieneFilter === "duplicates");

  const loadDuplicates = useCallback(() => {
    invoke<DuplicateCandidate[]>("get_duplicate_people")
      .then(setDuplicates)
      .catch(() => setDuplicates([]));
  }, []);

  useEffect(() => {
    loadDuplicates();
  }, [loadDuplicates]);

  useEffect(() => {
    setTab(parseRelationshipTab(search.relationship));
  }, [search.relationship]);

  useEffect(() => {
    if (activeHygieneFilter === "duplicates") {
      setShowDuplicates(true);
    }
  }, [activeHygieneFilter]);

  const handleCreatePerson = useCallback(async () => {
    if (!newEmail.trim() || !newName.trim()) return;
    try {
      setCreating(true);
      const personId = await invoke<string>("create_person", {
        email: newEmail.trim(),
        name: newName.trim(),
      });
      setShowAddForm(false);
      setNewEmail("");
      setNewName("");
      navigate({ to: "/people/$personId", params: { personId } });
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }, [newEmail, newName, navigate]);

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

  // I176: load archived people
  const loadArchivedPeople = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<PersonListItem[]>("get_archived_people");
      setArchivedPeople(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (archiveTab === "active") {
      loadPeople();
    } else {
      loadArchivedPeople();
    }
  }, [archiveTab, loadPeople, loadArchivedPeople]);

  useEffect(() => {
    const unlisten = listen("people-updated", () => {
      if (archiveTab === "active") {
        loadPeople();
      } else {
        loadArchivedPeople();
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [archiveTab, loadPeople, loadArchivedPeople]);

  const filtered = searchQuery
    ? people.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          p.email.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.organization ?? "").toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.role ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : people;

  const hygieneFiltered =
    activeHygieneFilter === "unnamed"
      ? filtered.filter(isLikelyUnnamedPerson)
      : filtered;

  // Sort by temperature (hot first), then by last-seen (most recent first)
  const sorted = useMemo(() => {
    return [...hygieneFiltered].sort((a, b) => {
      const ta = tempOrder[a.temperature] ?? 4;
      const tb = tempOrder[b.temperature] ?? 4;
      if (ta !== tb) return ta - tb;
      const aDate = a.lastSeen ? new Date(a.lastSeen).getTime() : 0;
      const bDate = b.lastSeen ? new Date(b.lastSeen).getTime() : 0;
      return bDate - aDate;
    });
  }, [hygieneFiltered]);

  // I176: filter archived people by search query
  const filteredArchived = searchQuery
    ? archivedPeople.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          p.email.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.organization ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedPeople;

  const isArchived = archiveTab === "archived";

  const tabCounts: Record<RelationshipTab, number> = {
    all: people.length,
    external: people.filter((p) => p.relationship === "external").length,
    internal: people.filter((p) => p.relationship === "internal").length,
    unknown: people.filter((p) => p.relationship === "unknown").length,
  };

  if (loading && (isArchived ? archivedPeople.length === 0 : people.length === 0)) {
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
        <PageError message={error} onRetry={isArchived ? loadArchivedPeople : loadPeople} />
      </main>
    );
  }

  if (!isArchived && people.length === 0) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageEmpty
          icon={Users}
          {...getPersonalityCopy("people-empty", personality)}
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
                  {isArchived ? filteredArchived.length : filtered.length}
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                {isArchived
                  ? "Previously tracked people"
                  : "People discovered from your calendar and meetings"}
              </p>
            </div>
            <div className="flex items-center gap-1">
              {!isArchived && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setShowAddForm(true)}
                >
                  <Plus className="mr-1 size-4" />
                  Add Person
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={isArchived ? loadArchivedPeople : loadPeople}
                disabled={loading}
              >
                <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
              </Button>
            </div>
          </div>

          <TabFilter
            tabs={archiveTabs}
            active={archiveTab}
            onChange={setArchiveTab}
            className="mb-4"
          />

          {!isArchived && showAddForm && (
            <Card className="mb-4">
              <CardContent className="flex items-center gap-2 py-3">
                <input
                  type="email"
                  autoFocus
                  value={newEmail}
                  onChange={(e) => setNewEmail(e.target.value)}
                  placeholder="Email"
                  className="flex-1 rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  onKeyDown={(e) => {
                    if (e.key === "Escape") {
                      setShowAddForm(false);
                      setNewEmail("");
                      setNewName("");
                    }
                  }}
                />
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="Name"
                  className="flex-1 rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleCreatePerson();
                    if (e.key === "Escape") {
                      setShowAddForm(false);
                      setNewEmail("");
                      setNewName("");
                    }
                  }}
                />
                <Button
                  size="sm"
                  onClick={handleCreatePerson}
                  disabled={creating || !newEmail.trim() || !newName.trim()}
                >
                  {creating ? "Creating..." : "Create"}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    setShowAddForm(false);
                    setNewEmail("");
                    setNewName("");
                  }}
                >
                  Cancel
                </Button>
              </CardContent>
            </Card>
          )}

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search people..."
            className="mb-4"
          />

          {!isArchived && (
            <TabFilter
              tabs={tabs}
              active={tab}
              onChange={(next) => {
                setTab(next);
                navigate({
                  to: "/people",
                  search: (prev: Record<string, unknown>) => ({
                    ...prev,
                    relationship: next === "all" ? undefined : next,
                  }),
                });
              }}
              counts={tabCounts}
              className="mb-6"
            />
          )}

          {!isArchived && activeHygieneFilter === "unnamed" && (
            <div className="mb-4 flex items-center justify-between rounded-lg border border-primary/20 bg-primary/5 px-4 py-2.5">
              <span className="text-sm text-charcoal/70">
                Showing people with likely placeholder names.
              </span>
              <Button
                variant="ghost"
                size="sm"
                className="text-xs text-primary"
                onClick={() => {
                  navigate({
                    to: "/people",
                    search: (prev: Record<string, unknown>) => ({
                      ...prev,
                      hygiene: undefined,
                    }),
                  });
                }}
              >
                Clear
              </Button>
            </div>
          )}

          {/* I172: Duplicate detection banner */}
          {duplicates.length > 0 && !isArchived && (
            <div className="mb-4 space-y-2">
              <div className="rounded-lg border border-primary/20 bg-primary/5 px-4 py-2.5 flex items-center justify-between">
                <span className="text-sm text-charcoal/70">
                  {duplicates.length} potential duplicate{duplicates.length !== 1 ? "s" : ""} detected
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-primary"
                  onClick={() => {
                    const nextShow = !showDuplicates;
                    setShowDuplicates(nextShow);
                    navigate({
                      to: "/people",
                      search: (prev: Record<string, unknown>) => ({
                        ...prev,
                        hygiene: nextShow ? "duplicates" : undefined,
                      }),
                    });
                  }}
                >
                  {showDuplicates ? "Hide" : "Review"}
                </Button>
              </div>
              {showDuplicates && (
                <div className="space-y-2">
                  {duplicates.map((d, i) => (
                    <div key={i} className="flex items-center justify-between rounded border px-3 py-2 text-sm">
                      <div className="flex items-center gap-2">
                        <Link to="/people/$personId" params={{ personId: d.person1Id }} className="text-primary hover:underline">
                          {d.person1Name}
                        </Link>
                        <span className="text-muted-foreground">{"\u2194"}</span>
                        <Link to="/people/$personId" params={{ personId: d.person2Id }} className="text-primary hover:underline">
                          {d.person2Name}
                        </Link>
                        <span className="text-xs text-muted-foreground">({Math.round(d.confidence * 100)}%)</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="text-xs text-muted-foreground">{d.reason}</span>
                        {d.confidence >= 0.6 ? (
                          <AlertDialog>
                            <AlertDialogTrigger asChild>
                              <Button variant="ghost" size="sm" className="text-xs">
                                Merge
                              </Button>
                            </AlertDialogTrigger>
                            <AlertDialogContent>
                              <AlertDialogHeader>
                                <AlertDialogTitle>Merge people</AlertDialogTitle>
                                <AlertDialogDescription>
                                  Merge <strong>{d.person2Name}</strong> into <strong>{d.person1Name}</strong>?
                                  This will consolidate their meeting history, entity links, and captures.
                                </AlertDialogDescription>
                              </AlertDialogHeader>
                              <AlertDialogFooter>
                                <AlertDialogCancel>Cancel</AlertDialogCancel>
                                <AlertDialogAction
                                  onClick={async () => {
                                    try {
                                      await invoke("merge_people", {
                                        keepId: d.person1Id,
                                        removeId: d.person2Id,
                                      });
                                      loadPeople();
                                      loadDuplicates();
                                    } catch (err) {
                                      console.error("Merge failed:", err);
                                    }
                                  }}
                                >
                                  Merge
                                </AlertDialogAction>
                              </AlertDialogFooter>
                            </AlertDialogContent>
                          </AlertDialog>
                        ) : (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="text-xs"
                            onClick={() => {
                              navigate({
                                to: "/people/$personId",
                                params: { personId: d.person1Id },
                              });
                            }}
                          >
                            Review
                          </Button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* People list */}
          <div>
            {isArchived ? (
              filteredArchived.length === 0 ? (
                <SectionEmpty
                  icon={Users}
                  {...getPersonalityCopy("people-archived-empty", personality)}
                />
              ) : (
                filteredArchived.map((person) => (
                  <ArchivedPersonRow key={person.id} person={person} />
                ))
              )
            ) : sorted.length === 0 ? (
              <SectionEmpty
                icon={Users}
                {...getPersonalityCopy("people-no-matches", personality)}
              />
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
          {person.accountNames ?? person.organization}
          {(person.accountNames ?? person.organization) && person.role && " \u00B7 "}
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

/** I176: Simplified row for archived people (no temperature/trend signals). */
function ArchivedPersonRow({ person }: { person: PersonListItem }) {
  return (
    <ListRow
      to="/people/$personId"
      params={{ personId: person.id }}
      name={person.name}
      subtitle={
        [
          person.email,
          person.accountNames ?? person.organization,
        ]
          .filter(Boolean)
          .join(" \u00B7 ") || undefined
      }
      columns={
        person.relationship !== "unknown" ? (
          <ListColumn value={person.relationship} className="w-16" />
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

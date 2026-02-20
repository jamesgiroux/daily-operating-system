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
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import { formatRelativeDate } from "@/lib/utils";
import {
  EntityListSkeleton,
  EntityListError,
  EntityListEmpty,
  EntityListHeader,
  EntityListEndMark,
  ArchiveToggle,
  FilterTabs,
} from "@/components/entity/EntityListShell";
import { EntityRow } from "@/components/entity/EntityRow";
import type { PersonListItem, DuplicateCandidate } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";

type ArchiveTab = "active" | "archived";
type RelationshipTab = "all" | "external" | "internal" | "unknown";
type HygieneFilter = "unnamed" | "duplicates";

const relationshipTabs: readonly RelationshipTab[] = ["all", "external", "internal", "unknown"];

const tempDotColor: Record<string, string> = {
  hot: "var(--color-garden-sage)",
  warm: "var(--color-spice-turmeric)",
  cool: "var(--color-paper-linen)",
  cold: "var(--color-spice-terracotta)",
};

const tempOrder: Record<string, number> = {
  hot: 0,
  warm: 1,
  cool: 2,
  cold: 3,
};

function parseRelationshipTab(value: unknown): RelationshipTab {
  if (value === "external" || value === "internal" || value === "unknown") return value;
  return "all";
}

function parseHygieneFilter(value: unknown): HygieneFilter | undefined {
  if (value === "unnamed" || value === "duplicates") return value;
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
      .catch((err) => {
        console.error("get_duplicate_people failed:", err);
        setDuplicates([]);
      });
  }, []);

  useEffect(() => { loadDuplicates(); }, [loadDuplicates]);

  useEffect(() => {
    setTab(parseRelationshipTab(search.relationship));
  }, [search.relationship]);

  useEffect(() => {
    if (activeHygieneFilter === "duplicates") setShowDuplicates(true);
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
    return () => { unlisten.then((f) => f()); };
  }, [archiveTab, loadPeople, loadArchivedPeople]);

  // Filters
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

  const filteredArchived = searchQuery
    ? archivedPeople.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          p.email.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.organization ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedPeople;

  const isArchived = archiveTab === "archived";
  const showRelationship = tab === "all";

  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // FolioBar stats
  const folioStats = useMemo((): ReadinessStat[] => {
    const stats: ReadinessStat[] = [];
    if (people.length > 0) stats.push({ label: `${people.length} contacts`, color: "sage" });
    if (duplicates.length > 0) stats.push({ label: `${duplicates.length} duplicates`, color: "terracotta" });
    return stats;
  }, [people.length, duplicates.length]);

  const folioAddButton = useMemo(() => (
    <button
      onClick={() => setShowAddForm(true)}
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase" as const,
        color: "var(--color-garden-larkspur)",
        background: "none",
        border: "1px solid var(--color-garden-larkspur)",
        borderRadius: 4,
        padding: "2px 10px",
        cursor: "pointer",
      }}
    >
      + Add
    </button>
  ), []);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "People",
      atmosphereColor: "larkspur" as const,
      activePage: "people" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioStats,
      folioActions: isArchived ? undefined : folioAddButton,
    }),
    [formattedDate, folioStats, isArchived, folioAddButton],
  );
  useRegisterMagazineShell(shellConfig);

  // Loading
  if (loading && (isArchived ? archivedPeople.length === 0 : people.length === 0)) {
    return <EntityListSkeleton />;
  }

  // Error
  if (error) {
    return <EntityListError error={error} onRetry={isArchived ? loadArchivedPeople : loadPeople} />;
  }

  // Empty
  if (!isArchived && people.length === 0) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        <h1 style={{ fontFamily: "var(--font-serif)", fontSize: 36, fontWeight: 400, letterSpacing: "-0.02em", color: "var(--color-text-primary)", margin: "0 0 24px 0" }}>
          The Room
        </h1>
        <EntityListEmpty
          title={getPersonalityCopy("people-empty", personality).title}
          message={getPersonalityCopy("people-empty", personality).message}
        />
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      <EntityListHeader
        headline="The Room"
        count={isArchived ? filteredArchived.length : filtered.length}
        countLabel="contacts"
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder="⌘  Search people..."
      >
        <ArchiveToggle archiveTab={archiveTab} onTabChange={setArchiveTab} />
        {!isArchived && (
          <FilterTabs
            tabs={relationshipTabs}
            active={tab}
            onChange={(t) => {
              setTab(t);
              navigate({
                to: "/people",
                search: (prev: Record<string, unknown>) => ({
                  ...prev,
                  relationship: t === "all" ? undefined : t,
                }),
              });
            }}
          />
        )}
      </EntityListHeader>

      {/* Add person form */}
      {!isArchived && showAddForm && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            padding: "12px 0",
            borderBottom: "1px solid var(--color-rule-heavy)",
            marginBottom: 8,
          }}
        >
          <input
            type="email"
            autoFocus
            value={newEmail}
            onChange={(e) => setNewEmail(e.target.value)}
            placeholder="Email"
            onKeyDown={(e) => {
              if (e.key === "Escape") { setShowAddForm(false); setNewEmail(""); setNewName(""); }
            }}
            style={{ flex: 1, fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)", background: "none", border: "none", borderBottom: "1px solid var(--color-rule-light)", padding: "4px 0", outline: "none" }}
          />
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Name"
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreatePerson();
              if (e.key === "Escape") { setShowAddForm(false); setNewEmail(""); setNewName(""); }
            }}
            style={{ flex: 1, fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)", background: "none", border: "none", borderBottom: "1px solid var(--color-rule-light)", padding: "4px 0", outline: "none" }}
          />
          <button
            onClick={handleCreatePerson}
            disabled={creating || !newEmail.trim() || !newName.trim()}
            style={{
              fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600,
              color: (!newEmail.trim() || !newName.trim()) ? "var(--color-text-tertiary)" : "var(--color-garden-larkspur)",
              background: "none", border: "1px solid", borderColor: (!newEmail.trim() || !newName.trim()) ? "var(--color-rule-heavy)" : "var(--color-garden-larkspur)",
              borderRadius: 4, padding: "3px 12px", cursor: (!newEmail.trim() || !newName.trim()) ? "default" : "pointer",
            }}
          >
            {creating ? "Creating..." : "Create"}
          </button>
          <button
            onClick={() => { setShowAddForm(false); setNewEmail(""); setNewName(""); }}
            style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", background: "none", border: "none", cursor: "pointer", padding: 0 }}
          >
            Cancel
          </button>
        </div>
      )}

      {/* Hygiene banner: unnamed filter */}
      {!isArchived && activeHygieneFilter === "unnamed" && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderLeft: "3px solid var(--color-spice-turmeric)",
            background: "rgba(201, 162, 39, 0.06)",
            borderRadius: 8,
            padding: "10px 16px",
            marginBottom: 16,
          }}
        >
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
            Showing people with likely placeholder names.
          </span>
          <button
            onClick={() => navigate({ to: "/people", search: (prev: Record<string, unknown>) => ({ ...prev, hygiene: undefined }) })}
            style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer" }}
          >
            Clear
          </button>
        </div>
      )}

      {/* Duplicate detection banner */}
      {duplicates.length > 0 && !isArchived && (
        <div style={{ marginBottom: 16 }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              borderLeft: "3px solid var(--color-spice-turmeric)",
              background: "rgba(201, 162, 39, 0.06)",
              borderRadius: 8,
              padding: "10px 16px",
            }}
          >
            <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
              {duplicates.length} potential duplicate{duplicates.length !== 1 ? "s" : ""} detected
            </span>
            <button
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
              style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer" }}
            >
              {showDuplicates ? "Hide" : "Review"}
            </button>
          </div>

          {showDuplicates && (
            <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 8 }}>
              {duplicates.map((d, i) => (
                <div
                  key={i}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "8px 12px",
                    borderBottom: "1px solid var(--color-rule-light)",
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Link to="/people/$personId" params={{ personId: d.person1Id }} style={{ color: "var(--color-garden-larkspur)", textDecoration: "none" }}>
                      {d.person1Name}
                    </Link>
                    <span style={{ color: "var(--color-text-tertiary)" }}>{"\u2194"}</span>
                    <Link to="/people/$personId" params={{ personId: d.person2Id }} style={{ color: "var(--color-garden-larkspur)", textDecoration: "none" }}>
                      {d.person2Name}
                    </Link>
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                      ({Math.round(d.confidence * 100)}%)
                    </span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                      {d.reason}
                    </span>
                    {d.confidence >= 0.6 ? (
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <button style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-spice-turmeric)", background: "none", border: "none", cursor: "pointer" }}>
                            Merge
                          </button>
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
                                  await invoke("merge_people", { keepId: d.person1Id, removeId: d.person2Id });
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
                      <button
                        onClick={() => navigate({ to: "/people/$personId", params: { personId: d.person1Id } })}
                        style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", background: "none", border: "none", cursor: "pointer" }}
                      >
                        Review
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* People rows */}
      <section>
        {isArchived ? (
          filteredArchived.length === 0 ? (
            <EntityListEmpty
              title={getPersonalityCopy("people-archived-empty", personality).title}
              message={getPersonalityCopy("people-archived-empty", personality).message ?? ""}
            />
          ) : (
            <div style={{ display: "flex", flexDirection: "column" }}>
              {filteredArchived.map((person, i) => (
                <ArchivedPersonRow key={person.id} person={person} showBorder={i < filteredArchived.length - 1} />
              ))}
            </div>
          )
        ) : sorted.length === 0 ? (
          <EntityListEmpty
            title={getPersonalityCopy("people-no-matches", personality).title}
            message={getPersonalityCopy("people-no-matches", personality).message ?? ""}
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {sorted.map((person, i) => (
              <PersonRow key={person.id} person={person} showRelationship={showRelationship} showBorder={i < sorted.length - 1} />
            ))}
          </div>
        )}
      </section>

      {/* End mark */}
      {((isArchived && filteredArchived.length > 0) || (!isArchived && sorted.length > 0)) && (
        <EntityListEndMark text="That's everyone." />
      )}
    </div>
  );
}

// ─── Person Row ─────────────────────────────────────────────────────────────

function PersonRow({
  person,
  showRelationship,
  showBorder,
}: {
  person: PersonListItem;
  showRelationship: boolean;
  showBorder: boolean;
}) {
  const trendArrow =
    person.trend === "increasing" ? (
      <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-garden-sage)" }}>{"\u25B2"}</span>
    ) : person.trend === "decreasing" ? (
      <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-spice-terracotta)" }}>{"\u25BC"}</span>
    ) : null;

  const lastSeen = person.lastSeen ? formatRelativeDate(person.lastSeen) : null;

  const nameSuffix = (
    <>
      {trendArrow}
      {showRelationship && person.relationship !== "unknown" && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: person.relationship === "external"
              ? "var(--color-garden-larkspur)"
              : "var(--color-text-tertiary)",
          }}
        >
          {person.relationship}
        </span>
      )}
    </>
  );

  const subtitle = (
    <>
      {person.accountNames ?? person.organization}
      {(person.accountNames ?? person.organization) && person.role && " \u00B7 "}
      {person.role}
    </>
  );

  return (
    <EntityRow
      to="/people/$personId"
      params={{ personId: person.id }}
      dotColor={tempDotColor[person.temperature] ?? "var(--color-paper-linen)"}
      name={person.name}
      showBorder={showBorder}
      nameSuffix={nameSuffix}
      subtitle={subtitle}
    >
      {lastSeen && (
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
          {lastSeen}
        </span>
      )}
    </EntityRow>
  );
}

// ─── Archived Person Row ────────────────────────────────────────────────────

function ArchivedPersonRow({ person, showBorder }: { person: PersonListItem; showBorder: boolean }) {
  const subtitle = [
    person.email,
    person.accountNames ?? person.organization,
  ].filter(Boolean).join(" \u00B7 ");

  return (
    <EntityRow
      to="/people/$personId"
      params={{ personId: person.id }}
      dotColor="var(--color-paper-linen)"
      name={person.name}
      showBorder={showBorder}
      subtitle={subtitle || undefined}
    >
      {person.relationship !== "unknown" && (
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
          {person.relationship}
        </span>
      )}
    </EntityRow>
  );
}

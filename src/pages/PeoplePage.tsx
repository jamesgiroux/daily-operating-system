import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useTauriEvent } from "@/hooks/useTauriEvent";
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
import {
  EntityListSkeleton,
  EntityListError,
  EntityListEmpty,
  EntityListHeader,
  EntityListEndMark,
  ArchiveToggle,
  FilterTabs,
} from "@/components/entity/EntityListShell";
import shellStyles from "@/components/entity/EntityListShell.module.css";
import s from "./PeoplePage.module.css";
import { EditorialPageHeader } from "@/components/editorial/EditorialPageHeader";
import { EntityRow } from "@/components/entity/EntityRow";
import { EmptyState } from "@/components/editorial/EmptyState";
import { Avatar } from "@/components/ui/Avatar";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { PersonListItem, DuplicateCandidate } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";

type ArchiveTab = "active" | "archived";
type RelationshipTab = "all" | "external" | "internal" | "unknown";
type HygieneFilter = "unnamed" | "duplicates";

const relationshipTabs: readonly RelationshipTab[] = ["all", "external", "internal", "unknown"];

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
        console.error("get_duplicate_people failed:", err); // Expected: background data fetch on mount
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

  const onPeopleUpdated = useCallback(() => {
    if (archiveTab === "active") {
      loadPeople();
    } else {
      loadArchivedPeople();
    }
  }, [archiveTab, loadPeople, loadArchivedPeople]);
  useTauriEvent("people-updated", onPeopleUpdated);

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

  // Group people by relationship when showing "all" tab
  const PEOPLE_SECTIONS: { type: string; title: string }[] = [
    { type: "external", title: "Your Contacts" },
    { type: "internal", title: "Your Team" },
    { type: "unknown", title: "Unclassified" },
  ];

  const groupedPeople = useMemo(() => {
    if (!showRelationship) return null; // flat list when filtered to one tab
    const groups: Record<string, PersonListItem[]> = {
      external: [], internal: [], unknown: [],
    };
    for (const person of sorted) {
      const rel = person.relationship ?? "unknown";
      if (groups[rel]) groups[rel].push(person);
      else groups.unknown.push(person);
    }
    return groups;
  }, [sorted, showRelationship]);

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
      className={s.folioAddButton}
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
      <div className={shellStyles.pageShell}>
        <EditorialPageHeader title="The Room" scale="standard" width="standard" />
        {(() => {
          const copy = getPersonalityCopy("people-empty", personality);
          return (
            <EmptyState
              headline={copy.title}
              explanation={copy.explanation ?? copy.message ?? ""}
              benefit={copy.benefit}
              action={{ label: "Connect Google", onClick: () => navigate({ to: "/settings", search: { tab: "connectors" } }) }}
            />
          );
        })()}
      </div>
    );
  }

  return (
    <div className={shellStyles.pageShell}>
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
        <div className={s.addForm}>
          <input
            type="email"
            autoFocus
            value={newEmail}
            onChange={(e) => setNewEmail(e.target.value)}
            placeholder="Email"
            onKeyDown={(e) => {
              if (e.key === "Escape") { setShowAddForm(false); setNewEmail(""); setNewName(""); }
            }}
            className={s.addFormInput}
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
            className={s.addFormInput}
          />
          <button
            onClick={handleCreatePerson}
            disabled={creating || !newEmail.trim() || !newName.trim()}
            className={s.addFormCreate}
          >
            {creating ? "Creating..." : "Create"}
          </button>
          <button
            onClick={() => { setShowAddForm(false); setNewEmail(""); setNewName(""); }}
            className={s.addFormCancel}
          >
            Cancel
          </button>
        </div>
      )}

      {/* Hygiene banner: unnamed filter */}
      {!isArchived && activeHygieneFilter === "unnamed" && (
        <div className={`${s.banner} ${s.bannerStandalone}`}>
          <span className={s.bannerText}>
            Showing people with likely placeholder names.
          </span>
          <button
            onClick={() => navigate({ to: "/people", search: (prev: Record<string, unknown>) => ({ ...prev, hygiene: undefined }) })}
            className={s.bannerAction}
          >
            Clear
          </button>
        </div>
      )}

      {/* Duplicate detection banner */}
      {duplicates.length > 0 && !isArchived && (
        <div className={s.duplicatesContainer}>
          <div className={s.banner}>
            <span className={s.bannerText}>
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
              className={s.bannerAction}
            >
              {showDuplicates ? "Hide" : "Review"}
            </button>
          </div>

          {showDuplicates && (
            <div className={s.duplicatesList}>
              {duplicates.map((d, i) => (
                <div key={i} className={s.duplicateRow}>
                  <div className={s.duplicateNames}>
                    <Link to="/people/$personId" params={{ personId: d.person1Id }} className={s.duplicateLink}>
                      {d.person1Name}
                    </Link>
                    <span className={s.duplicateSeparator}>{"\u2194"}</span>
                    <Link to="/people/$personId" params={{ personId: d.person2Id }} className={s.duplicateLink}>
                      {d.person2Name}
                    </Link>
                    <span className={s.duplicateConfidence}>
                      ({Math.round(d.confidence * 100)}%)
                    </span>
                  </div>
                  <div className={s.duplicateActions}>
                    <span className={s.duplicateReason}>
                      {d.reason}
                    </span>
                    {d.confidence >= 0.6 ? (
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <button className={s.bannerAction}>
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
                                  toast.error("Failed to merge contacts");
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
                        className={s.duplicateReviewButton}
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
            <div className={s.personList}>
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
        ) : groupedPeople ? (
          /* Grouped view: external → internal → unknown */
          <div className={s.personList}>
            {PEOPLE_SECTIONS.map(({ type, title }) => {
              const sectionPeople = groupedPeople[type] ?? [];
              if (sectionPeople.length === 0) return null;
              return (
                <div key={type}>
                  <ChapterHeading title={title} />
                  {sectionPeople.map((person, i) => (
                    <PersonRow key={person.id} person={person} showRelationship={false} showBorder={i < sectionPeople.length - 1} />
                  ))}
                </div>
              );
            })}
          </div>
        ) : (
          /* Flat view: single relationship tab selected */
          <div className={s.personList}>
            {sorted.map((person, i) => (
              <PersonRow key={person.id} person={person} showRelationship={false} showBorder={i < sorted.length - 1} />
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

const relationshipRingColor: Record<string, string> = {
  external: "var(--color-spice-turmeric)",
  internal: "var(--color-garden-larkspur)",
  unknown: "var(--color-paper-linen)",
};

function PersonRow({
  person,
  showRelationship,
  showBorder,
}: {
  person: PersonListItem;
  showRelationship: boolean;
  showBorder: boolean;
}) {
  const nameSuffix = showRelationship && person.relationship !== "unknown" ? (
    <span
      className={`${s.relationshipBadge} ${person.relationship === "external" ? s.relationshipBadgeExternal : ""}`}
    >
      {person.relationship}
    </span>
  ) : undefined;

  const subtitle = (
    <>
      {person.accountNames ?? person.organization}
      {(person.accountNames ?? person.organization) && person.role && " \u00B7 "}
      {person.role}
    </>
  );

  const ringColor = relationshipRingColor[person.relationship] ?? "var(--color-paper-linen)";

  const avatar = (
    <div
      className={s.avatarRing}
      style={{ "--avatar-ring-color": ringColor } as React.CSSProperties}
    >
      <Avatar name={person.name} personId={person.id} photoUrl={person.photoUrl ?? undefined} size={26} />
    </div>
  );

  return (
    <EntityRow
      to="/people/$personId"
      params={{ personId: person.id }}
      dotColor="var(--color-garden-larkspur)"
      name={person.name}
      showBorder={showBorder}
      nameSuffix={nameSuffix}
      subtitle={subtitle}
      avatar={avatar}
    />
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
        <span className={s.archivedRelationshipLabel}>
          {person.relationship}
        </span>
      )}
    </EntityRow>
  );
}

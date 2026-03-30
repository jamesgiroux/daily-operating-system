/**
 * StakeholderGallery — People chapter.
 * 2-column grid of stakeholder cards with editable engagement badges.
 * Falls back to linkedPeople when no intelligence stakeholders exist.
 * Includes an optional "Your Team" strip for account team members.
 * Generalized: configurable title/id, accountTeam optional.
 *
 * I261: Live editing (name, role, assessment, engagement), add/remove
 * stakeholders, internal people filter, create contact from stakeholder.
 *
 * I493: Enriched cards show title/organization from linked person data,
 * engagement badges, last interaction date, and coverage analysis strip.
 */
import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";
import { Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { X, Plus, UserPlus, Search, LinkIcon, Check, Award } from "lucide-react";
import type { EntityIntelligence, StakeholderInsight, Person, AccountTeamMember, StakeholderFull } from "@/types";
import { formatRelativeDate } from "@/lib/utils";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { Avatar } from "@/components/ui/Avatar";
import { EngagementSelector } from "./EngagementSelector";
import { getEngagementDisplay } from "./EngagementSelector";
import { TeamRoleSelector, getTeamRoleDisplay } from "./TeamRoleSelector";
import css from "./StakeholderGallery.module.css";

interface StakeholderGalleryProps {
  intelligence: EntityIntelligence | null;
  linkedPeople: Person[];
  accountTeam?: AccountTeamMember[];
  /** DB-first stakeholder read model — primary display source when non-empty. */
  stakeholdersFull?: StakeholderFull[];
  sectionId?: string;
  chapterTitle?: string;
  /** Entity ID for intelligence updates. */
  entityId?: string;
  /** Entity type for intelligence updates. */
  entityType?: string;
  /** Called after any intelligence field is updated (for parent re-fetch). */
  onIntelligenceUpdated?: () => void;
  /** Team edit callbacks — when provided, enables inline team editing. */
  onRemoveTeamMember?: (personId: string, role: string) => void;
  onChangeTeamRole?: (personId: string, newRole: string) => void;
  onAddTeamMember?: (personId: string, role: string) => void;
  onCreateTeamMember?: (name: string, email: string, role: string) => void;
  teamSearchQuery?: string;
  onTeamSearchQueryChange?: (query: string) => void;
  teamSearchResults?: Person[];
}

function buildEpigraph(stakeholders: { name: string }[]): string {
  const count = stakeholders.length;
  if (count === 0) return "";
  const numberWords: Record<number, string> = {
    1: "One", 2: "Two", 3: "Three", 4: "Four", 5: "Five",
    6: "Six", 7: "Seven", 8: "Eight", 9: "Nine", 10: "Ten",
    11: "Eleven", 12: "Twelve",
  };
  const word = numberWords[count] ?? String(count);
  const noun = count === 1 ? "stakeholder shapes" : "stakeholders shape";
  return `${word} ${noun} this relationship across the organization.`;
}

/** Filter out internal people from stakeholder list. */
function filterInternalStakeholders(
  stakeholders: StakeholderInsight[],
  linkedPeople: Person[],
): StakeholderInsight[] {
  const internalNames = new Set(
    linkedPeople
      .filter((p) => p.relationship === "internal")
      .map((p) => p.name.toLowerCase()),
  );
  if (internalNames.size === 0) return stakeholders;
  return stakeholders.filter((s) => !internalNames.has(s.name.toLowerCase()));
}

const ASSESSMENT_CHAR_LIMIT = 150;

function TruncatedAssessment({ text }: { text: string }) {
  const [showFull, setShowFull] = useState(false);
  const truncated = text.length > ASSESSMENT_CHAR_LIMIT && !showFull;
  const displayText = truncated ? text.slice(0, ASSESSMENT_CHAR_LIMIT) + "\u2026" : text;
  return (
    <p className={css.assessment}>
      {displayText}
      {truncated && (
        <button
          onClick={(e) => { e.preventDefault(); e.stopPropagation(); setShowFull(true); }}
          className={css.readMore}
        >
          Read more
        </button>
      )}
    </p>
  );
}

/** Team add form — search input + role selector with portal dropdown for results. */
function TeamAddForm({
  teamSearchQuery,
  onTeamSearchQueryChange,
  teamNewRole,
  setTeamNewRole,
  teamNewEmail: _teamNewEmail,
  setTeamNewEmail: _setTeamNewEmail,
  teamSearchResults,
  onClose,
  onAddTeamMember,
  onCreateTeamMember,
}: {
  teamSearchQuery: string;
  onTeamSearchQueryChange?: (q: string) => void;
  teamNewRole: string;
  setTeamNewRole: (r: string) => void;
  teamNewEmail: string;
  setTeamNewEmail: (e: string) => void;
  teamSearchResults: Person[];
  onClose: () => void;
  onAddTeamMember: (personId: string) => void;
  onCreateTeamMember?: (query: string) => void;
}) {
  const inputRef = useRef<HTMLDivElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0, width: 0 });

  const updatePos = useCallback(() => {
    if (!inputRef.current) return;
    const rect = inputRef.current.getBoundingClientRect();
    setPos({ top: rect.bottom + 4, left: rect.left, width: Math.max(rect.width, 320) });
  }, []);

  const hasResults = teamSearchResults.length > 0;
  const hasQuery = teamSearchQuery.trim().length >= 2;
  const showDropdown = hasQuery && (hasResults || !!onCreateTeamMember);

  useEffect(() => {
    if (!showDropdown) return;
    updatePos();

    function handleClickOutside(e: MouseEvent) {
      const target = e.target as Node;
      if (
        inputRef.current && !inputRef.current.contains(target) &&
        dropdownRef.current && !dropdownRef.current.contains(target)
      ) {
        // Don't close the whole form, just let the dropdown disappear naturally
      }
    }

    function handleScroll() { updatePos(); }

    document.addEventListener("mousedown", handleClickOutside);
    window.addEventListener("scroll", handleScroll, true);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      window.removeEventListener("scroll", handleScroll, true);
    };
  }, [showDropdown, updatePos]);

  return (
    <div className={css.teamAddForm}>
      <div ref={inputRef} className={css.searchInputWrapper}>
        <Search size={12} className={css.searchIcon} />
        <input
          value={teamSearchQuery}
          onChange={(e) => onTeamSearchQueryChange?.(e.target.value)}
          placeholder="Search people"
          autoFocus
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
          }}
          className={css.searchInput}
        />
      </div>
      <TeamRoleSelector value={teamNewRole} onChange={setTeamNewRole} />
      <button onClick={onClose} className={css.teamAddCancel}>Cancel</button>

      {showDropdown && createPortal(
        <div
          ref={dropdownRef}
          className={css.teamSearchDropdownPortal}
          style={{ top: pos.top, left: pos.left, width: pos.width }}
        >
          {hasResults && (
            <p className={css.searchResultsLabel}>Existing People</p>
          )}
          {teamSearchResults.map((person) => (
            <button
              key={person.id}
              onClick={() => onAddTeamMember(person.id)}
              className={css.searchResultItem}
            >
              <UserPlus size={14} className={css.searchResultIcon} />
              <div>
                <span className={css.searchResultName}>{person.name}</span>
                {(person.role || person.organization) && (
                  <span className={css.searchResultMeta}>
                    {[person.role, person.organization].filter(Boolean).join(" \u00b7 ")}
                  </span>
                )}
              </div>
            </button>
          ))}
          {onCreateTeamMember && (
            <button
              onClick={() => onCreateTeamMember(teamSearchQuery.trim())}
              className={css.searchResultItem}
            >
              <Plus size={14} className={css.searchResultIcon} />
              <span className={css.searchResultName}>
                Create &ldquo;{teamSearchQuery.trim()}&rdquo;
              </span>
            </button>
          )}
        </div>,
        document.body,
      )}
    </div>
  );
}

/** Build a title/organization line from enriched person data. */
function buildPersonDetail(matched: Person | undefined): string | null {
  if (!matched) return null;
  const parts: string[] = [];
  // Use role as title (Person.role is the job title field)
  if (matched.role) parts.push(matched.role);
  if (matched.organization) parts.push(matched.organization);
  return parts.length > 0 ? parts.join(" \u00b7 ") : null;
}

export function StakeholderGallery({
  intelligence,
  linkedPeople,
  accountTeam,
  stakeholdersFull,
  sectionId = "the-room",
  chapterTitle = "The Room",
  entityId,
  entityType,
  onIntelligenceUpdated,
  onRemoveTeamMember,
  onChangeTeamRole,
  onAddTeamMember,
  onCreateTeamMember,
  teamSearchQuery,
  onTeamSearchQueryChange,
  teamSearchResults,
}: StakeholderGalleryProps) {
  const navigate = useNavigate();

  // DB-first: use stakeholdersFull as primary source when available
  const useDbFirst = (stakeholdersFull?.length ?? 0) > 0;

  const allStakeholders = intelligence?.stakeholderInsights ?? [];
  const stakeholders = useDbFirst ? [] : filterInternalStakeholders(allStakeholders, linkedPeople);
  const hasStakeholders = useDbFirst || stakeholders.length > 0;
  const epigraphSource = useDbFirst
    ? (stakeholdersFull ?? []).map((s) => ({ name: s.personName }))
    : stakeholders;
  const epigraph = hasStakeholders ? buildEpigraph(epigraphSource) : undefined;
  const teamMembers = accountTeam ?? [];
  const canEdit = !!entityId && !!entityType;
  const canEditTeam = !!onRemoveTeamMember;

  // teamEditMode removed — always inline-editable (no pencil toggle)
  const [teamAddingMember, setTeamAddingMember] = useState(false);
  const [teamNewRole, setTeamNewRole] = useState("associated");
  const [teamNewEmail, setTeamNewEmail] = useState("");

  const [expandedGrid, setExpandedGrid] = useState(false);
  const [addingStakeholder, setAddingStakeholder] = useState(false);
  const [newName, setNewName] = useState("");
  const [newRole, setNewRole] = useState("");
  const [hoveredCard, setHoveredCard] = useState<number | null>(null);
  const [searchResults, setSearchResults] = useState<Person[]>([]);
  const [showDropdown, setShowDropdown] = useState(false);
  const searchTimeout = useRef<ReturnType<typeof setTimeout>>();
  const addContainerRef = useRef<HTMLDivElement>(null);

  // Close dropdown on click outside
  useEffect(() => {
    if (!showDropdown) return;
    function handleClickOutside(e: MouseEvent) {
      if (addContainerRef.current && !addContainerRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showDropdown]);

  // Empty section collapse: return null when nothing to show (and not editing)
  if (!hasStakeholders && !useDbFirst && linkedPeople.length === 0 && teamMembers.length === 0 && !canEdit && !canEditTeam) {
    return null;
  }

  const STAKEHOLDER_LIMIT = 6;
  const dbStakeholders = stakeholdersFull ?? [];
  const visibleDbStakeholders = expandedGrid ? dbStakeholders : dbStakeholders.slice(0, STAKEHOLDER_LIMIT);
  const hasMoreDbStakeholders = dbStakeholders.length > STAKEHOLDER_LIMIT && !expandedGrid;
  const visibleStakeholders = expandedGrid ? stakeholders : stakeholders.slice(0, STAKEHOLDER_LIMIT);
  const hasMoreStakeholders = !useDbFirst && stakeholders.length > STAKEHOLDER_LIMIT && !expandedGrid;

  // ── Coverage analysis ──
  const totalKnown = useDbFirst ? dbStakeholders.length : stakeholders.length;
  // For DB-first, count stakeholders where we have intelligence engagement data
  const intelInsights = intelligence?.stakeholderInsights ?? [];
  const engagedCount = useDbFirst
    ? dbStakeholders.filter((s) => {
        const insight = intelInsights.find((i) => (i.personId && i.personId === s.personId) || i.name.toLowerCase() === s.personName.toLowerCase());
        return insight?.engagement && insight.engagement !== "unknown" && insight.engagement !== "none";
      }).length
    : stakeholders.filter(
        (s) => s.engagement && s.engagement !== "unknown" && s.engagement !== "none",
      ).length;

  // ── Field update helper ──
  async function updateField(fieldPath: string, value: string) {
    if (!entityId || !entityType) return;
    try {
      await invoke("update_intelligence_field", {
        entityId,
        entityType,
        fieldPath,
        value,
      });
      onIntelligenceUpdated?.();
    } catch (e) {
      console.error("Failed to update intelligence field:", e);
      toast.error("Failed to save");
    }
  }

  // ── Stakeholders bulk update ──
  async function updateStakeholders(updated: StakeholderInsight[]) {
    if (!entityId || !entityType) return;
    try {
      await invoke("update_stakeholders", {
        entityId,
        entityType,
        stakeholdersJson: JSON.stringify(updated),
      });
      onIntelligenceUpdated?.();
    } catch (e) {
      console.error("Failed to update stakeholders:", e);
      toast.error("Failed to save");
    }
  }

  // ── Remove stakeholder ──
  function handleRemove(index: number) {
    // Find the actual index in allStakeholders (since we filtered)
    const name = stakeholders[index].name;
    const updated = allStakeholders.filter(
      (s) => s.name.toLowerCase() !== name.toLowerCase(),
    );
    updateStakeholders(updated);
  }

  // ── Search people as user types ──
  function handleNameChange(value: string) {
    setNewName(value);
    if (searchTimeout.current) clearTimeout(searchTimeout.current);
    if (value.trim().length < 2) {
      setSearchResults([]);
      setShowDropdown(false);
      return;
    }
    searchTimeout.current = setTimeout(async () => {
      try {
        const results = await invoke<Person[]>("search_people", { query: value.trim() });
        // Filter out people already in the stakeholder list
        const existingNames = new Set(allStakeholders.map((s) => s.name.toLowerCase()));
        const filtered = results.filter((p) => !existingNames.has(p.name.toLowerCase()));
        setSearchResults(filtered.slice(0, 5));
        setShowDropdown(filtered.length > 0);
      } catch {
        setSearchResults([]);
        setShowDropdown(false);
      }
    }, 200);
  }

  // ── Select existing person from search ──
  function handleSelectPerson(person: Person) {
    const newStakeholder: StakeholderInsight = {
      name: person.name,
      role: person.role || newRole.trim() || undefined,
      engagement: "unknown",
    };
    const updated = [...allStakeholders, newStakeholder];
    updateStakeholders(updated);
    // Also link the person to this entity
    if (entityId && entityType) {
      invoke("link_person_entity", { personId: person.id, entityId, relationshipType: "associated" }).catch((err) => {
        console.error("link_person_entity failed:", err); // Expected: best-effort person link
      });
    }
    setNewName("");
    setNewRole("");
    setSearchResults([]);
    setShowDropdown(false);
    setAddingStakeholder(false);
  }

  // ── Add new stakeholder (create new) ──
  function handleAdd() {
    if (!newName.trim()) return;
    const newStakeholder: StakeholderInsight = {
      name: newName.trim(),
      role: newRole.trim() || undefined,
      engagement: "unknown",
    };
    const updated = [...allStakeholders, newStakeholder];
    updateStakeholders(updated);
    setNewName("");
    setNewRole("");
    setSearchResults([]);
    setShowDropdown(false);
    setAddingStakeholder(false);
  }

  // ── Create person entity from stakeholder ──
  async function handleCreateContact(s: StakeholderInsight) {
    if (!entityId || !entityType) return;
    try {
      const personId = await invoke<string>("create_person_from_stakeholder", {
        entityId,
        entityType,
        name: s.name,
        role: s.role ?? null,
      });
      onIntelligenceUpdated?.();
      if (personId) {
        navigate({ to: "/people/$personId", params: { personId } });
      }
    } catch (e) {
      console.error("Failed to create person from stakeholder:", e);
      toast.error("Failed to save");
    }
  }

  // ── Confirm a suggested person link (I420) ──
  async function confirmSuggestion(idx: number, personId: string, canonicalName: string) {
    await updateField(`stakeholderInsights[${idx}].personId`, personId);
    await updateField(`stakeholderInsights[${idx}].name`, canonicalName);
    // suggestedPersonId will be cleared on next enrichment cycle
  }

  // ── Find actual index in allStakeholders for a filtered stakeholder ──
  function actualIndex(filteredIdx: number): number {
    const name = stakeholders[filteredIdx].name.toLowerCase();
    return allStakeholders.findIndex((s) => s.name.toLowerCase() === name);
  }

  return (
    <section id={sectionId || undefined} className={css.section}>
      <ChapterHeading title={chapterTitle} epigraph={epigraph} />

      {useDbFirst ? (
        <>
        <div className={css.grid}>
          {visibleDbStakeholders.map((s) => {
            // Look up supplementary AI assessment from intelligence by matching personId or name
            const insight = intelInsights.find(
              (ins) => (ins.personId && ins.personId === s.personId) || ins.name.toLowerCase() === s.personName.toLowerCase(),
            );
            const personDetail = [s.personRole, s.organization].filter(Boolean).join(" \u00b7 ") || null;
            const isGlean = s.dataSource === "glean";
            const isGoogle = s.dataSource === "google";

            return (
              <div key={s.personId} className={css.card}>
                <div className={css.cardHeader}>
                  <div className={css.avatarRingLinked}>
                    <Avatar name={s.personName} personId={s.personId} size={24} />
                  </div>
                  <Link to="/people/$personId" params={{ personId: s.personId }} className={css.nameLink}>
                    {s.personName}
                  </Link>
                  <LinkIcon size={12} strokeWidth={1.5} className={css.linkIcon} aria-label={`Linked to ${s.personName}`} />
                  {insight?.engagement && canEdit ? (
                    <EngagementSelector
                      value={insight.engagement}
                      onChange={(v) => {
                        const idx = intelInsights.findIndex(
                          (ins) => (ins.personId && ins.personId === s.personId) || ins.name.toLowerCase() === s.personName.toLowerCase(),
                        );
                        if (idx >= 0) updateField(`stakeholderInsights[${idx}].engagement`, v);
                      }}
                    />
                  ) : insight?.engagement ? (
                    <span className={`${css.engagementBadge} ${getEngagementBadgeClass(insight.engagement)}`}>
                      {getStaticBadgeLabel(insight.engagement)}
                    </span>
                  ) : null}
                </div>
                {personDetail && <p className={css.titleLine}>{personDetail}</p>}
                {s.stakeholderRole && s.stakeholderRole !== "associated" && (
                  <p className={css.role}>{s.stakeholderRole}</p>
                )}
                {insight?.assessment && <TruncatedAssessment text={insight.assessment} />}
                {/* Source provenance indicator */}
                {isGlean && (
                  <div className={css.sourceRow}>
                    <span className={css.sourceLabel} data-source="glean">via Glean</span>
                    {entityId && (
                      <>
                        <button
                          onClick={() => {
                            invoke("add_account_team_member", {
                              accountId: entityId,
                              personId: s.personId,
                              role: s.stakeholderRole || "associated",
                            }).then(() => onIntelligenceUpdated?.()).catch((e) => {
                              console.error("Failed to accept stakeholder:", e);
                              toast.error("Failed to accept");
                            });
                          }}
                          className={css.sourceAccept}
                          title="Confirm stakeholder"
                        >
                          <Check size={11} strokeWidth={2} /> Accept
                        </button>
                        <button
                          onClick={() => {
                            invoke("remove_account_team_member", {
                              accountId: entityId,
                              personId: s.personId,
                              role: s.stakeholderRole || "associated",
                            }).then(() => onIntelligenceUpdated?.()).catch((e) => {
                              console.error("Failed to dismiss stakeholder:", e);
                              toast.error("Failed to dismiss");
                            });
                          }}
                          className={css.sourceDismiss}
                          title="Remove stakeholder"
                        >
                          Dismiss
                        </button>
                      </>
                    )}
                  </div>
                )}
                {isGoogle && (
                  <span className={css.sourceLabel} data-source="google">via Google</span>
                )}
                {/* Meeting count + last seen from DB */}
                {(s.meetingCount != null && s.meetingCount > 0) && (
                  <div className={css.lastSeen}>
                    {s.meetingCount} meeting{s.meetingCount === 1 ? "" : "s"}
                    {s.lastSeen ? ` \u00b7 Last seen ${formatRelativeDate(s.lastSeen)}` : ""}
                  </div>
                )}
                {!s.meetingCount && s.lastSeen && (
                  <div className={css.lastSeen}>
                    Last seen {formatRelativeDate(s.lastSeen)}
                  </div>
                )}
              </div>
            );
          })}
        </div>
        {hasMoreDbStakeholders && (
          <button onClick={() => setExpandedGrid(true)} className={css.showMore}>
            Show {dbStakeholders.length - STAKEHOLDER_LIMIT} more
          </button>
        )}
        </>
      ) : hasStakeholders ? (
        <>
        <div className={css.grid}>
          {visibleStakeholders.map((s, i) => {
            // I420: personId-first matching, then name fallback
            const matched = s.personId
              ? linkedPeople.find((p) => p.id === s.personId)
              : linkedPeople.find((p) => p.name.toLowerCase() === s.name.toLowerCase());
            const suggested = !matched && s.suggestedPersonId
              ? linkedPeople.find((p) => p.id === s.suggestedPersonId)
              : null;
            const idx = actualIndex(i);
            const isHovered = hoveredCard === i;
            const personDetail = buildPersonDetail(matched);

            const card = (
              <div
                key={i}
                className={css.card}
                onMouseEnter={() => setHoveredCard(i)}
                onMouseLeave={() => setHoveredCard(null)}
              >
                {/* Remove button (hover) */}
                {canEdit && isHovered && (
                  <button
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleRemove(i);
                    }}
                    className={css.removeButton}
                    title="Remove stakeholder"
                  >
                    <X size={12} strokeWidth={1.5} className={css.removeIcon} />
                  </button>
                )}

                <div className={css.cardHeader}>
                  {/* Avatar with larkspur ring for linked person entities */}
                  <div className={matched ? css.avatarRingLinked : css.avatarRing}>
                    <Avatar name={s.name} personId={matched?.id} size={24} />
                  </div>
                  {canEdit ? (
                    <EditableText
                      value={s.name}
                      onChange={(v) => updateField(`stakeholderInsights[${idx}].name`, v)}
                      multiline={false}
                      className={css.editableName}
                    />
                  ) : matched ? (
                    <Link to="/people/$personId" params={{ personId: matched.id }} className={css.nameLink}>
                      {s.name}
                    </Link>
                  ) : (
                    <span className={css.name}>
                      {s.name}
                    </span>
                  )}
                  {matched && (
                    <LinkIcon size={12} strokeWidth={1.5} className={css.linkIcon} aria-label={`Linked to ${matched.name}`} />
                  )}
                  {s.engagement && canEdit ? (
                    <EngagementSelector
                      value={s.engagement}
                      onChange={(v) => updateField(`stakeholderInsights[${idx}].engagement`, v)}
                    />
                  ) : s.engagement ? (
                    <span
                      className={`${css.engagementBadge} ${getEngagementBadgeClass(s.engagement)}`}
                    >
                      {getStaticBadgeLabel(s.engagement)}
                    </span>
                  ) : null}
                </div>
                {/* I493: Title and organization from linked person data */}
                {personDetail && (
                  <p className={css.titleLine}>{personDetail}</p>
                )}
                {s.role != null && (
                  canEdit ? (
                    <EditableText
                      value={s.role}
                      onChange={(v) => updateField(`stakeholderInsights[${idx}].role`, v)}
                      as="p"
                      multiline={false}
                      className={css.editableRole}
                    />
                  ) : (
                    <p className={css.role}>
                      {s.role}
                    </p>
                  )
                )}
                {s.assessment != null && (
                  canEdit ? (
                    <EditableText
                      value={s.assessment}
                      onChange={(v) => updateField(`stakeholderInsights[${idx}].assessment`, v)}
                      as="p"
                      multiline
                      className={css.editableAssessment}
                    />
                  ) : (
                    <TruncatedAssessment text={s.assessment} />
                  )
                )}
                {s.source && (s.source === "clay" || s.source === "gravatar") && (
                  <span className={css.enrichmentTag} data-source={s.source}>
                    {s.source === "clay" ? "Clay" : "Gravatar"}
                  </span>
                )}
                {/* I493: Last interaction date from linked person data */}
                {matched?.lastSeen && (
                  <div className={css.lastSeen}>
                    Last seen {formatRelativeDate(matched.lastSeen)}
                  </div>
                )}
                {/* I420: Suggestion confirmation prompt */}
                {canEdit && suggested && (
                  <button
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      confirmSuggestion(idx, suggested.id, suggested.name);
                    }}
                    className={css.actionButtonSuggestion}
                    title={`Link to ${suggested.name}`}
                  >
                    <Check size={12} strokeWidth={1.5} />
                    Link to {suggested.name}?
                  </button>
                )}
                {/* Create contact action for unlinked stakeholders */}
                {canEdit && !matched && !suggested && isHovered && (
                  <button
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleCreateContact(s);
                    }}
                    className={css.actionButtonCreate}
                  >
                    <UserPlus size={12} strokeWidth={1.5} />
                    Create contact
                  </button>
                )}
              </div>
            );

            // Card body does NOT navigate — only name click navigates
            return card;
          })}
        </div>
        {hasMoreStakeholders && (
          <button
            onClick={() => setExpandedGrid(true)}
            className={css.showMore}
          >
            Show {stakeholders.length - STAKEHOLDER_LIMIT} more
          </button>
        )}
        </>
      ) : linkedPeople.length > 0 ? (
        <div className={css.grid}>
          {linkedPeople.map((p) => (
            <div key={p.id} className={css.card}>
              <div className={css.cardHeader}>
                <Link to="/people/$personId" params={{ personId: p.id }} className={css.nameLink}>
                  {p.name}
                </Link>
              </div>
              {p.role && (
                <p className={css.titleLine}>
                  {[p.role, p.organization].filter(Boolean).join(" \u00b7 ")}
                </p>
              )}
              {p.lastSeen && (
                <div className={css.lastSeen}>
                  Last seen {formatRelativeDate(p.lastSeen)}
                </div>
              )}
            </div>
          ))}
        </div>
      ) : null}

      {/* Add stakeholder */}
      {canEdit && (
        <div className={hasStakeholders ? css.addSection : css.addSectionCompact}>
          {addingStakeholder ? (
            <div ref={addContainerRef}>
              <div className={css.addForm}>
                <div className={css.searchInputWrapper}>
                  <Search size={12} className={css.searchIcon} />
                  <input
                    value={newName}
                    onChange={(e) => handleNameChange(e.target.value)}
                    placeholder="Search people or type name"
                    autoFocus
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleAdd();
                      if (e.key === "Escape") { setAddingStakeholder(false); setNewName(""); setNewRole(""); setShowDropdown(false); }
                    }}
                    className={css.searchInput}
                  />
                </div>
                <input
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value)}
                  placeholder="Role (optional)"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleAdd();
                    if (e.key === "Escape") { setAddingStakeholder(false); setNewName(""); setNewRole(""); }
                  }}
                  className={css.roleInput}
                />
                <button
                  onClick={handleAdd}
                  disabled={!newName.trim()}
                  className={newName.trim() ? css.addButtonActive : css.addButtonDisabled}
                >
                  Add
                </button>
              </div>

              {/* Inline search results */}
              {showDropdown && searchResults.length > 0 && (
                <div className={css.searchResults}>
                  <p className={css.searchResultsLabel}>
                    Existing People
                  </p>
                  {searchResults.map((person) => (
                    <button
                      key={person.id}
                      onClick={() => handleSelectPerson(person)}
                      className={css.searchResultItem}
                    >
                      <UserPlus size={14} className={css.searchResultIcon} />
                      <div>
                        <span className={css.searchResultName}>
                          {person.name}
                        </span>
                        {(person.role || person.organization) && (
                          <span className={css.searchResultMeta}>
                            {[person.role, person.organization].filter(Boolean).join(" \u00b7 ")}
                          </span>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <button
              onClick={() => setAddingStakeholder(true)}
              className={css.addTrigger}
            >
              <Plus size={12} strokeWidth={1.5} />
              Add Stakeholder
            </button>
          )}
        </div>
      )}

      {/* I493: Coverage analysis strip */}
      {totalKnown > 0 && (
        <div className={css.coverageStrip}>
          <span className={css.coverageNumbers}>{engagedCount} of {totalKnown}</span>
          <span className={css.coverageLabel}>stakeholders with defined engagement</span>
        </div>
      )}

      {/* I557: Relationship Depth Summary */}
      {intelligence?.relationshipDepth && (
        <div className={css.relationshipDepthSection}>
          <div className={css.depthStrip}>
            {intelligence.relationshipDepth.championStrength && (
              <div className={css.depthCell}>
                <div className={css.depthCellLabel}>Champion</div>
                <div className={`${css.depthCellValue} ${getDepthColor("champion", intelligence.relationshipDepth.championStrength)}`}>
                  {intelligence.relationshipDepth.championStrength}
                </div>
              </div>
            )}
            {intelligence.relationshipDepth.executiveAccess && (
              <div className={css.depthCell}>
                <div className={css.depthCellLabel}>Executive Access</div>
                <div className={`${css.depthCellValue} ${getDepthColor("access", intelligence.relationshipDepth.executiveAccess)}`}>
                  {intelligence.relationshipDepth.executiveAccess}
                </div>
              </div>
            )}
            {intelligence.relationshipDepth.stakeholderCoverage && (
              <div className={css.depthCell}>
                <div className={css.depthCellLabel}>Coverage</div>
                <div className={`${css.depthCellValue} ${getDepthColor("coverage", intelligence.relationshipDepth.stakeholderCoverage)}`}>
                  {intelligence.relationshipDepth.stakeholderCoverage}
                </div>
              </div>
            )}
          </div>
          {intelligence.relationshipDepth.coverageGaps && intelligence.relationshipDepth.coverageGaps.length > 0 && (
            <div className={css.depthGapsRow}>
              <span className={css.depthGapsLabel}>Gaps</span>
              <span className={css.depthGapsText}>
                {intelligence.relationshipDepth.coverageGaps.join(" · ")}
              </span>
            </div>
          )}
        </div>
      )}


      {/* I646 C2: Champion designation badge — visible even without AI enrichment */}
      {teamMembers.filter((m) => m.role?.toLowerCase().includes("champion")).length > 0 &&
        !intelligence?.relationshipDepth?.championStrength && (
        <div className={css.championBadgeRow}>
          {teamMembers
            .filter((m) => m.role?.toLowerCase().includes("champion"))
            .map((champ) => (
              <div className={css.championBadge} key={champ.personId}>
                <Award size={14} />
                <Link
                  to="/people/$personId"
                  params={{ personId: champ.personId }}
                  className={css.championBadgeLink}
                >
                  {champ.personName}
                </Link>
                <span className={css.championBadgeLabel}>Champion</span>
              </div>
            ))}
        </div>
      )}

      {/* Your Team — compact clickable chips with inline editing */}
      {(teamMembers.length > 0 || canEditTeam) && (
        <div className={css.teamChipsSection}>
          <div className={css.teamHeader}>
            <span className={css.teamLabel}>Your Team</span>
          </div>
          <div className={css.teamChips}>
            {teamMembers.map((member) => (
              <div key={member.personId} className={css.teamChipEditable}>
                <Link
                  to="/people/$personId"
                  params={{ personId: member.personId }}
                  className={css.teamChipNameLink}
                >
                  {member.personName}
                </Link>
                {canEditTeam && onChangeTeamRole ? (
                  <TeamRoleSelector
                    value={member.role}
                    onChange={(newRole) => onChangeTeamRole(member.personId, newRole)}
                  />
                ) : member.role ? (
                  <span className={css.teamChipRole}>{getTeamRoleDisplay(member.role)}</span>
                ) : null}
                {canEditTeam && onRemoveTeamMember && (
                  <button
                    onClick={() => onRemoveTeamMember(member.personId, member.role)}
                    className={css.teamChipRemove}
                    title="Remove from team"
                  >
                    <X size={10} strokeWidth={2} />
                  </button>
                )}
              </div>
            ))}
            {canEditTeam && !teamAddingMember && (
              <button
                onClick={() => setTeamAddingMember(true)}
                className={css.teamAddTrigger}
              >
                <Plus size={10} strokeWidth={2} />
                Add
              </button>
            )}
          </div>

          {/* Inline add member form */}
          {canEditTeam && teamAddingMember && (
            <TeamAddForm
              teamSearchQuery={teamSearchQuery ?? ""}
              onTeamSearchQueryChange={onTeamSearchQueryChange}
              teamNewRole={teamNewRole}
              setTeamNewRole={setTeamNewRole}
              teamNewEmail={teamNewEmail}
              setTeamNewEmail={setTeamNewEmail}
              teamSearchResults={(teamSearchResults ?? []).filter(
                (p) => p.relationship === "internal" && !teamMembers.some((m) => m.personId === p.id),
              )}
              onClose={() => {
                setTeamAddingMember(false);
                onTeamSearchQueryChange?.("");
              }}
              onAddTeamMember={(personId) => {
                onAddTeamMember?.(personId, teamNewRole);
                setTeamAddingMember(false);
                onTeamSearchQueryChange?.("");
                setTeamNewRole("associated");
              }}
              onCreateTeamMember={onCreateTeamMember ? (query) => {
                onCreateTeamMember(query, teamNewEmail, teamNewRole);
                setTeamAddingMember(false);
                onTeamSearchQueryChange?.("");
                setTeamNewRole("associated");
                setTeamNewEmail("");
              } : undefined}
            />
          )}
        </div>
      )}
    </section>
  );
}

// Relationship depth color helpers
function getDepthColor(dimension: string, value: string): string {
  const v = value.toLowerCase().replace(/[_\s-]/g, "");
  if (dimension === "champion") {
    if (v === "strong") return css.depthBadgeSage;
    if (v === "moderate" || v === "adequate") return css.depthBadgeTurmeric;
    if (v === "weak") return css.depthBadgeTerracotta;
    if (v === "none") return css.depthBadgeRed;
    return css.depthBadgeNeutral;
  }
  if (dimension === "access") {
    if (v === "direct") return css.depthBadgeSage;
    if (v === "indirect" || v === "limited") return css.depthBadgeTurmeric;
    if (v === "none") return css.depthBadgeTerracotta;
    return css.depthBadgeNeutral;
  }
  if (dimension === "coverage") {
    if (v === "broad") return css.depthBadgeSage;
    if (v === "narrow") return css.depthBadgeTurmeric;
    if (v === "singlethreaded") return css.depthBadgeTerracotta;
    return css.depthBadgeNeutral;
  }
  return css.depthBadgeNeutral;
}

// Static badge helpers for non-editable mode
function getEngagementBadgeClass(engagement: string): string {
  const e = (engagement ?? "").toLowerCase();
  if (e === "high" || e === "active") return css.engagementActive;
  if (e === "medium" || e === "warm") return css.engagementWarm;
  if (e === "low" || e === "cooling") return css.engagementCooling;
  return css.engagementNew;
}

function getStaticBadgeLabel(engagement: string): string {
  return getEngagementDisplay(engagement).label;
}

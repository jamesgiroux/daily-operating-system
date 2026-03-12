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
import { useState, useRef, useEffect } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { X, Plus, UserPlus, Search, LinkIcon, Check } from "lucide-react";
import type { EntityIntelligence, StakeholderInsight, Person, AccountTeamMember } from "@/types";
import { formatRelativeDate } from "@/lib/utils";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { Avatar } from "@/components/ui/Avatar";
import { EngagementSelector } from "./EngagementSelector";
import { getEngagementDisplay } from "./EngagementSelector";
import css from "./StakeholderGallery.module.css";

interface StakeholderGalleryProps {
  intelligence: EntityIntelligence | null;
  linkedPeople: Person[];
  accountTeam?: AccountTeamMember[];
  sectionId?: string;
  chapterTitle?: string;
  /** Entity ID for intelligence updates. */
  entityId?: string;
  /** Entity type for intelligence updates. */
  entityType?: string;
  /** Called after any intelligence field is updated (for parent re-fetch). */
  onIntelligenceUpdated?: () => void;
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
  sectionId = "the-room",
  chapterTitle = "The Room",
  entityId,
  entityType,
  onIntelligenceUpdated,
}: StakeholderGalleryProps) {
  const navigate = useNavigate();
  const allStakeholders = intelligence?.stakeholderInsights ?? [];
  const stakeholders = filterInternalStakeholders(allStakeholders, linkedPeople);
  const hasStakeholders = stakeholders.length > 0;
  const epigraph = hasStakeholders ? buildEpigraph(stakeholders) : undefined;
  const teamMembers = accountTeam ?? [];
  const canEdit = !!entityId && !!entityType;

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
  if (!hasStakeholders && linkedPeople.length === 0 && teamMembers.length === 0 && !canEdit) {
    return null;
  }

  const STAKEHOLDER_LIMIT = 6;
  const visibleStakeholders = expandedGrid ? stakeholders : stakeholders.slice(0, STAKEHOLDER_LIMIT);
  const hasMoreStakeholders = stakeholders.length > STAKEHOLDER_LIMIT && !expandedGrid;

  // ── Coverage analysis ──
  const totalKnown = stakeholders.length + linkedPeople.filter(
    (p) => !stakeholders.some((s) => s.name.toLowerCase() === p.name.toLowerCase()),
  ).length;
  const engagedCount = stakeholders.filter(
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
        console.error("link_person_entity failed:", err);
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
    <section id={sectionId} className={css.section}>
      <ChapterHeading title={chapterTitle} epigraph={epigraph} />

      {hasStakeholders ? (
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
                      style={{ fontFamily: "var(--font-sans)", fontSize: 16, fontWeight: 500, color: "var(--color-text-primary)" }}
                    />
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
                      className={css.engagementBadge}
                      style={getStaticBadgeStyle(s.engagement)}
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
                      style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 400, color: "var(--color-text-tertiary)", margin: "0 0 8px 0" }}
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
                      style={{ fontFamily: "var(--font-sans)", fontSize: 14, lineHeight: 1.6, color: "var(--color-text-secondary)", margin: 0 }}
                    />
                  ) : (
                    <TruncatedAssessment text={s.assessment} />
                  )
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

            if (matched) {
              return (
                <Link key={i} to="/people/$personId" params={{ personId: matched.id }} className={css.cardLink}>
                  {card}
                </Link>
              );
            }
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
            <Link key={p.id} to="/people/$personId" params={{ personId: p.id }} className={css.cardLink}>
              <span className={css.name}>
                {p.name}
              </span>
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
            </Link>
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
          <span className={css.coverageLabel}>known stakeholders with defined roles</span>
        </div>
      )}

      {/* Your Team strip */}
      {teamMembers.length > 0 && (
        <div className={css.teamStrip}>
          <span className={css.teamLabel}>
            Your Team
          </span>
          {teamMembers.map((member) => (
            <span key={member.personId} className={css.teamMember}>
              <span className={css.teamMemberRole}>
                {member.role}
              </span>
              <span className={css.teamMemberName}>
                {member.personName}
              </span>
            </span>
          ))}
        </div>
      )}
    </section>
  );
}

// Static badge helpers for non-editable mode
function getStaticBadgeStyle(engagement: string): { background: string; color: string } {
  const d = getEngagementDisplay(engagement);
  return { background: d.background, color: d.color };
}

function getStaticBadgeLabel(engagement: string): string {
  return getEngagementDisplay(engagement).label;
}

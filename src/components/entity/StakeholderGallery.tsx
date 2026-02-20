/**
 * StakeholderGallery — People chapter.
 * 2-column grid of stakeholder cards with editable engagement badges.
 * Falls back to linkedPeople when no intelligence stakeholders exist.
 * Includes an optional "Your Team" strip for account team members.
 * Generalized: configurable title/id, accountTeam optional.
 *
 * I261: Live editing (name, role, assessment, engagement), add/remove
 * stakeholders, internal people filter, create contact from stakeholder.
 */
import { useState, useRef, useEffect } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { X, Plus, UserPlus, Search } from "lucide-react";
import type { EntityIntelligence, StakeholderInsight, Person, AccountTeamMember } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditableText } from "@/components/ui/EditableText";
import { Avatar } from "@/components/ui/Avatar";
import { EngagementSelector } from "./EngagementSelector";

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
  const displayText = truncated ? text.slice(0, ASSESSMENT_CHAR_LIMIT) + "…" : text;
  return (
    <p style={{ fontFamily: "var(--font-sans)", fontSize: 14, lineHeight: 1.6, color: "var(--color-text-secondary)", margin: 0 }}>
      {displayText}
      {truncated && (
        <button
          onClick={(e) => { e.preventDefault(); e.stopPropagation(); setShowFull(true); }}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "0 0 0 4px",
          }}
        >
          Read more
        </button>
      )}
    </p>
  );
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
      invoke("link_person_entity", { personId: person.id, entityId, relationshipType: "associated" }).catch(() => {});
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
      await invoke("create_person_from_stakeholder", {
        entityId,
        entityType,
        name: s.name,
        role: s.role ?? null,
      });
      onIntelligenceUpdated?.();
    } catch (e) {
      console.error("Failed to create person from stakeholder:", e);
    }
  }

  // ── Find actual index in allStakeholders for a filtered stakeholder ──
  function actualIndex(filteredIdx: number): number {
    const name = stakeholders[filteredIdx].name.toLowerCase();
    return allStakeholders.findIndex((s) => s.name.toLowerCase() === name);
  }

  return (
    <section id={sectionId} style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <ChapterHeading title={chapterTitle} epigraph={epigraph} />

      {hasStakeholders ? (
        <>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "40px 48px" }}>
          {visibleStakeholders.map((s, i) => {
            const matched = linkedPeople.find(
              (p) => p.name.toLowerCase() === s.name.toLowerCase()
            );
            const idx = actualIndex(i);
            const isHovered = hoveredCard === i;

            const card = (
              <div
                key={i}
                style={{ position: "relative" }}
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
                    style={{
                      position: "absolute",
                      top: -4,
                      right: -4,
                      width: 20,
                      height: 20,
                      borderRadius: "50%",
                      border: "1px solid var(--color-rule-light)",
                      background: "var(--color-paper-cream)",
                      cursor: "pointer",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      zIndex: 2,
                    }}
                    title="Remove stakeholder"
                  >
                    <X size={12} strokeWidth={1.5} style={{ color: "var(--color-text-tertiary)" }} />
                  </button>
                )}

                <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 8, flexWrap: "wrap" }}>
                  <Avatar name={s.name} size={24} />
                  {canEdit ? (
                    <EditableText
                      value={s.name}
                      onChange={(v) => updateField(`stakeholderInsights[${idx}].name`, v)}
                      multiline={false}
                      style={{ fontFamily: "var(--font-sans)", fontSize: 16, fontWeight: 500, color: "var(--color-text-primary)" }}
                    />
                  ) : (
                    <span style={{ fontFamily: "var(--font-sans)", fontSize: 16, fontWeight: 500, color: "var(--color-text-primary)" }}>
                      {s.name}
                    </span>
                  )}
                  {s.engagement && canEdit ? (
                    <EngagementSelector
                      value={s.engagement}
                      onChange={(v) => updateField(`stakeholderInsights[${idx}].engagement`, v)}
                    />
                  ) : s.engagement ? (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 9,
                        fontWeight: 500,
                        textTransform: "uppercase",
                        letterSpacing: "0.08em",
                        padding: "2px 7px",
                        borderRadius: 3,
                        ...getStaticBadgeStyle(s.engagement),
                      }}
                    >
                      {getStaticBadgeLabel(s.engagement)}
                    </span>
                  ) : null}
                </div>
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
                    <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 400, color: "var(--color-text-tertiary)", margin: "0 0 8px 0" }}>
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
                {/* Create contact action for unlinked stakeholders */}
                {canEdit && !matched && isHovered && (
                  <button
                    onClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleCreateContact(s);
                    }}
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      marginTop: 8,
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: 0,
                    }}
                  >
                    <UserPlus size={12} strokeWidth={1.5} />
                    Create contact
                  </button>
                )}
              </div>
            );

            if (matched) {
              return (
                <Link key={i} to="/people/$personId" params={{ personId: matched.id }} style={{ textDecoration: "none", color: "inherit" }}>
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
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: "12px 0 0",
              textTransform: "uppercase",
              letterSpacing: "0.06em",
            }}
          >
            Show {stakeholders.length - STAKEHOLDER_LIMIT} more
          </button>
        )}
        </>
      ) : linkedPeople.length > 0 ? (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "40px 48px" }}>
          {linkedPeople.map((p) => (
            <Link key={p.id} to="/people/$personId" params={{ personId: p.id }} style={{ textDecoration: "none", color: "inherit" }}>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 16, fontWeight: 500, color: "var(--color-text-primary)" }}>
                {p.name}
              </span>
              {p.role && (
                <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 400, color: "var(--color-text-tertiary)", margin: "4px 0 0 0" }}>
                  {p.role}
                </p>
              )}
            </Link>
          ))}
        </div>
      ) : null}

      {/* Add stakeholder */}
      {canEdit && (
        <div style={{ marginTop: hasStakeholders ? 32 : 16 }}>
          {addingStakeholder ? (
            <div ref={addContainerRef}>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <div style={{ position: "relative", width: 200 }}>
                  <Search size={12} style={{ position: "absolute", left: 8, top: 9, color: "var(--color-text-tertiary)" }} />
                  <input
                    value={newName}
                    onChange={(e) => handleNameChange(e.target.value)}
                    placeholder="Search people or type name"
                    autoFocus
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleAdd();
                      if (e.key === "Escape") { setAddingStakeholder(false); setNewName(""); setNewRole(""); setShowDropdown(false); }
                    }}
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      padding: "4px 8px 4px 26px",
                      border: "1px solid var(--color-rule-light)",
                      borderRadius: 4,
                      background: "transparent",
                      color: "var(--color-text-primary)",
                      width: "100%",
                    }}
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
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    padding: "4px 8px",
                    border: "1px solid var(--color-rule-light)",
                    borderRadius: 4,
                    background: "transparent",
                    color: "var(--color-text-primary)",
                    width: 160,
                  }}
                />
                <button
                  onClick={handleAdd}
                  disabled={!newName.trim()}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                    color: newName.trim() ? "var(--color-spice-turmeric)" : "var(--color-text-tertiary)",
                    background: "none",
                    border: "none",
                    cursor: newName.trim() ? "pointer" : "default",
                    padding: "4px 0",
                  }}
                >
                  Add
                </button>
              </div>

              {/* Inline search results — no absolute positioning, no z-index issues */}
              {showDropdown && searchResults.length > 0 && (
                <div style={{
                  marginTop: 8,
                  border: "1px solid var(--color-rule-light)",
                  borderRadius: 4,
                  background: "var(--color-paper-warm-white)",
                }}>
                  <p style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                    color: "var(--color-text-tertiary)",
                    padding: "8px 12px 4px",
                    margin: 0,
                  }}>
                    Existing People
                  </p>
                  {searchResults.map((person) => (
                    <button
                      key={person.id}
                      onClick={() => handleSelectPerson(person)}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                        width: "100%",
                        padding: "8px 12px",
                        background: "none",
                        border: "none",
                        borderTop: "1px solid var(--color-rule-light)",
                        cursor: "pointer",
                        textAlign: "left",
                      }}
                    >
                      <UserPlus size={14} style={{ color: "var(--color-garden-larkspur)", flexShrink: 0 }} />
                      <div>
                        <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                          {person.name}
                        </span>
                        {(person.role || person.organization) && (
                          <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--color-text-tertiary)", letterSpacing: "0.04em", marginLeft: 8 }}>
                            {[person.role, person.organization].filter(Boolean).join(" · ")}
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
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
              }}
            >
              <Plus size={12} strokeWidth={1.5} />
              Add Stakeholder
            </button>
          )}
        </div>
      )}

      {/* Your Team strip */}
      {teamMembers.length > 0 && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-heavy)",
            borderBottom: "1px solid var(--color-rule-heavy)",
            padding: "14px 0",
            marginTop: 40,
            display: "flex",
            alignItems: "baseline",
            gap: 24,
            flexWrap: "wrap",
          }}
        >
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 500, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)" }}>
            Your Team
          </span>
          {teamMembers.map((member) => (
            <span key={member.personId} style={{ display: "inline-flex", alignItems: "baseline", gap: 6 }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 500, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)" }}>
                {member.role}
              </span>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-secondary)" }}>
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
import { getEngagementDisplay } from "./EngagementSelector";

function getStaticBadgeStyle(engagement: string): { background: string; color: string } {
  const d = getEngagementDisplay(engagement);
  return { background: d.background, color: d.color };
}

function getStaticBadgeLabel(engagement: string): string {
  return getEngagementDisplay(engagement).label;
}

/**
 * StakeholderGallery — People chapter.
 * 2-column grid of stakeholder cards with multi-role badges and engagement levels.
 * Renders exclusively from DB-backed `stakeholdersFull` data.
 * AI suggestions come from a separate `suggestions` prop (I652 phase 2).
 * Includes an optional "Your Team" strip for account team members.
 *
 * I652: Removed intelligence JSON rendering. Stakeholders are DB-first.
 * Multi-role display, engagement from DB, AI suggestions from prop.
 */
import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";
import { Link, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { X, Plus, UserPlus, Search, LinkIcon, Award, Check } from "lucide-react";
import type { EntityIntelligence, Person, AccountTeamMember, StakeholderFull, StakeholderSuggestion, StakeholderRole } from "@/types";
import { formatRelativeDate } from "@/lib/utils";
import { formatProvenanceSource } from "@/components/ui/ProvenanceLabel";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { Avatar } from "@/components/ui/Avatar";
import { EngagementSelector, getEngagementDisplay, getEngagementLabel } from "./EngagementSelector";
import { TeamRoleSelector, getTeamRoleDisplay } from "./TeamRoleSelector";
import css from "./StakeholderGallery.module.css";

/** Stakeholder role definitions for the multi-role picker. */
const STAKEHOLDER_ROLES = [
  { stored: "champion", label: "Champion", bg: "var(--color-spice-turmeric-12)", fg: "var(--color-spice-turmeric)" },
  { stored: "executive_sponsor", label: "Exec Sponsor", bg: "var(--color-garden-rosemary-14)", fg: "var(--color-garden-rosemary)" },
  { stored: "decision_maker", label: "Decision Maker", bg: "var(--color-garden-rosemary-14)", fg: "var(--color-garden-rosemary)" },
  { stored: "primary_contact", label: "Primary Contact", bg: "var(--color-garden-larkspur-14)", fg: "var(--color-garden-larkspur)" },
  { stored: "technical_contact", label: "Technical Contact", bg: "var(--color-garden-larkspur-14)", fg: "var(--color-garden-larkspur)" },
  { stored: "power_user", label: "Power User", bg: "var(--color-garden-larkspur-14)", fg: "var(--color-garden-larkspur)" },
  { stored: "end_user", label: "End User", bg: "var(--color-garden-larkspur-8)", fg: "var(--color-text-tertiary)" },
];

/** Get config (label + colors) for a role. */
function getRoleConfig(stored: string) {
  return (
    STAKEHOLDER_ROLES.find((r) => r.stored === stored.toLowerCase()) ??
    STAKEHOLDER_ROLES[STAKEHOLDER_ROLES.length - 1]
  );
}

interface StakeholderGalleryProps {
  intelligence: EntityIntelligence | null;
  linkedPeople: Person[];
  accountTeam?: AccountTeamMember[];
  /** DB-first stakeholder read model — primary display source when non-empty. */
  stakeholdersFull?: StakeholderFull[];
  /** AI-suggested stakeholders pending accept/dismiss (I652). */
  suggestions?: StakeholderSuggestion[];
  sectionId?: string;
  chapterTitle?: string;
  /** DOS-18: Optional freshness strip rendered under the chapter heading. */
  chapterFreshness?: React.ReactNode;
  /** DOS-18: When true, render "Their team" / "Our team" subsection labels per account-context mockup. */
  subsectionLabels?: boolean;
  /**
   * DOS-18: When subsectionLabels is true, the italic hints next to the
   * "Their team" / "Our team" labels show who we're meeting with. Optional —
   * if omitted we fall back to generic phrasing without the customer name.
   */
  accountName?: string;
  /**
   * DOS-18: Optional anchor builder — returns the href (e.g. "#dimension-adoption")
   * to the Health tab chapter that mentions this person. When provided, each
   * confirmed stakeholder card renders an "Active in Health →" cross-reference
   * pill. Returning null skips the pill for that person.
   */
  healthAnchorFor?: (personId: string) => string | null;
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
  /** Accept a stakeholder suggestion (I652). */
  onAcceptSuggestion?: (suggestionId: number) => void;
  /** Dismiss a stakeholder suggestion (I652). */
  onDismissSuggestion?: (suggestionId: number) => void;
  /** Update engagement level for a stakeholder (I652). */
  onUpdateEngagement?: (personId: string, engagement: string) => void;
  /** Update assessment for a stakeholder (I652). */
  onUpdateAssessment?: (personId: string, assessment: string) => void;
  /** Add a role to a stakeholder (I652 multi-role). */
  onAddRole?: (personId: string, role: string) => void;
  /** Remove a role from a stakeholder (I652 multi-role). */
  onRemoveRole?: (personId: string, role: string) => void;
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

/** Role picker dropdown for adding roles to a stakeholder. */
function RolePicker({
  existingRoles,
  onSelect,
  onClose,
}: {
  existingRoles: string[];
  onSelect: (role: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const existing = new Set(existingRoles.map((r) => r.toLowerCase()));
  const available = STAKEHOLDER_ROLES.filter((r) => !existing.has(r.stored));

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [onClose]);

  if (available.length === 0) return null;

  return (
    <div ref={ref} className={css.rolePickerDropdown}>
      {available.map((r) => (
        <button
          key={r.stored}
          className={css.rolePickerItem}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onSelect(r.stored);
            onClose();
          }}
        >
          {r.label}
        </button>
      ))}
    </div>
  );
}

export function StakeholderGallery({
  intelligence,
  linkedPeople,
  accountTeam,
  stakeholdersFull,
  suggestions,
  sectionId = "the-room",
  chapterTitle = "The Room",
  chapterFreshness,
  subsectionLabels = false,
  accountName,
  healthAnchorFor,
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
  onAcceptSuggestion,
  onDismissSuggestion,
  onUpdateEngagement,
  onUpdateAssessment: _onUpdateAssessment,
  onAddRole,
  onRemoveRole,
}: StakeholderGalleryProps) {
  const navigate = useNavigate();

  // DB-backed confirmed stakeholders are the primary source
  const confirmedStakeholders = stakeholdersFull ?? [];
  // Dedup suggestions against confirmed stakeholders (by email first, name second)
  const confirmedEmails = new Set(confirmedStakeholders.map((s) => s.personEmail?.toLowerCase()).filter(Boolean));
  const confirmedNames = new Set(confirmedStakeholders.map((s) => s.personName?.toLowerCase()).filter(Boolean));
  const pendingSuggestions = (suggestions ?? [])
    .filter((s) => s.status === "pending")
    .filter((s) => {
      if (s.suggestedEmail && confirmedEmails.has(s.suggestedEmail.toLowerCase())) return false;
      if (s.suggestedName && confirmedNames.has(s.suggestedName.toLowerCase())) return false;
      return true;
    });

  const hasStakeholders = confirmedStakeholders.length > 0 || pendingSuggestions.length > 0;
  const epigraphSource = confirmedStakeholders.map((s) => ({ name: s.personName }));
  const epigraph = hasStakeholders ? buildEpigraph(epigraphSource) : undefined;
  const teamMembers = accountTeam ?? [];
  const canEdit = !!entityId && !!entityType;
  const canEditTeam = !!onRemoveTeamMember;

  const [teamAddingMember, setTeamAddingMember] = useState(false);
  const [teamNewRole, setTeamNewRole] = useState("associated");
  const [teamNewEmail, setTeamNewEmail] = useState("");

  const [expandedGrid, setExpandedGrid] = useState(false);
  const [addingStakeholder, setAddingStakeholder] = useState(false);
  const [newName, setNewName] = useState("");
  const [newRole, setNewRole] = useState("");
  const [searchResults, setSearchResults] = useState<Person[]>([]);
  const [showDropdown, setShowDropdown] = useState(false);
  const [rolePickerFor, setRolePickerFor] = useState<string | null>(null);
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

  // Empty section collapse
  if (!hasStakeholders && linkedPeople.length === 0 && teamMembers.length === 0 && !canEdit && !canEditTeam) {
    return null;
  }

  const STAKEHOLDER_LIMIT = 6;
  const visibleConfirmed = expandedGrid ? confirmedStakeholders : confirmedStakeholders.slice(0, STAKEHOLDER_LIMIT);
  const hasMoreStakeholders = confirmedStakeholders.length > STAKEHOLDER_LIMIT && !expandedGrid;

  // ── Coverage analysis ──
  const totalKnown = confirmedStakeholders.length;
  // Count stakeholders with defined engagement OR at least one role assigned
  const engagedCount = confirmedStakeholders.filter(
    (s) =>
      (s.engagement && s.engagement !== "unknown" && s.engagement !== "none") ||
      (s.roles && s.roles.length > 0),
  ).length;

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
        const existingIds = new Set(confirmedStakeholders.map((s) => s.personId));
        const filtered = results.filter((p) => !existingIds.has(p.id));
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
    if (entityId && entityType) {
      invoke("link_person_entity", { personId: person.id, entityId, relationshipType: "associated" }).catch((err) => {
        console.error("link_person_entity failed:", err);
      });
    }
    onIntelligenceUpdated?.();
    setNewName("");
    setNewRole("");
    setSearchResults([]);
    setShowDropdown(false);
    setAddingStakeholder(false);
  }

  // ── Add new stakeholder (create new) ──
  async function handleAdd() {
    if (!newName.trim() || !entityId || !entityType) return;
    try {
      const personId = await invoke<string>("create_person_from_stakeholder", {
        entityId,
        entityType,
        name: newName.trim(),
        role: newRole.trim() || null,
      });
      onIntelligenceUpdated?.();
      if (personId) {
        navigate({ to: "/people/$personId", params: { personId } });
      }
    } catch (e) {
      console.error("Failed to create stakeholder:", e);
      toast.error("Failed to save");
    }
    setNewName("");
    setNewRole("");
    setSearchResults([]);
    setShowDropdown(false);
    setAddingStakeholder(false);
  }

  return (
    <section id={sectionId || undefined} className={css.section}>
      <ChapterHeading title={chapterTitle} epigraph={epigraph} freshness={chapterFreshness} />

      {subsectionLabels && visibleConfirmed.length > 0 && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.12em",
            color: "var(--color-text-secondary)",
            marginBottom: 16,
            marginTop: 8,
            display: "flex",
            alignItems: "baseline",
            gap: 12,
            flexWrap: "wrap",
          }}
        >
          Their team
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              fontWeight: 400,
              color: "var(--color-text-tertiary)",
              textTransform: "none",
              letterSpacing: 0,
              fontStyle: "italic",
            }}
          >
            who we&apos;re meeting with{accountName ? ` — ${accountName}` : ""}
          </span>
        </div>
      )}

      {/* ── Confirmed stakeholders (from DB) ── */}
      {visibleConfirmed.length > 0 && (
        <div className={css.grid}>
          {visibleConfirmed.map((s) => {
            const personDetail = [s.personRole, s.organization].filter(Boolean).join(" \u00b7 ") || null;
            const roles: StakeholderRole[] = s.roles ?? [];

            return (
              <div key={s.personId} className={css.card}>
                {/* Remove button (hover-revealed) */}
                {onRemoveTeamMember && (
                  <button
                    className={css.cardRemoveButton}
                    onClick={(e) => {
                      e.stopPropagation();
                      onRemoveTeamMember(s.personId, s.stakeholderRole);
                    }}
                    title="Remove from account"
                  >
                    <X size={13} />
                  </button>
                )}
                <div className={css.cardHeader}>
                  <div className={css.avatarRingLinked}>
                    <Avatar name={s.personName} personId={s.personId} size={24} />
                  </div>
                  <Link to="/people/$personId" params={{ personId: s.personId }} className={css.nameLink}>
                    {s.personName}
                  </Link>
                  <LinkIcon size={12} strokeWidth={1.5} className={css.linkIcon} aria-label={`Linked to ${s.personName}`} />

                  {/* Engagement badge from DB field */}
                  {s.engagement && onUpdateEngagement ? (
                    <EngagementSelector
                      value={s.engagement}
                      onChange={(v) => onUpdateEngagement(s.personId, v)}
                    />
                  ) : s.engagement && s.engagement !== "unknown" ? (
                    <span
                      className={css.engagementBadge}
                      style={{
                        background: getEngagementDisplay(s.engagement).background,
                        color: getEngagementDisplay(s.engagement).color,
                      }}
                    >
                      {getEngagementLabel(s.engagement)}
                    </span>
                  ) : null}
                </div>

                {personDetail && <p className={css.titleLine}>{personDetail}</p>}

                {/* Multi-role badges (I652) */}
                {(roles.length > 0 || onAddRole) && (
                  <div className={css.roleBadges}>
                    {roles.map((r) => (
                      <span key={r.role} className={css.roleBadge} data-source={r.dataSource} style={{ background: getRoleConfig(r.role).bg, color: getRoleConfig(r.role).fg }}>
                        {getRoleConfig(r.role).label}
                        {onRemoveRole && (
                          <button
                            className={css.roleRemove}
                            onClick={(e) => {
                              e.preventDefault();
                              e.stopPropagation();
                              onRemoveRole(s.personId, r.role);
                            }}
                          >
                            &times;
                          </button>
                        )}
                      </span>
                    ))}
                    {onAddRole && (
                      <div style={{ position: "relative", display: "inline-block" }}>
                        <button
                          className={css.addRoleBtn}
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            setRolePickerFor(rolePickerFor === s.personId ? null : s.personId);
                          }}
                        >
                          +
                        </button>
                        {rolePickerFor === s.personId && (
                          <RolePicker
                            existingRoles={roles.map((r) => r.role)}
                            onSelect={(role) => onAddRole(s.personId, role)}
                            onClose={() => setRolePickerFor(null)}
                          />
                        )}
                      </div>
                    )}
                  </div>
                )}

                {/* Legacy single role fallback when no multi-role data */}
                {roles.length === 0 && s.stakeholderRole && s.stakeholderRole !== "associated" && !onAddRole && (
                  <span className={`${css.engagementBadge} ${css.engagementNew}`}>
                    {s.stakeholderRole}
                  </span>
                )}

                {/* Assessment from DB (I652) or gap-state placeholder per mockup */}
                {s.assessment ? (
                  <TruncatedAssessment text={s.assessment} />
                ) : (
                  <>
                    <div
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontStyle: "italic",
                        fontSize: 13,
                        lineHeight: 1.55,
                        color: "var(--color-text-tertiary)",
                        background: "var(--color-spice-saffron-10, rgba(196,147,53,0.10))",
                        borderLeft: "2px solid var(--color-spice-saffron)",
                        borderRadius: "0 var(--radius-sm, 4px) var(--radius-sm, 4px) 0",
                        padding: "8px 12px",
                        marginTop: 4,
                      }}
                    >
                      {s.meetingCount != null && s.meetingCount > 0
                        ? `Assessment pending — attended ${s.meetingCount} meeting${s.meetingCount === 1 ? "" : "s"} but never characterized.`
                        : "Assessment pending — never characterized."}
                    </div>
                    <span
                      className={css.engagementBadge}
                      style={{
                        background: "var(--color-spice-saffron-15, rgba(196,147,53,0.15))",
                        color: "#a6862c",
                        alignSelf: "flex-start",
                        marginTop: 6,
                      }}
                    >
                      Needs assessment
                    </span>
                  </>
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

                {/* Cross-reference to Health tab — rendered when host provides an anchor. */}
                {(() => {
                  const anchor = healthAnchorFor?.(s.personId);
                  if (!anchor) return null;
                  return (
                    <a
                      href={anchor}
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 9,
                        textTransform: "uppercase",
                        letterSpacing: "0.08em",
                        color: "var(--color-spice-turmeric)",
                        borderBottom: "1px dotted var(--color-spice-turmeric)",
                        textDecoration: "none",
                        alignSelf: "flex-start",
                        marginTop: 6,
                      }}
                    >
                      Active in Health →
                    </a>
                  );
                })()}
              </div>
            );
          })}
        </div>
      )}

      {/* ── Suggested stakeholders — rendered as editorial cards in the grid (I652) ── */}
      {pendingSuggestions.length > 0 && (
        <div className={css.grid} style={visibleConfirmed.length > 0 ? { marginTop: 40 } : undefined}>
          {pendingSuggestions.map((s) => {
            const sourceLabel = formatProvenanceSource(s.source) ?? "AI";
            return (
              <div key={s.id} className={css.card}>
                {/* Hover-revealed action buttons */}
                <div className={css.suggestedCardActions}>
                  {onAcceptSuggestion && (
                    <button
                      className={css.suggestedAcceptBtn}
                      onClick={(e) => { e.stopPropagation(); onAcceptSuggestion(s.id); }}
                      title="Accept as stakeholder"
                    >
                      <Check size={13} strokeWidth={1.5} />
                    </button>
                  )}
                  {onDismissSuggestion && (
                    <button
                      className={css.suggestedDismissBtn}
                      onClick={(e) => { e.stopPropagation(); onDismissSuggestion(s.id); }}
                      title="Dismiss"
                    >
                      <X size={13} strokeWidth={1.5} />
                    </button>
                  )}
                </div>

                <div className={css.cardHeader}>
                  <div className={css.avatarRing}>
                    <Avatar name={s.suggestedName ?? "?"} size={24} />
                  </div>
                  <span className={css.name}>{s.suggestedName ?? "Unknown"}</span>
                </div>

                {s.suggestedEmail && <p className={css.titleLine}>{s.suggestedEmail}</p>}

                {s.suggestedRole && (
                  <div className={css.roleBadges}>
                    <span className={css.roleBadge} style={{ background: getRoleConfig(s.suggestedRole).bg, color: getRoleConfig(s.suggestedRole).fg }}>
                      {getRoleConfig(s.suggestedRole).label}
                    </span>
                  </div>
                )}

                <div className={css.suggestedBadge}>Suggested via {sourceLabel}</div>
              </div>
            );
          })}
        </div>
      )}

      {/* Fallback: linkedPeople when no confirmed or suggested stakeholders */}
      {!hasStakeholders && linkedPeople.length > 0 && (
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
      )}

      {/* Show more */}
      {hasMoreStakeholders && (
        <button onClick={() => setExpandedGrid(true)} className={css.showMore}>
          Show {confirmedStakeholders.length - STAKEHOLDER_LIMIT} more
        </button>
      )}

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

      {/* Coverage analysis strip */}
      {totalKnown > 0 && (
        <div className={css.coverageStrip}>
          <span className={css.coverageNumbers}>{engagedCount} of {totalKnown}</span>
          <span className={css.coverageLabel}>stakeholders with defined roles</span>
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
            <span className={css.teamLabel}>{subsectionLabels ? "Our team" : "Your Team"}</span>
            {subsectionLabels && (
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  fontWeight: 400,
                  color: "var(--color-text-tertiary)",
                  textTransform: "none",
                  letterSpacing: 0,
                  fontStyle: "italic",
                }}
              >
                who we bring into{accountName ? ` ${accountName}` : " these"} conversations — Automattic
              </span>
            )}
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

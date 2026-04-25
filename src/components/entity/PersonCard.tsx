/**
 * PersonCard — single stakeholder card for the Context / The Room chapter.
 *
 * Three variants share the same shape but differ in density:
 *   - "primary"  — span 2 cols, full detail (avatar 56, serif 16 name, assessment)
 *   - "compact"  — 1 col, condensed
 *   - "internal" — 1 col, charcoal-tinted (for "Our team" members)
 *
 * Multi-role pills wire to atomic add/remove mutations via
 * `onAddRole` / `onRemoveRole`. Pills carry per-role provenance — AI-surfaced
 * roles render with a dashed outline so the user can see what hasn't been
 * human-pinned yet. The role chip editor is the v1.2.1 replacement for the
 * old single-value dropdown that silently wiped AI-surfaced rows.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { StakeholderFull } from "@/types";
import css from "./StakeholderGrid.module.css";

export type PersonCardVariant = "primary" | "compact" | "internal";

export interface RoleOption {
  /** Stored value (lowercase, snake-case): "champion", "technical", etc. */
  value: string;
  /** Display label. */
  label: string;
  /** Which tone class to apply (maps to .rolePillChampion / .rolePillTechnical / …). */
  tone: "champion" | "technical" | "economic" | "internal" | "gap" | "default";
}

/** Canonical role catalog for external stakeholders (Their team). */
export const EXTERNAL_ROLE_CATALOG: RoleOption[] = [
  { value: "champion", label: "Champion", tone: "champion" },
  { value: "executive_sponsor", label: "Exec Sponsor", tone: "champion" },
  { value: "decision_maker", label: "Decision Maker", tone: "economic" },
  { value: "economic_buyer", label: "Economic Buyer", tone: "economic" },
  { value: "primary_contact", label: "Primary Contact", tone: "technical" },
  { value: "technical_contact", label: "Technical Contact", tone: "technical" },
  { value: "technical", label: "Technical", tone: "technical" },
  { value: "power_user", label: "Power User", tone: "technical" },
  { value: "end_user", label: "End User", tone: "default" },
];

/**
 * Canonical role catalog for internal team members (Our team).
 *
 * Source of truth: the stored values used by the existing
 * `set_team_member_role` / `add_stakeholder_role` backends and by the
 * legacy `TeamRoleSelector` component. Keeping these in lockstep so the
 * same stored role renders the same label everywhere in the app.
 * Internal labels are short codes per product vocabulary (CSM / AE / RM)
 * — different from external stakeholder types (Champion / Technical /
 * Economic), which is a separate catalog entirely.
 */
export const INTERNAL_ROLE_CATALOG: RoleOption[] = [
  { value: "ae", label: "AE", tone: "internal" },
  { value: "csm", label: "CSM", tone: "internal" },
  { value: "tam", label: "TAM", tone: "internal" },
  { value: "rm", label: "RM", tone: "internal" },
  { value: "ao", label: "AO", tone: "internal" },
  { value: "se", label: "SE", tone: "internal" },
  { value: "executive_sponsor", label: "Exec Sponsor", tone: "internal" },
  { value: "implementation", label: "Implementation", tone: "internal" },
  { value: "associated", label: "Associated", tone: "internal" },
];

function toneClass(tone: RoleOption["tone"]): string {
  switch (tone) {
    case "champion":
      return css.rolePillChampion;
    case "technical":
      return css.rolePillTechnical;
    case "economic":
      return css.rolePillEconomic;
    case "internal":
      return css.rolePillInternal;
    case "gap":
      return css.rolePillGap;
    default:
      return css.rolePillDefault;
  }
}

function findRoleOption(catalog: RoleOption[], value: string): RoleOption {
  const found = catalog.find((r) => r.value === value.toLowerCase());
  if (found) return found;
  // Unknown role — render as default tone with humanised label.
  return {
    value,
    label: value
      .split(/[_\s]+/)
      .map((w) => (w.length > 0 ? w[0].toUpperCase() + w.slice(1) : ""))
      .join(" "),
    tone: "default",
  };
}

interface PersonCardProps {
  person: StakeholderFull;
  variant: PersonCardVariant;
  /** Role catalog this card should show in the add-role picker. */
  roleCatalog: RoleOption[];
  /** Pin / add a role (atomic — doesn't disturb other roles). */
  onAddRole?: (personId: string, role: string) => void;
  /** Unpin a role (removes a single role row). */
  onRemoveRole?: (personId: string, role: string) => void;
  /** Optional remove-from-team affordance (internal variant only). */
  onRemoveMember?: (personId: string, primaryRole: string) => void;
  /**
   * Permanent remove for external stakeholders. When provided, a small
   * "×" sits at the top-right of the card header. The parent decides
   * whether this unlinks from just this account or deletes the person
   * entirely (e.g. when a bot email got auto-created as a person and
   * needs to go).
   */
  onRemoveStakeholder?: (personId: string, personName: string) => void;
}

export function PersonCard({
  person,
  variant,
  roleCatalog,
  onAddRole,
  onRemoveRole,
  onRemoveMember,
  onRemoveStakeholder,
}: PersonCardProps) {
  const cardClasses = [css.personCard];
  if (variant === "primary") cardClasses.push(css.personCardPrimary);
  if (variant === "internal") cardClasses.push(css.personCardInternal);

  const initials = buildInitials(person.personName);
  const title = person.personRole ?? person.organization ?? null;
  const location = deriveLocation(person);
  const meetingCount = person.meetingCount ?? null;
  const lastSeen = formatLastSeen(person.lastSeen);
  const canEditRoles = !!(onAddRole && onRemoveRole);
  const hasAssessment = !!(person.assessment && person.assessment.trim().length > 0);
  const showGapState = !hasAssessment && (meetingCount ?? 0) > 0;

  return (
    <article className={cardClasses.join(" ")}>
      <div className={css.personHeader}>
        <div className={css.avatar}>
          <PersonAvatar
            photoUrl={person.photoUrl}
            name={person.personName}
            initials={initials}
          />
        </div>
        <div className={css.personIdentity}>
          <div className={css.personName}>{person.personName}</div>
          {title ? <div className={css.personTitle}>{title}</div> : null}
          {location ? <div className={css.personLocation}>{location}</div> : null}
          <RolePills
            person={person}
            catalog={roleCatalog}
            canEdit={canEditRoles}
            onAddRole={onAddRole}
            onRemoveRole={onRemoveRole}
          />
        </div>
        {onRemoveMember && variant === "internal" ? (
          <button
            type="button"
            className={css.rolePillRemove}
            aria-label="Remove from team"
            title="Remove from team"
            onClick={() =>
              onRemoveMember(
                person.personId,
                person.roles[0]?.role ?? "associated",
              )
            }
          >
            ×
          </button>
        ) : null}
        {onRemoveStakeholder && variant !== "internal" ? (
          <button
            type="button"
            className={css.rolePillRemove}
            aria-label={`Remove ${person.personName}`}
            title="Remove from account"
            onClick={() => onRemoveStakeholder(person.personId, person.personName)}
          >
            ×
          </button>
        ) : null}
      </div>

      {/* Assessment prose — only on primary + internal, not compact (to keep
          secondary cards scan-fast per the mockup). */}
      {variant !== "compact" && hasAssessment ? (
        <p className={css.assessment}>{person.assessment}</p>
      ) : null}

      {/* Gap-state: show when the person has been in meetings but no one has
          characterised them yet. Primary + compact render this; internal
          skips it because internal team members don't get the same
          AI-assessment pipeline. */}
      {variant !== "internal" && showGapState ? (
        <p className={css.assessmentGap}>
          Assessment pending — attended
          {" "}
          {meetingCount}
          {" "}
          meeting{meetingCount === 1 ? "" : "s"} but never characterized.
        </p>
      ) : null}

      {/* Footer — compact meta. Skip on compact variant to save vertical
          space (compact cards render next to primaries). */}
      {variant !== "compact" ? (
        <MetaFooter meetingCount={meetingCount} lastSeen={lastSeen} emailCount={null} />
      ) : null}
    </article>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

/**
 * Avatar renderer with graceful fallback.
 *
 * When a person's photo URL 404s, stalls, or returns an invalid image
 * (common for Clay-enriched URLs that expire, or Gravatar hashes with
 * no account backing), the raw <img> tag renders the browser's default
 * broken-image glyph — a gray square with a question mark. Ugly, and
 * breaks the editorial register of the card grid.
 *
 * This component swaps to serif initials on the `onError` event so
 * every card reads the same (initials) whether or not the photo
 * resolved. We also reset the error flag when the URL changes so a
 * later enrichment that writes a working URL gets a fresh attempt
 * instead of being stuck on the last failure.
 */
function PersonAvatar({
  photoUrl,
  name,
  initials,
}: {
  photoUrl: string | null | undefined;
  name: string;
  initials: string;
}) {
  const [failed, setFailed] = useState(false);
  // Reset error state when the photo URL changes (e.g. a re-enrichment
  // writes a new Clay URL). Without this, a single bad URL would latch
  // failure for the component's lifetime.
  useEffect(() => {
    setFailed(false);
  }, [photoUrl]);

  if (!photoUrl || failed) {
    return <>{initials}</>;
  }
  return (
    <img
      src={photoUrl}
      alt={name}
      className={css.avatarImage}
      onError={() => setFailed(true)}
    />
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

interface RolePillsProps {
  person: StakeholderFull;
  catalog: RoleOption[];
  canEdit: boolean;
  onAddRole?: (personId: string, role: string) => void;
  onRemoveRole?: (personId: string, role: string) => void;
}

function RolePills({ person, catalog, canEdit, onAddRole, onRemoveRole }: RolePillsProps) {
  const [picking, setPicking] = useState(false);
  const [pos, setPos] = useState({ top: 0, left: 0 });
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Recompute the menu's viewport-relative position — called when opening
  // and on scroll while open. Using position: fixed at document.body level
  // (via portal) puts the menu above every other stacking context, so a
  // charcoal-tinted parent card can no longer bleed through.
  const updatePosition = useCallback(() => {
    const btn = buttonRef.current;
    if (!btn) return;
    const rect = btn.getBoundingClientRect();
    setPos({ top: rect.bottom + 4, left: rect.left });
  }, []);

  useEffect(() => {
    if (!picking) return;
    updatePosition();
    // Close on outside click (covers both the trigger area and the
    // portaled menu since both refs are checked).
    function onMouseDown(e: MouseEvent) {
      const t = e.target as Node;
      if (buttonRef.current?.contains(t)) return;
      if (menuRef.current?.contains(t)) return;
      setPicking(false);
    }
    // Reposition on scroll so the menu follows the trigger — same
    // behaviour the existing TeamRoleSelector uses.
    function onScroll() {
      updatePosition();
    }
    document.addEventListener("mousedown", onMouseDown);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      document.removeEventListener("mousedown", onMouseDown);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [picking, updatePosition]);

  const roles = person.roles ?? [];
  const assigned = new Set(roles.map((r) => r.role.toLowerCase()));
  const available = catalog.filter((r) => !assigned.has(r.value));

  // Gap pill: no roles at all, treat as "Needs verification".
  const showGapPill = roles.length === 0;

  if (roles.length === 0 && !canEdit) {
    // No roles and no edit affordance — render nothing for a truly empty state.
    return null;
  }

  return (
    <div className={css.rolePills}>
      {roles.map((r) => {
        const option = findRoleOption(catalog, r.role);
        const pillClasses = [css.rolePill, toneClass(option.tone)];
        if (r.dataSource !== "user") pillClasses.push(css.rolePillProvenanceAi);
        return (
          <span key={r.role} className={pillClasses.join(" ")}>
            {option.label}
            {canEdit && onRemoveRole ? (
              <button
                type="button"
                className={css.rolePillRemove}
                aria-label={`Remove ${option.label} role`}
                title={`Remove ${option.label}`}
                onClick={() => onRemoveRole(person.personId, r.role)}
              >
                ×
              </button>
            ) : null}
          </span>
        );
      })}
      {showGapPill ? (
        <span className={`${css.rolePill} ${css.rolePillGap}`}>Needs verification</span>
      ) : null}
      {canEdit && onAddRole && available.length > 0 ? (
        <>
          <button
            ref={buttonRef}
            type="button"
            className={css.rolePillAdd}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              setPicking((p) => !p);
            }}
          >
            + role
          </button>
          {picking
            ? createPortal(
                <div
                  ref={menuRef}
                  className={css.rolePickerMenu}
                  // Runtime coordinates keep the portal menu anchored to its trigger.
                  style={{
                    top: pos.top,
                    left: pos.left,
                  }}
                >
                  {available.map((r) => (
                    <button
                      key={r.value}
                      type="button"
                      className={css.rolePickerItem}
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        onAddRole(person.personId, r.value);
                        setPicking(false);
                      }}
                    >
                      {r.label}
                    </button>
                  ))}
                </div>,
                document.body,
              )
            : null}
        </>
      ) : null}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

function MetaFooter({
  meetingCount,
  lastSeen,
  emailCount,
}: {
  meetingCount: number | null;
  lastSeen: string | null;
  emailCount: number | null;
}) {
  const bits: string[] = [];
  if (lastSeen) bits.push(`Last in meeting · ${lastSeen}`);
  if (meetingCount !== null && meetingCount > 0) {
    bits.push(`${meetingCount} meeting${meetingCount === 1 ? "" : "s"}`);
  }
  if (emailCount !== null && emailCount > 0) {
    bits.push(`${emailCount} emails`);
  }
  if (bits.length === 0) return null;
  return (
    <div className={css.personFooter}>
      {bits.map((b, i) => (
        <span key={i}>{b}</span>
      ))}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

function buildInitials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0][0]?.toUpperCase() ?? "?";
  return (parts[0][0] ?? "").toUpperCase() + (parts[parts.length - 1][0] ?? "").toUpperCase();
}

function deriveLocation(person: StakeholderFull): string | null {
  // If we have a specific city/location field later, read it here. For now
  // the mockup shows "New York · external" — we don't have city, but we
  // can still show the internal/external marker when the org differs.
  // Return null to skip; an explicit schema field lives in DOS-249.
  void person;
  return null;
}

function formatLastSeen(iso: string | null | undefined): string | null {
  if (!iso) return null;
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return null;
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  } catch {
    return null;
  }
}

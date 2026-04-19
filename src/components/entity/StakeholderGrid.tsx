/**
 * StakeholderGrid — Context tab "The Room" chapter (v1.2.1 rebuild).
 *
 * Replaces the v1.0 StakeholderGallery (single-shape card stack, 1038 lines)
 * with the mockup's editorial primary/secondary grid split into Their team
 * + Our team subsections, plus a "+N more associated" tier-2 ellipsis row.
 *
 * Design decisions locked in with the user:
 *   - Card tier   = AI-ranked top 4 by signal (engagement + meetings + roles)
 *   - Role shape  = multi-role per person; pills carry provenance (AI vs user)
 *   - Health xref = hide the pill until a per-stakeholder anchor map exists
 *   - Extras      = "+N more associated" tier-2 row. No engagement dot, no
 *                   per-person quote block, no gap-state new logic beyond
 *                   what PersonCard already handles.
 *
 * Data integrity:
 *   Role mutations flow through `onAddRole` / `onRemoveRole` which call the
 *   atomic backend commands `add_stakeholder_role` / `remove_stakeholder_role`.
 *   No destructive "replace all" paths. The previous single-value dropdown
 *   (which deleted AI-surfaced rows on swap) is gone from this component.
 */
import { useState } from "react";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { AccountTeamMember, StakeholderFull, StakeholderSuggestion } from "@/types";
import { PersonCard, EXTERNAL_ROLE_CATALOG, INTERNAL_ROLE_CATALOG, type RoleOption } from "./PersonCard";
import css from "./StakeholderGrid.module.css";

const PRIMARY_COUNT = 4;

interface StakeholderGridProps {
  stakeholders?: StakeholderFull[];
  accountTeam?: AccountTeamMember[];
  /** Account name drives the italic hint next to subsection labels. */
  accountName?: string;
  /** Chapter title passed to the inline h2. Defaults to "The Room". */
  chapterTitle?: string;
  /** Optional freshness strip under the h2. */
  chapterFreshness?: React.ReactNode;

  /** Role mutations (wire to add_stakeholder_role / remove_stakeholder_role). */
  onAddRole?: (personId: string, role: string) => void;
  onRemoveRole?: (personId: string, role: string) => void;

  /** Internal team add/remove (preserved from the v1.0 component). */
  onAddTeamMember?: () => void;
  onRemoveTeamMember?: (personId: string, role: string) => void;

  /** AI-proposed stakeholder overrides awaiting human review (Phase 3). */
  suggestions?: StakeholderSuggestion[];
  onAcceptSuggestion?: (suggestionId: number) => void;
  onDismissSuggestion?: (suggestionId: number) => void;
}

export function StakeholderGrid({
  stakeholders,
  accountTeam,
  accountName,
  chapterTitle = "The Room",
  chapterFreshness,
  onAddRole,
  onRemoveRole,
  onAddTeamMember,
  onRemoveTeamMember,
  suggestions,
  onAcceptSuggestion,
  onDismissSuggestion,
}: StakeholderGridProps) {
  const external = stakeholders ?? [];
  const internal = accountTeam ?? [];

  // Ranking: engagement weight + multi-role weight + meeting-count log scale.
  // Highest-signal at the top. Deterministic — no AI call, just a rank based
  // on what the DB already carries.
  const ranked = [...external].sort(signalDescending);
  const primary = ranked.slice(0, PRIMARY_COUNT);
  const secondary = ranked.slice(PRIMARY_COUNT);

  // Tier-2: "associated" people who showed up in signals but have no role
  // assignment AND no assessment. Collapsed into the "+N more associated"
  // ellipsis row — the mockup's editorial signal that we know these people
  // exist but haven't characterized them.
  const tier2 = secondary.filter(isTier2);
  const secondaryRendered = secondary.filter((s) => !isTier2(s));

  const hasAnyExternal = primary.length > 0 || secondaryRendered.length > 0 || tier2.length > 0;
  const hasAnyInternal = internal.length > 0 || !!onAddTeamMember;

  return (
    <section className={css.section}>
      <ChapterHeading title={chapterTitle} freshness={chapterFreshness} />

      {hasAnyExternal ? (
        <>
          <div className={css.subsectionLabel}>
            Their team
            {accountName ? (
              <span className={css.subsectionHint}>
                who we're meeting with — {accountName}
              </span>
            ) : null}
          </div>
          <div className={css.roomGrid}>
            {primary.map((p) => (
              <PersonCard
                key={p.personId}
                person={p}
                variant="primary"
                roleCatalog={EXTERNAL_ROLE_CATALOG}
                onAddRole={onAddRole}
                onRemoveRole={onRemoveRole}
              />
            ))}
            {secondaryRendered.map((p) => (
              <PersonCard
                key={p.personId}
                person={p}
                variant="compact"
                roleCatalog={EXTERNAL_ROLE_CATALOG}
                onAddRole={onAddRole}
                onRemoveRole={onRemoveRole}
              />
            ))}
          </div>
          {tier2.length > 0 ? <MoreAssociatedRow people={tier2} /> : null}
        </>
      ) : null}

      <SuggestionsQueue
        suggestions={suggestions}
        onAccept={onAcceptSuggestion}
        onDismiss={onDismissSuggestion}
      />

      {hasAnyInternal ? (
        <>
          <div className={css.subsectionLabel}>
            Our team
            {accountName ? (
              <span className={css.subsectionHint}>
                who we bring into {accountName} conversations
              </span>
            ) : null}
          </div>
          <div className={css.roomGridInternal}>
            {internal.map((m) => (
              <PersonCard
                key={m.personId}
                person={teamMemberAsStakeholder(m)}
                variant="internal"
                roleCatalog={INTERNAL_ROLE_CATALOG}
                onAddRole={onAddRole}
                onRemoveRole={onRemoveRole}
                onRemoveMember={onRemoveTeamMember}
              />
            ))}
            {onAddTeamMember ? (
              <button
                type="button"
                className={css.internalAddRow}
                onClick={onAddTeamMember}
              >
                + add teammate
              </button>
            ) : null}
          </div>
        </>
      ) : null}

      {!hasAnyExternal && !hasAnyInternal ? (
        <p className={css.empty}>
          No stakeholders on file for this account yet. People surface here
          automatically the moment they appear on a meeting invite.
        </p>
      ) : null}
    </section>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

function MoreAssociatedRow({ people }: { people: StakeholderFull[] }) {
  // Compact ellipsis of tier-2 people. Show first three names inline, then
  // "… and N others" so the row stays one-line scannable.
  const [expanded, setExpanded] = useState(false);
  const names = people.map((p) => p.personName).filter((n) => !!n);
  const shown = expanded ? names : names.slice(0, 3);
  const hiddenCount = expanded ? 0 : Math.max(0, names.length - shown.length);
  return (
    <div className={css.moreAssociated}>
      <span className={css.moreAssociatedCount}>+{people.length}</span>
      more associated · {shown.join(" · ")}
      {hiddenCount > 0 ? (
        <>
          {" "}
          ·
          <button
            type="button"
            className={css.rolePillAdd}
            onClick={() => setExpanded(true)}
          >
            +{hiddenCount} more
          </button>
        </>
      ) : null}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────────────── */

function signalDescending(a: StakeholderFull, b: StakeholderFull): number {
  return signalScore(b) - signalScore(a);
}

function signalScore(s: StakeholderFull): number {
  const engagement = engagementWeight(s.engagement);
  const roleWeight = (s.roles?.length ?? 0) * 10;
  // Log scale on meetings so a highly-active person doesn't drown every
  // other signal.
  const meetingWeight = Math.log2((s.meetingCount ?? 0) + 1) * 5;
  const assessmentWeight = s.assessment && s.assessment.trim().length > 0 ? 5 : 0;
  return engagement + roleWeight + meetingWeight + assessmentWeight;
}

function engagementWeight(engagement: string | null | undefined): number {
  switch ((engagement ?? "").toLowerCase()) {
    case "high":
      return 30;
    case "medium":
      return 15;
    case "low":
      return 5;
    default:
      return 0;
  }
}

function isTier2(s: StakeholderFull): boolean {
  const hasRole = (s.roles?.length ?? 0) > 0;
  const hasAssessment = !!(s.assessment && s.assessment.trim().length > 0);
  return !hasRole && !hasAssessment;
}

/* ─────────────────────────────────────────────────────────────────────── */

/**
 * SuggestionsQueue — pending AI-proposed stakeholder overrides.
 *
 * AI discovers new people or disagrees with human-pinned assignments
 * during enrichment. Rather than silently overwriting user decisions,
 * the Intelligence Loop writes to `stakeholder_suggestions` where they
 * sit until a human accepts or dismisses. Without a UI surface for
 * this queue, the suggestions accumulate invisibly — this component
 * is that surface.
 *
 * Filters out non-pending rows (already resolved) and dedupes against
 * confirmed stakeholders handled upstream in useTeamManagement. Empty
 * set → renders nothing (no "no suggestions yet" placeholder to clutter
 * the chapter).
 */
function SuggestionsQueue({
  suggestions,
  onAccept,
  onDismiss,
}: {
  suggestions?: StakeholderSuggestion[];
  onAccept?: (suggestionId: number) => void;
  onDismiss?: (suggestionId: number) => void;
}) {
  const pending = (suggestions ?? []).filter((s) => s.status === "pending");
  if (pending.length === 0) return null;

  return (
    <section className={css.suggestionsSection}>
      <div className={css.suggestionsLabel}>
        Suggestions · AI-proposed additions
        <span className={css.suggestionsHint}>
          Accept to add to the team. Dismiss teaches the system.
        </span>
      </div>
      <div className={css.suggestionList}>
        {pending.map((s) => {
          const displayName = s.suggestedName || "Unnamed suggestion";
          const metaParts: string[] = [];
          if (s.suggestedRole) metaParts.push(roleLabelForMeta(s.suggestedRole));
          if (s.suggestedEmail) metaParts.push(s.suggestedEmail);
          if (s.source) metaParts.push(`via ${s.source}`);
          return (
            <div key={s.id} className={css.suggestionRow}>
              <div className={css.suggestionBody}>
                <div className={css.suggestionName}>{displayName}</div>
                {metaParts.length > 0 ? (
                  <div className={css.suggestionMeta}>
                    {metaParts.map((part, i) => (
                      <span key={i}>{part}</span>
                    ))}
                  </div>
                ) : null}
              </div>
              <div className={css.suggestionActions}>
                {onDismiss ? (
                  <button
                    type="button"
                    className={css.suggestionBtn}
                    onClick={() => onDismiss(s.id)}
                  >
                    Dismiss
                  </button>
                ) : null}
                {onAccept ? (
                  <button
                    type="button"
                    className={`${css.suggestionBtn} ${css.suggestionBtnPrimary}`}
                    onClick={() => onAccept(s.id)}
                  >
                    Accept
                  </button>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

/** Look up a role's display label across both catalogs for suggestion meta. */
function roleLabelForMeta(storedRole: string): string {
  const normalized = storedRole.trim().toLowerCase();
  const catalogs: RoleOption[][] = [EXTERNAL_ROLE_CATALOG, INTERNAL_ROLE_CATALOG];
  for (const catalog of catalogs) {
    const found = catalog.find((r) => r.value === normalized);
    if (found) return found.label;
  }
  // Unknown — humanise the stored value.
  return normalized
    .split(/[_\s]+/)
    .map((w) => (w.length > 0 ? w[0].toUpperCase() + w.slice(1) : ""))
    .join(" ");
}

/* ─────────────────────────────────────────────────────────────────────── */

function teamMemberAsStakeholder(m: AccountTeamMember): StakeholderFull {
  // Internal team members come from `get_account_team` which returns the
  // concatenated role string GROUP_CONCAT'd from account_stakeholder_roles
  // (e.g. "rm,csm" when a person carries multiple roles). Split back into
  // individual StakeholderRole entries so the chip editor renders one pill
  // per role — the old adapter treated the whole concatenated string as a
  // single role and broke the add/remove flow for internal members.
  //
  // `get_account_team` doesn't carry per-role `data_source`, so we assume
  // 'user' here. Internal team assignments are almost always human-pinned
  // (team management is a manual workflow); if the backend ever wires
  // per-role provenance we'll thread it through.
  const roleStrings = (m.role ?? "")
    .split(",")
    .map((r) => r.trim())
    .filter((r) => r.length > 0);
  return {
    personId: m.personId,
    personName: m.personName,
    personEmail: m.personEmail || null,
    organization: null,
    personRole: null,
    stakeholderRole: m.role,
    roles: roleStrings.map((role) => ({ role, dataSource: "user" })),
    dataSource: "user",
    engagement: null,
    dataSourceEngagement: null,
    assessment: null,
    dataSourceAssessment: null,
    lastSeenInGlean: null,
    createdAt: m.createdAt,
    linkedinUrl: null,
    photoUrl: null,
    meetingCount: null,
    lastSeen: null,
  };
}

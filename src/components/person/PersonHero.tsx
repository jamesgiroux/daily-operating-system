/**
 * PersonHero — editorial headline for a person.
 * Larkspur-tinted watermark, circular initial avatar, relationship + temperature badges.
 * Meta row: Refresh, Merge, Archive, Delete. Name and role are inline-editable.
 */
import { useState } from "react";
import type { PersonDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";
import { Avatar } from "@/components/ui/Avatar";
import styles from "./PersonHero.module.css";

interface PersonHeroProps {
  detail: PersonDetail;
  intelligence: EntityIntelligence | null;
  editName?: string;
  setEditName?: (value: string) => void;
  editRole?: string;
  setEditRole?: (value: string) => void;
  onSave?: () => void;
  onSaveField?: (field: string, value: string) => void;
  onEnrich?: () => void;
  enriching?: boolean;
  enrichSeconds?: number;
  onClayEnrich?: () => void;
  clayEnriching?: boolean;
  onMerge?: () => void;
  onArchive?: () => void;
  onUnarchive?: () => void;
  onDelete?: () => void;
}

const relationshipClass: Record<string, string> = {
  external: styles.relationshipExternal,
  internal: styles.relationshipInternal,
  unknown: styles.relationshipUnknown,
};

const temperatureClass: Record<string, string> = {
  hot: styles.temperatureHot,
  warm: styles.temperatureWarm,
  cool: styles.temperatureCool,
  cold: styles.temperatureCold,
};

export function PersonHero({
  detail,
  intelligence,
  editName,
  setEditName,
  editRole,
  setEditRole,
  onSave: _onSave,
  onSaveField,
  onEnrich,
  enriching,
  enrichSeconds,
  onClayEnrich,
  clayEnriching,
  onMerge,
  onArchive,
  onUnarchive,
  onDelete,
}: PersonHeroProps) {
  const ledeFull = intelligence?.executiveAssessment?.split("\n")[0] ?? null;
  const LEDE_LIMIT = 300;
  const [showFullLede, setShowFullLede] = useState(false);
  const ledeTruncated = !!ledeFull && ledeFull.length > LEDE_LIMIT && !showFullLede;
  const lede = ledeFull && ledeTruncated ? ledeFull.slice(0, LEDE_LIMIT) + "…" : ledeFull;
  const temperature = detail.signals?.temperature;

  // Build subtitle: email + org/role
  const subtitleParts: string[] = [];
  if (detail.email) subtitleParts.push(detail.email);
  const accounts = detail.entities?.filter((e) => e.entityType === "account") ?? [];
  const orgLabel = accounts.length > 0
    ? accounts.map((a) => a.name).join(", ")
    : detail.organization;
  if (orgLabel && detail.role) {
    subtitleParts.push(`${orgLabel} \u00B7 ${detail.role}`);
  } else if (orgLabel) {
    subtitleParts.push(orgLabel);
  } else if (detail.role) {
    subtitleParts.push(detail.role);
  }

  return (
    <div className={styles.hero}>
      {/* Archived banner */}
      {detail.archived && (
        <div className={styles.archivedBanner}>
          <span className={styles.archivedText}>
            This person is archived and hidden from active views.
          </span>
        </div>
      )}

      {/* Hero date / intelligence timestamp */}
      <div className={styles.heroDate}>
        <IntelligenceQualityBadge enrichedAt={intelligence?.enrichedAt} />
        {intelligence ? ` Last updated ${formatRelativeDateShort(intelligence.enrichedAt)}` : ""}
        {detail.lastEnrichedAt && ` \u00B7 Clay ${formatRelativeDateShort(detail.lastEnrichedAt)}`}
      </div>

      {/* Name with avatar */}
      <div className={styles.nameRow}>
        <Avatar name={detail.name} personId={detail.id} photoUrl={detail.photoUrl} size={48} />
        <h1 className={styles.name}>
          {editName != null && setEditName ? (
            <EditableText
              as="span"
              value={editName}
              onChange={(v) => { setEditName(v); onSaveField?.("name", v); }}
              multiline={false}
              placeholder="Full name"
              fieldId="person-name"
            />
          ) : (
            detail.name
          )}
        </h1>
      </div>

      {/* Lede from intelligence */}
      {lede && (
        <p className={styles.lede}>
          {lede}
          {ledeTruncated && (
            <button
              onClick={() => setShowFullLede(true)}
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
      )}

      {/* Subtitle: email, org, role (role is inline-editable) */}
      {(subtitleParts.length > 0 || (editRole != null && setEditRole)) && (
        <div className={styles.subtitle} style={{ display: "flex", alignItems: "baseline", gap: 6, flexWrap: "wrap" }}>
          {detail.email && <span>{detail.email}</span>}
          {detail.email && (orgLabel || editRole != null) && <span> — </span>}
          {orgLabel && <span>{orgLabel}</span>}
          {orgLabel && (editRole != null || detail.role) && <span> · </span>}
          {editRole != null && setEditRole ? (
            <EditableText
              as="span"
              value={editRole}
              onChange={(v) => { setEditRole(v); onSaveField?.("role", v); }}
              multiline={false}
              placeholder="Role / Title"
              fieldId="person-role"
            />
          ) : (
            detail.role && <span>{detail.role}</span>
          )}
        </div>
      )}

      {/* Bio (from Clay/Gravatar enrichment — shown below lede when both exist) */}
      {detail.bio && (
        <p style={{
          fontFamily: "var(--font-serif)",
          fontSize: 15,
          lineHeight: 1.6,
          color: "var(--color-text-secondary)",
          fontStyle: "italic",
          marginTop: lede ? 8 : 0,
        }}>
          {detail.bio.length > LEDE_LIMIT ? detail.bio.slice(0, LEDE_LIMIT) + "…" : detail.bio}
        </p>
      )}

      {/* Social links + phone */}
      {(detail.linkedinUrl || detail.twitterHandle || detail.phone) && (
        <div style={{ display: "flex", gap: 12, marginTop: 8, flexWrap: "wrap" }}>
          {detail.linkedinUrl && (
            <a
              href={detail.linkedinUrl}
              target="_blank"
              rel="noopener noreferrer"
              title={detail.linkedinUrl}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-larkspur)",
                textDecoration: "none",
              }}
            >
              LinkedIn ↗
            </a>
          )}
          {detail.twitterHandle && (
            <a
              href={`https://x.com/${detail.twitterHandle.replace(/^@/, "")}`}
              target="_blank"
              rel="noopener noreferrer"
              title={`https://x.com/${detail.twitterHandle.replace(/^@/, "")}`}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-larkspur)",
                textDecoration: "none",
              }}
            >
              @{detail.twitterHandle.replace(/^@/, "")} ↗
            </a>
          )}
          {detail.phone && (
            <a
              href={`tel:${detail.phone}`}
              title={detail.phone}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                color: "var(--color-text-secondary)",
                textDecoration: "none",
              }}
            >
              {detail.phone}
            </a>
          )}
        </div>
      )}

      {/* Badges row */}
      <div className={styles.badges} style={{ marginTop: lede ? 24 : 0 }}>
        <span className={`${styles.badge} ${relationshipClass[detail.relationship] ?? styles.relationshipUnknown}`}>
          {detail.relationship}
        </span>
        {temperature && (
          <span className={`${styles.badge} ${temperatureClass[temperature] ?? styles.temperatureCool}`}>
            {temperature}
          </span>
        )}
      </div>

      {/* Meta row */}
      <div className={styles.meta} style={{ display: "flex", alignItems: "baseline", gap: 16, flexWrap: "wrap", marginTop: 16 }}>
        {onEnrich && (
          <FolioRefreshButton
            onClick={onEnrich}
            loading={!!enriching}
            loadingProgress={enriching ? `${enrichSeconds ?? 0}s` : undefined}
          />
        )}
        {onClayEnrich && (
          <button
            className={clayEnriching ? styles.metaButtonEnriching : styles.metaButton}
            onClick={onClayEnrich}
            disabled={clayEnriching}
          >
            {clayEnriching ? "Updating from Clay…" : "Update from Clay"}
          </button>
        )}
        {onMerge && !detail.archived && (
          <button className={styles.metaButton} onClick={onMerge}>
            Merge Into…
          </button>
        )}
        {detail.archived && onUnarchive && (
          <button className={styles.metaButton} onClick={onUnarchive}>
            Unarchive
          </button>
        )}
        {!detail.archived && onArchive && (
          <button className={styles.metaButton} onClick={onArchive}>
            Archive
          </button>
        )}
        {onDelete && (
          <button className={styles.metaButtonDanger} onClick={onDelete}>
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

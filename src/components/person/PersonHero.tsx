/**
 * PersonHero — editorial headline for a person.
 * Larkspur-tinted watermark, circular initial avatar, relationship + temperature badges.
 * Meta row: Edit Details, Build Intelligence, Merge, Archive, Delete.
 */
import type { PersonDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { BrandMark } from "../ui/BrandMark";
import styles from "./PersonHero.module.css";

interface PersonHeroProps {
  detail: PersonDetail;
  intelligence: EntityIntelligence | null;
  onEditDetails?: () => void;
  onEnrich?: () => void;
  enriching?: boolean;
  enrichSeconds?: number;
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
  onEditDetails,
  onEnrich,
  enriching,
  enrichSeconds,
  onMerge,
  onArchive,
  onUnarchive,
  onDelete,
}: PersonHeroProps) {
  const lede = intelligence?.executiveAssessment?.split("\n")[0] ?? null;
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
      <div className={styles.watermark}><BrandMark size="100%" /></div>

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
        Person Intelligence
        {intelligence && ` \u00B7 Last enriched ${formatRelativeDateShort(intelligence.enrichedAt)}`}
      </div>

      {/* Name with avatar */}
      <div className={styles.nameRow}>
        <div className={styles.avatar}>
          {detail.name.charAt(0).toUpperCase()}
        </div>
        <h1 className={styles.name}>{detail.name}</h1>
      </div>

      {/* Lede from intelligence */}
      {lede && <p className={styles.lede}>{lede}</p>}

      {/* Subtitle: email, org, role */}
      {subtitleParts.length > 0 && (
        <p className={styles.subtitle}>{subtitleParts.join(" \u2014 ")}</p>
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
        {onEditDetails && (
          <button className={styles.metaButton} onClick={onEditDetails}>
            Edit Details
          </button>
        )}
        {onEnrich && (
          <button
            className={enriching ? styles.metaButtonEnriching : styles.metaButton}
            onClick={onEnrich}
            disabled={enriching}
          >
            {enriching ? `Building intelligence\u2026 ${enrichSeconds ?? 0}s` : "Build Intelligence"}
          </button>
        )}
        {onMerge && !detail.archived && (
          <button className={styles.metaButton} onClick={onMerge}>
            Merge Into\u2026
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

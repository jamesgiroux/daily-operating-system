/**
 * ProjectHero — editorial headline for a project.
 * Olive-tinted watermark, status badge, owner + target date below name.
 */
import type { ProjectDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { BrandMark } from "../ui/BrandMark";
import styles from "./ProjectHero.module.css";

interface ProjectHeroProps {
  detail: ProjectDetail;
  intelligence: EntityIntelligence | null;
  onEditFields?: () => void;
  onEnrich?: () => void;
  enriching?: boolean;
  enrichSeconds?: number;
  onArchive?: () => void;
  onUnarchive?: () => void;
}

const statusClass: Record<string, string> = {
  active: styles.statusActive,
  on_hold: styles.statusOnHold,
  completed: styles.statusCompleted,
};

function formatStatus(s: string): string {
  return s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export function ProjectHero({
  detail,
  intelligence,
  onEditFields,
  onEnrich,
  enriching,
  enrichSeconds,
  onArchive,
  onUnarchive,
}: ProjectHeroProps) {
  const lede = intelligence?.executiveAssessment?.split("\n")[0] ?? null;

  return (
    <div className={styles.hero}>
      <div className={styles.watermark}><BrandMark size="100%" /></div>

      {/* Archived banner */}
      {detail.archived && (
        <div className={styles.archivedBanner}>
          <span className={styles.archivedText}>
            This project is archived and hidden from active views.
          </span>
        </div>
      )}

      {/* Hero date / intelligence timestamp */}
      <div className={styles.heroDate}>
        Project Intelligence
        {intelligence && ` · Last enriched ${formatRelativeDateShort(intelligence.enrichedAt)}`}
      </div>

      {/* Project name — 76px serif */}
      <h1 className={styles.name}>{detail.name}</h1>

      {/* Lede from intelligence */}
      {lede && <p className={styles.lede}>{lede}</p>}

      {/* Badges row */}
      <div className={styles.badges} style={{ marginTop: lede ? 24 : 0 }}>
        {detail.status && (
          <span className={`${styles.badge} ${statusClass[detail.status] ?? styles.statusDefault}`}>
            {formatStatus(detail.status)}
          </span>
        )}
        {detail.owner && (
          <span className={`${styles.badge} ${styles.ownerBadge}`}>
            {detail.owner}
          </span>
        )}
      </div>

      {/* Meta row */}
      <div className={styles.meta} style={{ display: "flex", alignItems: "baseline", gap: 16, flexWrap: "wrap", marginTop: 16 }}>
        {onEditFields && (
          <button className={styles.metaButton} onClick={onEditFields}>
            Edit Fields
          </button>
        )}
        {onEnrich && (
          <button
            className={enriching ? styles.metaButtonEnriching : styles.metaButton}
            onClick={onEnrich}
            disabled={enriching}
          >
            {enriching ? `Building intelligence… ${enrichSeconds ?? 0}s` : "Build Intelligence"}
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
      </div>
    </div>
  );
}

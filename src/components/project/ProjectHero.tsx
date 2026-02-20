/**
 * ProjectHero — editorial headline for a project.
 * Olive-tinted watermark, status badge, owner + target date below name.
 */
import { useState } from "react";
import type { ProjectDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import styles from "./ProjectHero.module.css";

interface ProjectHeroProps {
  detail: ProjectDetail;
  intelligence: EntityIntelligence | null;
  editName?: string;
  setEditName?: (value: string) => void;
  editStatus?: string;
  setEditStatus?: (value: string) => void;
  onSave?: () => void;
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
  editName,
  setEditName,
  editStatus,
  setEditStatus,
  onSave,
  onEnrich,
  enriching,
  enrichSeconds,
  onArchive,
  onUnarchive,
}: ProjectHeroProps) {
  const ledeFull = intelligence?.executiveAssessment?.split("\n")[0] ?? null;
  const LEDE_LIMIT = 300;
  const [showFullLede, setShowFullLede] = useState(false);
  const ledeTruncated = !!ledeFull && ledeFull.length > LEDE_LIMIT && !showFullLede;
  const lede = ledeFull && ledeTruncated ? ledeFull.slice(0, LEDE_LIMIT) + "…" : ledeFull;

  return (
    <div className={styles.hero}>
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
        <IntelligenceQualityBadge enrichedAt={intelligence?.enrichedAt} />
        {intelligence ? ` Last updated ${formatRelativeDateShort(intelligence.enrichedAt)}` : ""}
      </div>

      {/* Project name — 76px serif, inline-editable */}
      <h1 className={styles.name}>
        {editName != null && setEditName ? (
          <EditableText
            as="span"
            value={editName}
            onChange={(v) => { setEditName(v); onSave?.(); }}
            multiline={false}
            placeholder="Project name"
            fieldId="project-name"
          />
        ) : (
          detail.name
        )}
      </h1>

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

      {/* Badges row — always show when editable, with placeholder for empty status */}
      <div className={styles.badges} style={{ marginTop: lede ? 24 : 0 }}>
        {((editStatus ?? detail.status) || setEditStatus) && (
          <span
            className={`${styles.badge} ${statusClass[editStatus ?? detail.status ?? ""] ?? styles.statusDefault}`}
            onClick={() => {
              if (!setEditStatus) return;
              const cycle = ["active", "on_hold", "completed"];
              const current = editStatus ?? detail.status ?? "";
              const idx = cycle.indexOf(current);
              const next = cycle[(idx + 1) % cycle.length];
              setEditStatus(next);
              onSave?.();
            }}
            style={{
              cursor: setEditStatus ? "pointer" : "default",
              opacity: (editStatus ?? detail.status) ? 1 : 0.4,
              borderStyle: (editStatus ?? detail.status) ? undefined : "dashed",
            }}
            title={setEditStatus ? "Click to set status" : undefined}
          >
            {formatStatus(editStatus ?? detail.status ?? "Status")}
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
        {onEnrich && (
          <button
            className={enriching ? styles.metaButtonEnriching : styles.metaButton}
            onClick={onEnrich}
            disabled={enriching}
          >
            {enriching ? `Refreshing… ${enrichSeconds ?? 0}s` : "Refresh"}
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

/**
 * ProjectHero — editorial headline for a project.
 * Olive-tinted watermark, status badge, owner + target date below name.
 * Inline editable fields for name, status, milestone, owner, target date.
 */
import { useState } from "react";
import type { ProjectDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import { CyclingPill } from "@/components/ui/CyclingPill";
import { DatePicker } from "@/components/ui/date-picker";
import styles from "./ProjectHero.module.css";

const statusOptions = ["active", "on_hold", "completed", "archived"];

const statusColorMap: Record<string, string> = {
  active: "var(--color-garden-sage)",
  on_hold: "var(--color-spice-turmeric)",
  completed: "var(--color-garden-larkspur)",
  archived: "var(--color-text-tertiary)",
};

const inlineFieldLabelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 9,
  fontWeight: 600,
  textTransform: "uppercase",
  letterSpacing: "0.08em",
  color: "var(--color-text-tertiary)",
  marginBottom: 2,
};

interface ProjectHeroProps {
  detail: ProjectDetail;
  intelligence: EntityIntelligence | null;
  editName: string;
  onNameChange: (v: string) => void;
  editStatus: string;
  onStatusChange: (v: string) => void;
  editMilestone: string;
  onMilestoneChange: (v: string) => void;
  editOwner: string;
  onOwnerChange: (v: string) => void;
  editTargetDate: string;
  onTargetDateChange: (v: string) => void;
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

export function ProjectHero({
  detail,
  intelligence,
  editName,
  onNameChange,
  editStatus,
  onStatusChange,
  editMilestone,
  onMilestoneChange,
  editOwner,
  onOwnerChange,
  editTargetDate,
  onTargetDateChange,
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
  const lede = ledeFull && ledeTruncated ? ledeFull.slice(0, LEDE_LIMIT) + "..." : ledeFull;

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

      {/* Project name — 76px serif, inline editable */}
      <EditableText
        value={editName}
        onChange={onNameChange}
        as="h1"
        multiline={false}
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 76,
          fontWeight: 400,
          letterSpacing: "-0.025em",
          lineHeight: 1.06,
          color: "var(--color-text-primary)",
          margin: "0 0 40px",
        }}
      />

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

      {/* Badges row */}
      <div className={styles.badges} style={{ marginTop: lede ? 24 : 0 }}>
        {detail.status && (
          <span className={`${styles.badge} ${statusClass[detail.status] ?? styles.statusDefault}`}>
            {detail.status.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}
          </span>
        )}
        {detail.owner && (
          <span className={`${styles.badge} ${styles.ownerBadge}`}>
            {detail.owner}
          </span>
        )}
      </div>

      {/* Inline editable fields */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-end",
          gap: 20,
          flexWrap: "wrap",
          marginTop: 8,
          marginBottom: 8,
        }}
      >
        <div>
          <div style={inlineFieldLabelStyle}>Status</div>
          <CyclingPill
            options={statusOptions}
            value={editStatus}
            onChange={onStatusChange}
            colorMap={statusColorMap}
          />
        </div>
        <div>
          <div style={inlineFieldLabelStyle}>Milestone</div>
          <EditableText
            value={editMilestone}
            onChange={onMilestoneChange}
            as="span"
            multiline={false}
            placeholder="--"
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-secondary)",
            }}
          />
        </div>
        <div>
          <div style={inlineFieldLabelStyle}>Owner</div>
          <EditableText
            value={editOwner}
            onChange={onOwnerChange}
            as="span"
            multiline={false}
            placeholder="--"
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-secondary)",
            }}
          />
        </div>
        <div>
          <div style={inlineFieldLabelStyle}>Target Date</div>
          <DatePicker
            value={editTargetDate}
            onChange={onTargetDateChange}
            placeholder="Not set"
          />
        </div>
      </div>

      {/* Meta row */}
      <div className={styles.meta} style={{ display: "flex", alignItems: "baseline", gap: 16, flexWrap: "wrap", marginTop: 16 }}>
        {onEnrich && (
          <button
            className={enriching ? styles.metaButtonEnriching : styles.metaButton}
            onClick={onEnrich}
            disabled={enriching}
          >
            {enriching ? `Refreshing... ${enrichSeconds ?? 0}s` : "Refresh"}
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

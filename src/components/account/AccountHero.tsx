/**
 * AccountHero — editorial headline for an account.
 * Mockup: h1 76px serif, 2-3 sentence italic lede from intelligence,
 * hero-date line, watermark asterisk, health/lifecycle badges, and meta row.
 * Inline editable fields below badges for health, lifecycle, ARR, NPS, renewal.
 */
import { useState } from "react";
import { Link } from "@tanstack/react-router";
import type { AccountDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import { CyclingPill } from "@/components/ui/CyclingPill";
import { DatePicker } from "@/components/ui/date-picker";
import styles from "./AccountHero.module.css";

const healthOptions = ["green", "yellow", "red"];
const lifecycleOptions = ["onboarding", "adoption", "nurture", "renewal", "churned"];

const healthColorMap: Record<string, string> = {
  green: "var(--color-garden-sage)",
  yellow: "var(--color-spice-turmeric)",
  red: "var(--color-spice-terracotta)",
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

interface AccountHeroProps {
  detail: AccountDetail;
  intelligence: EntityIntelligence | null;
  editHealth: string;
  onHealthChange: (v: string) => void;
  editLifecycle: string;
  onLifecycleChange: (v: string) => void;
  editArr: string;
  onArrChange: (v: string) => void;
  editNps: string;
  onNpsChange: (v: string) => void;
  editRenewal: string;
  onRenewalChange: (v: string) => void;
  onManageTeam?: () => void;
  onEnrich?: () => void;
  enriching?: boolean;
  enrichSeconds?: number;
  onArchive?: () => void;
  onUnarchive?: () => void;
}

const healthClass: Record<string, string> = {
  green: styles.healthGreen,
  yellow: styles.healthYellow,
  red: styles.healthRed,
};

export function AccountHero({
  detail,
  intelligence,
  editHealth,
  onHealthChange,
  editLifecycle,
  onLifecycleChange,
  editArr,
  onArrChange,
  editNps,
  onNpsChange,
  editRenewal,
  onRenewalChange,
  onManageTeam,
  onEnrich,
  enriching,
  enrichSeconds,
  onArchive,
  onUnarchive,
}: AccountHeroProps) {
  // Extract first paragraph of executive assessment as lede
  const ledeFull = intelligence?.executiveAssessment?.split("\n")[0] ?? null;
  const LEDE_LIMIT = 300;
  const [showFullLede, setShowFullLede] = useState(false);
  const ledeTruncated = !!ledeFull && ledeFull.length > LEDE_LIMIT && !showFullLede;
  const lede = ledeFull && ledeTruncated ? ledeFull.slice(0, LEDE_LIMIT) + "..." : ledeFull;
  return (
    <div className={styles.hero}>
      {/* Parent breadcrumb */}
      {detail.parentId && detail.parentName && (
        <Link
          to="/accounts/$accountId"
          params={{ accountId: detail.parentId }}
          className={styles.parentLink}
        >
          &larr; {detail.parentName}
        </Link>
      )}

      {/* Archived banner */}
      {detail.archived && (
        <div className={styles.archivedBanner}>
          <span className={styles.archivedText}>
            This account is archived and hidden from active views.
          </span>
        </div>
      )}

      {/* Hero date / intelligence timestamp */}
      <div className={styles.heroDate}>
        <IntelligenceQualityBadge enrichedAt={intelligence?.enrichedAt} />
        {intelligence ? ` Last updated ${formatRelativeDateShort(intelligence.enrichedAt)}` : ""}
      </div>

      {/* Account name — 76px serif */}
      <h1 className={styles.name}>{detail.name}</h1>

      {/* Lede from intelligence — italic serif */}
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
        {detail.health && (
          <span className={`${styles.badge} ${healthClass[detail.health] ?? ""}`}>
            {detail.health}
          </span>
        )}
        {detail.lifecycle && (
          <span className={`${styles.badge} ${styles.lifecycleBadge}`}>
            {detail.lifecycle}
          </span>
        )}
        {detail.isInternal && (
          <span className={`${styles.badge} ${styles.internalBadge}`}>
            Internal
          </span>
        )}
      </div>

      {/* Inline editable fields */}
      {!detail.isInternal && (
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
            <div style={inlineFieldLabelStyle}>Health</div>
            <CyclingPill
              options={healthOptions}
              value={editHealth}
              onChange={onHealthChange}
              colorMap={healthColorMap}
              placeholder="Not set"
            />
          </div>
          <div>
            <div style={inlineFieldLabelStyle}>Lifecycle</div>
            <CyclingPill
              options={lifecycleOptions}
              value={editLifecycle}
              onChange={onLifecycleChange}
              placeholder="Not set"
            />
          </div>
          <div>
            <div style={inlineFieldLabelStyle}>ARR</div>
            <EditableText
              value={editArr}
              onChange={onArrChange}
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
            <div style={inlineFieldLabelStyle}>NPS</div>
            <EditableText
              value={editNps}
              onChange={onNpsChange}
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
            <div style={inlineFieldLabelStyle}>Renewal</div>
            <DatePicker
              value={editRenewal}
              onChange={onRenewalChange}
              placeholder="Not set"
            />
          </div>
        </div>
      )}

      {/* Meta row: action links */}
      <div className={styles.meta} style={{ display: "flex", alignItems: "baseline", gap: 16, flexWrap: "wrap", marginTop: 16 }}>
        {onManageTeam && (
          <button className={styles.metaButton} onClick={onManageTeam}>
            Manage Team
          </button>
        )}
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

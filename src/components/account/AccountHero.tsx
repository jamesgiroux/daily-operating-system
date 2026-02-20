/**
 * AccountHero — editorial headline for an account.
 * Mockup: h1 76px serif, 2-3 sentence italic lede from intelligence,
 * hero-date line, watermark asterisk, health/lifecycle badges, and meta row.
 */
import { useState } from "react";
import { Link } from "@tanstack/react-router";
import type { AccountDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import styles from "./AccountHero.module.css";

interface AccountHeroProps {
  detail: AccountDetail;
  intelligence: EntityIntelligence | null;
  editName?: string;
  setEditName?: (value: string) => void;
  editHealth?: string;
  setEditHealth?: (value: string) => void;
  editLifecycle?: string;
  setEditLifecycle?: (value: string) => void;
  onSave?: () => void;
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
  editName,
  setEditName,
  editHealth,
  setEditHealth,
  editLifecycle,
  setEditLifecycle,
  onSave,
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
  const lede = ledeFull && ledeTruncated ? ledeFull.slice(0, LEDE_LIMIT) + "…" : ledeFull;
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

      {/* Account name — 76px serif, inline-editable */}
      <h1 className={styles.name}>
        {editName != null && setEditName ? (
          <EditableText
            as="span"
            value={editName}
            onChange={(v) => { setEditName(v); onSave?.(); }}
            multiline={false}
            placeholder="Account name"
            fieldId="account-name"
          />
        ) : (
          detail.name
        )}
      </h1>

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
        {(editHealth ?? detail.health) && (
          <span
            className={`${styles.badge} ${healthClass[editHealth ?? detail.health ?? ""] ?? ""}`}
            onClick={() => {
              if (!setEditHealth) return;
              const cycle = ["green", "yellow", "red"];
              const current = editHealth ?? detail.health ?? "green";
              const next = cycle[(cycle.indexOf(current) + 1) % cycle.length];
              setEditHealth(next);
              onSave?.();
            }}
            style={{ cursor: setEditHealth ? "pointer" : "default" }}
            title={setEditHealth ? "Click to cycle health" : undefined}
          >
            {editHealth ?? detail.health}
          </span>
        )}
        {(editLifecycle ?? detail.lifecycle) && (
          <span
            className={`${styles.badge} ${styles.lifecycleBadge}`}
            onClick={() => {
              if (!setEditLifecycle) return;
              const stages = ["Onboarding", "Adopted", "Expanding", "Renewing", "At Risk"];
              const current = editLifecycle ?? detail.lifecycle ?? "";
              const idx = stages.findIndex(s => s.toLowerCase() === current.toLowerCase());
              const next = stages[(idx + 1) % stages.length];
              setEditLifecycle(next);
              onSave?.();
            }}
            style={{ cursor: setEditLifecycle ? "pointer" : "default" }}
            title={setEditLifecycle ? "Click to cycle lifecycle" : undefined}
          >
            {editLifecycle ?? detail.lifecycle}
          </span>
        )}
        {detail.isInternal && (
          <span className={`${styles.badge} ${styles.internalBadge}`}>
            Internal
          </span>
        )}
      </div>

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

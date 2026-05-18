/**
 * AccountHero — editorial headline for an account.
 * Mockup: h1 76px serif, 2-3 sentence italic lede from intelligence,
 * hero-date line, watermark asterisk, health/lifecycle badges, and meta row.
 */
import React from "react";
import { Link } from "@tanstack/react-router";
import type { AccountDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import { TypeBadge } from "@/components/ui/TypeBadge";
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
  onSaveField?: (field: string, value: string) => void;
  /** Slot for vitals strip, rendered between name and lede */
  vitalsSlot?: React.ReactNode;
  provenanceSlot?: React.ReactNode;
}


export function AccountHero({
  detail,
  intelligence,
  editName,
  setEditName,
  editHealth: _editHealth,
  setEditHealth: _setEditHealth,
  editLifecycle: _editLifecycle,
  setEditLifecycle: _setEditLifecycle,
  onSave: _onSave,
  onSaveField,
  vitalsSlot,
  provenanceSlot,
}: AccountHeroProps) {
  // NOTE: Executive assessment narrative moved to AccountExecutiveSummary
  // component, rendered in the Context view.

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

      {/* Hero date / intelligence timestamp + account type badge */}
      <div className={`${styles.heroDate} ${styles.heroDateLayout}`}>
        <IntelligenceQualityBadge enrichedAt={intelligence?.enrichedAt} />
        {(() => {
          if (!intelligence) return "";
          const at = intelligence.enrichedAt;
          if (!at) return "";
          const relative = formatRelativeDateShort(at);
          if (relative) return ` Last updated ${relative}`;
          return "";
        })()}
        {onSaveField && (
          <TypeBadge
            value={detail.accountType}
            onChange={(v) => onSaveField("account_type", v)}
          />
        )}
      </div>

      {/* Account name — 76px serif */}
      <h1 className={styles.name}>
        {editName != null && setEditName ? (
          <EditableText
            as="span"
            value={editName}
            onChange={(v) => { setEditName(v); onSaveField?.("name", v); }}
            multiline={false}
            placeholder="Account name"
            fieldId="account-name"
          />
        ) : (
          detail.name
        )}
      </h1>

      {/* Vitals strip below name — narrative moved to AccountExecutiveSummary (Context view) */}
      {vitalsSlot}
      {provenanceSlot ? <div className={styles.provenance}>{provenanceSlot}</div> : null}
    </div>
  );
}


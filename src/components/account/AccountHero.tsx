/**
 * AccountHero — editorial headline for an account.
 * Mockup: h1 76px serif, 2-3 sentence italic lede from intelligence,
 * hero-date line, watermark asterisk, health/lifecycle badges, and meta row.
 */
import { useState, useRef, useEffect } from "react";
import { Link } from "@tanstack/react-router";
import type { AccountDetail, EntityIntelligence } from "@/types";
import { formatRelativeDate as formatRelativeDateShort } from "@/lib/utils";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { EditableText } from "@/components/ui/EditableText";
import { HealthBadge } from "@/components/shared/HealthBadge";
import { ChevronDown } from "lucide-react";
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
  setEditHealth: _setEditHealth,
  editLifecycle,
  setEditLifecycle: _setEditLifecycle,
  onSave: _onSave,
  onSaveField,
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
            onChange={(v) => { setEditName(v); onSaveField?.("name", v); }}
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

      {/* Badges row — read-only display (editing happens via VitalsStrip) */}
      <div className={styles.badges} style={{ marginTop: lede ? 24 : 0 }}>
        {(editHealth ?? detail.health) && (
          <span
            className={`${styles.badge} ${healthClass[editHealth ?? detail.health ?? ""] ?? ""}`}
          >
            {editHealth ?? detail.health}
          </span>
        )}
        {(editLifecycle ?? detail.lifecycle) && (
          <span className={`${styles.badge} ${styles.lifecycleBadge}`}>
            {editLifecycle ?? detail.lifecycle}
          </span>
        )}
        {onSaveField ? (
          <AccountTypeBadge
            value={detail.accountType as "customer" | "internal" | "partner"}
            onChange={(v) => onSaveField("account_type", v)}
          />
        ) : (
          <span className={`${styles.badge} ${
            detail.accountType === "internal" ? styles.internalBadge
            : detail.accountType === "partner" ? styles.partnerBadge
            : styles.lifecycleBadge
          }`}>
            {detail.accountType === "internal" ? "Internal"
             : detail.accountType === "partner" ? "Partner"
             : "Customer"}
          </span>
        )}
      </div>

      {/* Intelligence health (hero size) — I502 */}
      {intelligence?.health && (
        <div style={{ marginTop: 24 }}>
          <HealthBadge
            score={intelligence.health.score}
            band={intelligence.health.band}
            trend={intelligence.health.trend}
            confidence={intelligence.health.confidence}
            source={intelligence.health.source === "org" ? "Org data" : intelligence.health.source === "userSet" ? "Your assessment" : undefined}
            divergence={intelligence.health.divergence}
            size="hero"
          />
        </div>
      )}

      {/* Meta row: action links */}
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

// ─── Account Type Badge (inline dropdown) ──────────────────────────────────

const ACCOUNT_TYPES: { value: "customer" | "internal" | "partner"; label: string; badgeClass: string; color: string }[] = [
  { value: "customer", label: "Customer", badgeClass: "lifecycleBadge", color: "var(--color-text-secondary)" },
  { value: "internal", label: "Internal", badgeClass: "internalBadge", color: "var(--color-spice-turmeric)" },
  { value: "partner", label: "Partner", badgeClass: "partnerBadge", color: "var(--color-garden-rosemary)" },
];

function AccountTypeBadge({
  value,
  onChange,
}: {
  value: "customer" | "internal" | "partner";
  onChange: (v: "customer" | "internal" | "partner") => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  const current = ACCOUNT_TYPES.find((t) => t.value === value) ?? ACCOUNT_TYPES[0];

  return (
    <div ref={ref} style={{ position: "relative", display: "inline-flex" }}>
      <button
        className={`${styles.badge} ${styles[current.badgeClass]}`}
        style={{
          cursor: "pointer",
          border: "none",
          display: "inline-flex",
          alignItems: "center",
          gap: 3,
        }}
        onClick={() => setOpen(!open)}
      >
        {current.label}
        <ChevronDown size={10} strokeWidth={2} style={{ opacity: 0.5 }} />
      </button>
      {open && (
        <div
          style={{
            position: "absolute",
            top: "calc(100% + 4px)",
            left: 0,
            background: "var(--color-paper-cream)",
            border: "1px solid var(--color-rule-light)",
            borderRadius: 4,
            boxShadow: "var(--shadow-lg)",
            zIndex: 50,
            minWidth: 120,
            padding: "4px 0",
          }}
        >
          {ACCOUNT_TYPES.map((opt) => (
            <button
              key={opt.value}
              onClick={() => { onChange(opt.value); setOpen(false); }}
              style={{
                display: "block",
                width: "100%",
                textAlign: "left",
                padding: "6px 12px",
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: opt.value === value ? 600 : 400,
                letterSpacing: "0.08em",
                textTransform: "uppercase",
                color: opt.value === value ? opt.color : "var(--color-text-tertiary)",
                background: opt.value === value ? "var(--color-desk-charcoal-4)" : "transparent",
                border: "none",
                cursor: "pointer",
              }}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

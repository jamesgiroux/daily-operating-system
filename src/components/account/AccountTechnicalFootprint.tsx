import { formatShortDate } from "@/lib/utils";
import type { AccountTechnicalFootprint as TechnicalFootprintData } from "@/types";

import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountTechnicalFootprintProps {
  footprint: TechnicalFootprintData;
  /** DOS-18: render as a full chapter surface — ref-grid with gap rows + feature list. */
  variant?: "inline" | "chapter";
  /** DOS-18: feature list from productAdoption.featureAdoption (chapter variant only). */
  featureAdoption?: string[];
}

interface RefRow {
  label: string;
  value: string;
  gap?: boolean;
}

export function AccountTechnicalFootprint({ footprint, variant = "inline", featureAdoption }: AccountTechnicalFootprintProps) {
  const tf = footprint;

  if (variant === "chapter") {
    const rows: RefRow[] = [
      { label: "Usage tier", value: tf.usageTier ?? "— not captured", gap: !tf.usageTier },
      { label: "Active users", value: tf.activeUsers != null && tf.activeUsers > 0 ? tf.activeUsers.toLocaleString() : "— not captured", gap: !(tf.activeUsers && tf.activeUsers > 0) },
      { label: "Services stage", value: tf.servicesStage ?? "— not captured", gap: !tf.servicesStage },
      { label: "Support tier", value: tf.supportTier ?? "— not captured", gap: !tf.supportTier },
      { label: "Open tickets", value: tf.openTickets != null ? String(tf.openTickets) : "— not captured", gap: tf.openTickets == null },
      { label: "CSAT", value: tf.csatScore != null && tf.csatScore > 0 ? `${tf.csatScore.toFixed(1)}/5` : "— not captured", gap: !(tf.csatScore && tf.csatScore > 0) },
      { label: "Adoption score", value: tf.adoptionScore != null && tf.adoptionScore > 0 ? `${Math.round(tf.adoptionScore * 100)}%` : "— not computed", gap: !(tf.adoptionScore && tf.adoptionScore > 0) },
    ];
    const gapCount = rows.filter((r) => r.gap).length;

    return (
      <div>
        <div className={styles.technicalFootprintGrid} style={{ display: "grid", gridTemplateColumns: "1fr 1fr", rowGap: 8, columnGap: 32 }}>
          {rows.map((row) => (
            <div key={row.label} style={{ display: "flex", justifyContent: "space-between", padding: "6px 0", borderBottom: "1px solid var(--color-rule-light)" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
                {row.label}
              </span>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: row.gap ? "var(--color-text-tertiary)" : "var(--color-text-primary)", fontStyle: row.gap ? "italic" : "normal", textAlign: "right" }}>
                {row.value}
              </span>
            </div>
          ))}
        </div>

        {featureAdoption && featureAdoption.length > 0 && (
          <>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 500, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", margin: "24px 0 12px" }}>
              Feature adoption · {featureAdoption.length} active
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "6px 16px" }}>
              {featureAdoption.map((feature) => (
                <div key={feature} style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)", display: "flex", alignItems: "center", gap: 8 }}>
                  <span aria-hidden style={{ width: 6, height: 6, borderRadius: "50%", background: "var(--color-garden-sage)", flexShrink: 0 }} />
                  {feature}
                </div>
              ))}
            </div>
            {tf.sourcedAt && (
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)", marginTop: 12 }}>
                All {featureAdoption.length} features active as of {formatShortDate(tf.sourcedAt)}
                {tf.source ? ` (${tf.source})` : ""}
              </div>
            )}
          </>
        )}

        {gapCount > 0 && (
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-spice-saffron)", marginTop: 20, padding: "8px 12px", background: "var(--color-spice-saffron-8, rgba(196,147,53,0.06))", border: "1px dashed var(--color-spice-saffron)" }}>
            {gapCount} of {rows.length} technical fields unfilled · Last enrichment {formatShortDate(tf.sourcedAt)}
          </div>
        )}
      </div>
    );
  }

  const items: { label: string; value: string }[] = [];
  if (tf.supportTier) items.push({ label: "Support", value: tf.supportTier });
  if (tf.csatScore != null && tf.csatScore > 0) items.push({ label: "CSAT", value: `${tf.csatScore.toFixed(1)}/5` });
  if (tf.openTickets != null && tf.openTickets > 0) items.push({ label: "Open Tickets", value: String(tf.openTickets) });
  if (tf.usageTier) items.push({ label: "Usage Tier", value: tf.usageTier });
  if (tf.activeUsers != null && tf.activeUsers > 0) items.push({ label: "Active Users", value: tf.activeUsers.toLocaleString() });
  if (tf.adoptionScore != null && tf.adoptionScore > 0) items.push({ label: "Adoption", value: `${Math.round(tf.adoptionScore * 100)}%` });
  if (tf.servicesStage) items.push({ label: "Services", value: tf.servicesStage });

  if (items.length === 0) return null;

  return (
    <div className={styles.technicalFootprint}>
      <div className={styles.technicalFootprintLabel}>Technical Footprint</div>
      <div className={styles.technicalFootprintGrid}>
        {items.map((item) => (
          <div key={item.label} className={styles.technicalFootprintItem}>
            <span className={styles.technicalFootprintItemLabel}>{item.label}</span>
            <span className={styles.technicalFootprintItemValue}>{item.value}</span>
          </div>
        ))}
      </div>
      <div className={styles.technicalFootprintSource}>
        Source: {tf.source} &middot; {formatShortDate(tf.sourcedAt)}
      </div>
    </div>
  );
}

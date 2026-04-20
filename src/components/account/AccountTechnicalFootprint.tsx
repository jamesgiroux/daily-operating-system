import { formatShortDate } from "@/lib/utils";
import type { AccountProduct, AccountTechnicalFootprint as TechnicalFootprintData } from "@/types";

import inlineStyles from "@/pages/AccountDetailEditorial.module.css";
import refCss from "@/components/context/ReferenceGrid.module.css";

interface AccountTechnicalFootprintProps {
  /**
   * DOS-18: chapter variant accepts null so the chapter can always render
   * with all-gap rows when the account has no captured technical footprint.
   * Inline variant still requires a populated footprint (guarded below).
   */
  footprint: TechnicalFootprintData | null;
  /** DOS-18: render as a full chapter surface — ref-grid with gap rows + feature list. */
  variant?: "inline" | "chapter";
  /** DOS-18: feature list from productAdoption.featureAdoption (chapter variant only). */
  featureAdoption?: string[];
  /**
   * Products owned by the account, rendered as a dotted list alongside
   * Feature adoption. Dot color reflects status (active / trial / churned).
   * Chapter variant only. Full edit UX (status dropdown, product-level
   * Bayesian feedback) is tracked in DOS-251 for v1.2.2.
   */
  products?: AccountProduct[];
  /**
   * DOS-231: reserved for structured capture flow; full wiring lands in
   * DOS-251 (v1.2.2). Currently unused on the chapter variant — gap rows
   * render as read-only saffron sentinels.
   */
  onCaptureGap?: (field: string) => void;
}

function productDotClass(status: string): string {
  switch (status.toLowerCase()) {
    case "active":
      return refCss.featureDot;
    case "trial":
      return `${refCss.featureDot} ${refCss.featureDotTrial}`;
    case "churned":
      return `${refCss.featureDot} ${refCss.featureDotChurned}`;
    default:
      return refCss.featureDot;
  }
}

interface RefRow {
  label: string;
  field: string;
  value: string;
  gap?: boolean;
}

export function AccountTechnicalFootprint({ footprint, variant = "inline", featureAdoption, products }: AccountTechnicalFootprintProps) {
  const tf = footprint;

  if (variant === "chapter") {
    const rows: RefRow[] = [
      { label: "Usage tier", field: "usage_tier", value: tf?.usageTier ?? "— not captured", gap: !tf?.usageTier },
      { label: "Active users", field: "active_users", value: tf?.activeUsers != null && tf.activeUsers > 0 ? tf.activeUsers.toLocaleString() : "— not captured", gap: !(tf?.activeUsers && tf.activeUsers > 0) },
      { label: "Services stage", field: "services_stage", value: tf?.servicesStage ?? "— not captured", gap: !tf?.servicesStage },
      { label: "Support tier", field: "support_tier", value: tf?.supportTier ?? "— not captured", gap: !tf?.supportTier },
      { label: "Open tickets", field: "open_tickets", value: tf?.openTickets != null ? String(tf.openTickets) : "— not captured", gap: tf?.openTickets == null },
      { label: "CSAT", field: "csat_score", value: tf?.csatScore != null && tf.csatScore > 0 ? `${tf.csatScore.toFixed(1)}/5` : "— not captured", gap: !(tf?.csatScore && tf.csatScore > 0) },
      { label: "Adoption score", field: "adoption_score", value: tf?.adoptionScore != null && tf.adoptionScore > 0 ? `${Math.round(tf.adoptionScore * 100)}%` : "— not computed", gap: !(tf?.adoptionScore && tf.adoptionScore > 0) },
      { label: "Integrations", field: "integrations", value: "— not captured", gap: true },
    ];

    return (
      <div>
        <div className={refCss.grid}>
          {rows.map((row) => (
            <div key={row.label} className={refCss.row}>
              <span className={refCss.label}>{row.label}</span>
              <span className={row.gap ? `${refCss.value} ${refCss.valueGap}` : refCss.value}>
                {row.value}
              </span>
            </div>
          ))}
        </div>

        {products && products.length > 0 && (
          <>
            <div className={refCss.featureHeading}>
              Products · {products.length}
            </div>
            <div className={refCss.featureList}>
              {products.map((product) => (
                <div key={`${product.id}-${product.name}`} className={refCss.featureItem}>
                  <span aria-hidden className={productDotClass(product.status)} />
                  {product.name}
                </div>
              ))}
            </div>
          </>
        )}

        {featureAdoption && featureAdoption.length > 0 && (
          <>
            <div className={refCss.featureHeading}>
              Feature adoption · {featureAdoption.length} active
            </div>
            <div className={refCss.featureList}>
              {featureAdoption.map((feature) => (
                <div key={feature} className={refCss.featureItem}>
                  <span aria-hidden className={refCss.featureDot} />
                  {feature}
                </div>
              ))}
            </div>
            {tf?.sourcedAt && (
              <div className={refCss.featureSource}>
                All {featureAdoption.length} features active as of {formatShortDate(tf.sourcedAt)}
                {tf.source ? ` (${tf.source})` : ""}
              </div>
            )}
          </>
        )}

        <div className={refCss.caveat}>
          Full field capture coming in the next release of DailyOS
        </div>
      </div>
    );
  }

  // Inline variant bails out when the account has no footprint at all.
  if (!tf) return null;

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
    <div className={inlineStyles.technicalFootprint}>
      <div className={inlineStyles.technicalFootprintLabel}>Technical Footprint</div>
      <div className={inlineStyles.technicalFootprintGrid}>
        {items.map((item) => (
          <div key={item.label} className={inlineStyles.technicalFootprintItem}>
            <span className={inlineStyles.technicalFootprintItemLabel}>{item.label}</span>
            <span className={inlineStyles.technicalFootprintItemValue}>{item.value}</span>
          </div>
        ))}
      </div>
      <div className={inlineStyles.technicalFootprintSource}>
        Source: {tf.source} &middot; {formatShortDate(tf.sourcedAt)}
      </div>
    </div>
  );
}

import { formatShortDate } from "@/lib/utils";
import type { AccountTechnicalFootprint as TechnicalFootprintData } from "@/types";

import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountTechnicalFootprintProps {
  footprint: TechnicalFootprintData;
}

export function AccountTechnicalFootprint({ footprint }: AccountTechnicalFootprintProps) {
  const tf = footprint;
  const items: { label: string; value: string }[] = [];
  if (tf.supportTier) items.push({ label: "Support", value: tf.supportTier });
  if (tf.csatScore != null) items.push({ label: "CSAT", value: `${tf.csatScore.toFixed(1)}/5` });
  if (tf.openTickets != null && tf.openTickets > 0) items.push({ label: "Open Tickets", value: String(tf.openTickets) });
  if (tf.usageTier) items.push({ label: "Usage Tier", value: tf.usageTier });
  if (tf.activeUsers != null) items.push({ label: "Active Users", value: tf.activeUsers.toLocaleString() });
  if (tf.adoptionScore != null) items.push({ label: "Adoption", value: `${Math.round(tf.adoptionScore * 100)}%` });
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

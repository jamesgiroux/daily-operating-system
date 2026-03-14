import styles from "./TalkBalanceBar.module.css";

interface TalkBalanceBarProps {
  customerPct: number;
  internalPct: number;
}

export function TalkBalanceBar({ customerPct, internalPct }: TalkBalanceBarProps) {
  // Normalize to ensure they sum to 100
  const total = customerPct + internalPct;
  const normalizedCustomer = total > 0 ? Math.round((customerPct / total) * 100) : 50;
  const normalizedInternal = total > 0 ? 100 - normalizedCustomer : 50;

  return (
    <div
      className={styles.container}
      style={{
        "--talk-customer-pct": `${normalizedCustomer}%`,
        "--talk-internal-pct": `${normalizedInternal}%`,
      } as React.CSSProperties}
    >
      <div className={styles.bar}>
        <div className={styles.segmentCustomer} />
        <div className={styles.segmentInternal} />
      </div>
      <div className={styles.labels}>
        <span className={styles.labelCustomer}>Customer {normalizedCustomer}%</span>
        <span className={styles.labelInternal}>Internal {normalizedInternal}%</span>
      </div>
    </div>
  );
}

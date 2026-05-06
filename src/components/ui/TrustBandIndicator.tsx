import clsx from "clsx";
import { AlertTriangle, CircleHelp, ShieldCheck, ShieldQuestion } from "lucide-react";
import type { ComponentPropsWithoutRef } from "react";
import type { TrustBandWire } from "@/lib/trust-band";
import styles from "./TrustBandIndicator.module.css";

interface TrustBandMeta {
  label: string;
  ariaLabel: string;
  Icon: typeof ShieldCheck;
  className: string;
}

const BAND_META: Record<TrustBandWire, TrustBandMeta> = {
  likely_current: {
    label: "Likely current",
    ariaLabel: "Trust band: Likely current. Shown in current evidence.",
    Icon: ShieldCheck,
    className: styles.likelyCurrent,
  },
  use_with_caution: {
    label: "Use with caution",
    ariaLabel: "Trust band: Use with caution. Shown in Background evidence.",
    Icon: AlertTriangle,
    className: styles.useWithCaution,
  },
  needs_verification: {
    label: "Needs verification",
    ariaLabel: "Trust band: Needs verification. Hidden until Show all evidence is enabled.",
    Icon: ShieldQuestion,
    className: styles.needsVerification,
  },
  unscored: {
    label: "Unscored",
    ariaLabel: "Trust band: Unscored. Legacy evidence remains visible.",
    Icon: CircleHelp,
    className: styles.unscored,
  },
};

export interface TrustBandIndicatorProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children"> {
  band: TrustBandWire;
  compact?: boolean;
}

export function TrustBandIndicator({
  band,
  compact = true,
  className,
  ...rest
}: TrustBandIndicatorProps) {
  const meta = BAND_META[band] ?? BAND_META.unscored;
  const Icon = meta.Icon;

  return (
    <span
      className={clsx(styles.indicator, meta.className, className)}
      data-band={band}
      data-compact={compact ? "true" : "false"}
      {...rest}
    >
      <Icon className={styles.icon} role="img" aria-label={meta.ariaLabel} />
      <span className={styles.label}>{meta.label}</span>
    </span>
  );
}

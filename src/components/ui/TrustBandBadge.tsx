import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Pill, type PillTone } from "./Pill";
import styles from "./TrustBandBadge.module.css";

export type TrustBand = "likely_current" | "use_with_caution" | "needs_verification";

const BAND_META: Record<
  TrustBand,
  { label: string; tone: PillTone; className: string }
> = {
  likely_current: {
    label: "Likely current",
    tone: "sage",
    className: styles.likelyCurrent,
  },
  use_with_caution: {
    label: "Use with caution",
    tone: "turmeric",
    className: styles.useWithCaution,
  },
  needs_verification: {
    label: "Needs verification",
    tone: "terracotta",
    className: styles.needsVerification,
  },
};

export interface TrustBandBadgeProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children"> {
  band: TrustBand;
  compact?: boolean;
  children?: ReactNode;
}

export function TrustBandBadge({
  band,
  compact = false,
  children,
  className,
  ...rest
}: TrustBandBadgeProps) {
  const meta = BAND_META[band];

  return (
    <Pill
      tone={meta.tone}
      dot
      size={compact ? "compact" : "standard"}
      className={clsx(styles.badge, compact && styles.compact, meta.className, className)}
      data-band={band}
      data-ds-name="TrustBandBadge"
      data-ds-spec="primitives/TrustBandBadge.md"
      {...rest}
    >
      {children ?? meta.label}
    </Pill>
  );
}

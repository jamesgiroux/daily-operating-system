import clsx from "clsx";
import { Circle } from "lucide-react";
import type { ComponentPropsWithoutRef } from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { TrustBandWire } from "@/lib/trust-band";
import styles from "./TrustBandIndicator.module.css";

interface TrustBandMeta {
  label: string;
  description: string;
  className: string;
}

const BAND_META: Record<
  Exclude<TrustBandWire, "likely_current">,
  TrustBandMeta
> = {
  use_with_caution: {
    label: "Use with caution",
    description:
      "This evidence has caveats — it may be stale, lightly sourced, or carry an unverified timestamp.",
    className: styles.useWithCaution,
  },
  needs_verification: {
    label: "Needs verification",
    description:
      "Confidence is low or a trust gate fired. Confirm against a primary source before acting on it.",
    className: styles.needsVerification,
  },
  unscored: {
    label: "Unscored",
    description: "The trust compiler has not scored this evidence yet.",
    className: styles.unscored,
  },
};

export interface TrustBandIndicatorProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children"> {
  band: TrustBandWire;
}

export function TrustBandIndicator({
  band,
  className,
  ...rest
}: TrustBandIndicatorProps) {
  if (band === "likely_current") {
    return null;
  }

  const meta = BAND_META[band] ?? BAND_META.unscored;
  const ariaLabel = `Trust band: ${meta.label}. ${meta.description}`;

  return (
    <TooltipProvider delayDuration={150}>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            className={clsx(styles.indicator, meta.className, className)}
            data-band={band}
            role="img"
            aria-label={ariaLabel}
            tabIndex={0}
            {...rest}
          >
            <Circle className={styles.icon} aria-hidden="true" />
          </span>
        </TooltipTrigger>
        <TooltipContent side="top" align="center">
          <span className={styles.tooltipLabel}>{meta.label}</span>
          <span className={styles.tooltipDescription}>{meta.description}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

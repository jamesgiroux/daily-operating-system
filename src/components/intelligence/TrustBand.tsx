import clsx from "clsx";
import {
  TrustBandBadge,
  type TrustBand as TrustBandValue,
} from "@/components/ui/TrustBandBadge";
import { FreshnessIndicator } from "@/components/ui/FreshnessIndicator";
import {
  ProvenanceTag,
  type ProvenanceTagSource,
} from "@/components/ui/ProvenanceTag";
import styles from "./TrustBand.module.css";

export type TrustBandDensity = "compact" | "default" | "expanded";
export type TrustBandAlign = "inline" | "row";

export interface TrustBandClaim {
  band?: TrustBandValue;
  trustBand?: TrustBandValue;
  source?: ProvenanceTagSource | null;
  itemSource?: ProvenanceTagSource | null;
  asOf?: string | Date | null;
  sourceAsof?: string | Date | null;
  sourcedAt?: string | Date | null;
  discrepancy?: boolean;
}

export interface TrustBandProps {
  band?: TrustBandValue;
  source?: ProvenanceTagSource | null;
  asOf?: string | Date | null;
  claim?: TrustBandClaim;
  density?: TrustBandDensity;
  align?: TrustBandAlign;
  discrepancy?: boolean;
  className?: string;
}

function resolveBand({
  band,
  claim,
}: Pick<TrustBandProps, "band" | "claim">): TrustBandValue | null {
  return band ?? claim?.band ?? claim?.trustBand ?? null;
}

function resolveSource({
  source,
  claim,
}: Pick<TrustBandProps, "source" | "claim">): ProvenanceTagSource | null | undefined {
  return source ?? claim?.itemSource ?? claim?.source;
}

function resolveAsOf({
  asOf,
  claim,
}: Pick<TrustBandProps, "asOf" | "claim">): string | Date | null | undefined {
  return asOf ?? claim?.asOf ?? claim?.sourceAsof ?? claim?.sourcedAt;
}

export function TrustBand({
  band,
  source,
  asOf,
  claim,
  density = "compact",
  align = "inline",
  discrepancy,
  className,
}: TrustBandProps) {
  const resolvedBand = resolveBand({ band, claim });
  if (!resolvedBand) return null;

  const resolvedSource = resolveSource({ source, claim });
  const resolvedAsOf = resolveAsOf({ asOf, claim });
  const compact = density === "compact";

  return (
    <span
      className={clsx(styles.band, className)}
      data-density={density}
      data-align={align}
      data-ds-name="TrustBand"
      data-ds-spec="patterns/TrustBand.md"
    >
      <TrustBandBadge band={resolvedBand} compact={compact} />
      <ProvenanceTag
        itemSource={resolvedSource}
        discrepancy={discrepancy ?? claim?.discrepancy}
      />
      <FreshnessIndicator
        at={resolvedAsOf}
        format={density === "expanded" ? "both" : "relative"}
      />
    </span>
  );
}

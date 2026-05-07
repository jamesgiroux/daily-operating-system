/**
 * ProvenanceStat — labeled metric primitive.
 *
 * Label + value pair. Optional trend tint on the value (up/down/flat).
 * Composed in stacked groups in the right column of MovingRow.
 *
 * Spec: .docs/design/primitives/ProvenanceStat.md
 * Contract: src/types/briefing.ts → ProvenanceStat (the wire shape carries
 *           TrustMixin via flatten — the component receives label/value/
 *           trend; the trust attribution is read by analytics consumers
 *           from the same wire object, not rendered here).
 */

import clsx from "clsx";
import type { ProvenanceStat as ProvenanceStatViewModel } from "@/types/briefing";
import styles from "./ProvenanceStat.module.css";

interface ProvenanceStatProps {
  stat: ProvenanceStatViewModel;
}

const TREND_CLASS: Record<NonNullable<ProvenanceStatViewModel["trend"]>, string> = {
  up: styles.up,
  down: styles.down,
  flat: styles.flat,
};

export function ProvenanceStat({ stat }: ProvenanceStatProps): JSX.Element {
  const trendClass = stat.trend ? TREND_CLASS[stat.trend] : undefined;
  return (
    <div
      className={styles.root}
      data-ds-name="ProvenanceStat"
      data-ds-tier="primitive"
      data-ds-spec="primitives/ProvenanceStat.md"
    >
      <span className={styles.label}>{stat.label}</span>
      <span className={clsx(styles.value, trendClass)}>{stat.value}</span>
    </div>
  );
}

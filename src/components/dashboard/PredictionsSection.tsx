/**
 * PredictionsSection — Daily Briefing Predictions section.
 *
 * Collapsed by default (single line, count + expand hint). Expands inline
 * to show the list of predictions, each with text + meta + TrustBandBadge.
 *
 * Spec: .docs/design/patterns/PredictionsSection.md
 * Contract: src/types/briefing.ts → PredictionsViewModel + PredictionItem
 */

import { useId, useState } from "react";
import clsx from "clsx";
import type {
  PredictionItem,
  PredictionsViewModel,
  TrustBandWire,
} from "@/types/briefing";
import { TrustBandBadge, type TrustBand } from "@/components/ui/TrustBandBadge";
import styles from "./PredictionsSection.module.css";

interface PredictionsSectionProps {
  predictions: PredictionsViewModel;
}

/** Wire band may be `unscored`; the badge primitive only renders the 3
 * scored bands. Components decide whether to render the badge at all. */
function asBadgeBand(band: TrustBandWire): TrustBand | null {
  return band === "unscored" ? null : band;
}

export function PredictionsSection({
  predictions,
}: PredictionsSectionProps): JSX.Element {
  const [open, setOpen] = useState(false);
  const listId = useId();
  const empty = predictions.count === 0;

  return (
    <section
      className={styles.root}
      data-ds-name="PredictionsSection"
      data-ds-tier="pattern"
      data-ds-spec="patterns/PredictionsSection.md"
      data-open={open}
    >
      <button
        type="button"
        className={styles.trigger}
        aria-expanded={open}
        aria-controls={listId}
        disabled={empty}
        onClick={() => setOpen((o) => !o)}
      >
        <span className={styles.collapsedLabel}>
          {predictions.collapsedLabel}
        </span>
        <span className={styles.expandHint}>
          {empty ? "" : open ? "collapse" : predictions.expandHint}
        </span>
      </button>
      {open && !empty && (
        <ul className={styles.list} id={listId}>
          {predictions.predictions.map((item) => (
            <PredictionRow key={item.id} item={item} />
          ))}
        </ul>
      )}
    </section>
  );
}

function PredictionRow({ item }: { item: PredictionItem }): JSX.Element {
  const badgeBand = asBadgeBand(item.trustBand);
  return (
    <li className={styles.row} data-ds-name="PredictionsSection.row">
      <p className={styles.text}>{item.text}</p>
      <div className={styles.meta}>
        <span className={styles.confidence}>{item.confidence.label}</span>
        <span className={styles.divider}>·</span>
        <span className={styles.abilitySource}>via {item.abilitySource.label}</span>
        <span className={styles.divider}>·</span>
        <a className={styles.basis} href={item.basisLink.href}>
          {item.basisLink.label}
        </a>
        {badgeBand && (
          <TrustBandBadge
            band={badgeBand}
            compact
            className={clsx(styles.trust)}
          />
        )}
      </div>
    </li>
  );
}

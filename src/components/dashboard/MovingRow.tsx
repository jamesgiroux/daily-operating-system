/**
 * MovingRow — Daily Briefing entity movement row.
 *
 * Three-column pattern: identity, narrative + ordered signal feed, and stacked
 * provenance stats. The whole row behaves as a link without wrapping nested
 * interactive controls in an anchor.
 *
 * Spec: .docs/design/patterns/MovingRow.md
 * Contract: src/types/briefing.ts → MovingEntityViewModel
 */

import type { ComponentPropsWithoutRef, KeyboardEvent } from "react";
import clsx from "clsx";
import { Pill } from "@/components/ui/Pill";
import type {
  MovingEntityViewModel,
  MovingSignalViewModel,
  PillView,
} from "@/types/briefing";
import { ProvenanceStat } from "./ProvenanceStat";
import { SignalDot } from "./SignalDot";
import styles from "./MovingRow.module.css";

export interface MovingRowProps
  extends MovingEntityViewModel,
    Omit<
      ComponentPropsWithoutRef<"div">,
      "children" | "onClick" | "onKeyDown" | "role" | "style" | "tabIndex"
    > {
  onNavigate: (href: MovingEntityViewModel["href"], row: MovingEntityViewModel) => void;
  onThreadAction?: (signal: MovingSignalViewModel) => void;
}

function StatePill({ statePill }: { statePill: PillView }): JSX.Element {
  return (
    <Pill
      tone={statePill.tone}
      size="compact"
      className={styles.statePill}
    >
      {statePill.label}
    </Pill>
  );
}

export function MovingRow({
  kind,
  entity,
  href,
  statePill,
  lede,
  signals,
  provenanceStats,
  onNavigate,
  onThreadAction,
  className,
  "aria-label": ariaLabel,
  ...rest
}: MovingRowProps): JSX.Element {
  const row: MovingEntityViewModel = {
    kind,
    entity,
    href,
    statePill,
    lede,
    signals,
    provenanceStats,
  };

  const navigate = () => {
    onNavigate(href, row);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget) return;
    if (event.key !== "Enter") return;

    event.preventDefault();
    navigate();
  };

  return (
    <div
      className={clsx(styles.root, className)}
      role="link"
      tabIndex={0}
      aria-label={ariaLabel ?? `Open ${entity.name}`}
      data-kind={kind}
      data-entity-id={entity.id}
      data-entity-type={entity.entityType}
      data-ds-name="MovingRow"
      data-ds-tier="pattern"
      data-ds-spec="patterns/MovingRow.md"
      onClick={navigate}
      onKeyDown={handleKeyDown}
      {...rest}
    >
      <div className={styles.identity}>
        <div className={styles.nameRow}>
          <span className={styles.name}>{entity.name}</span>
          <StatePill statePill={statePill} />
        </div>
      </div>

      <div className={styles.narrative}>
        <p className={styles.lede}>{lede}</p>
        <ul className={styles.signalFeed} data-ds-name="MovingRow.signalFeed">
          {signals.map((signal, index) => (
            <li
              className={styles.signalItem}
              key={`${signal.kind}-${signal.when}-${index}`}
            >
              <SignalDot signal={signal} onThreadAction={onThreadAction} />
            </li>
          ))}
        </ul>
      </div>

      <div
        className={styles.provenanceStats}
        aria-label={`${entity.name} provenance stats`}
        data-ds-name="MovingRow.provenanceStats"
      >
        {provenanceStats.map((stat, index) => (
          <ProvenanceStat
            stat={stat}
            key={`${stat.label}-${stat.value}-${index}`}
          />
        ))}
      </div>
    </div>
  );
}

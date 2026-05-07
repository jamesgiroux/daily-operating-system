/**
 * WatchRow — Daily Briefing Watch action triage row.
 *
 * Adaptive three-column row whose `kind` discriminator selects the affordance:
 * suggested action selector, open action completion, parked label, or aging
 * restore/archive choices.
 *
 * Spec: .docs/design/patterns/WatchRow.md
 * Contract: src/types/briefing.ts → WatchRowViewModel
 */

import type { MouseEvent } from "react";
import { InferredActionSelector } from "./InferredActionSelector";
import type {
  WatchAgingOption,
  WatchRowViewModel,
} from "@/types/briefing";
import styles from "./WatchRow.module.css";

export interface WatchRowCallbacks {
  onSelectorOption?: (actionId: string, optionId: string) => void;
  onMarkComplete?: (actionId: string) => void;
  onAgingAction?: (
    actionId: string,
    optionId: WatchAgingOption["id"],
  ) => void;
}

export type WatchRowProps = WatchRowViewModel & WatchRowCallbacks;

export function WatchRow(props: WatchRowProps): JSX.Element {
  return (
    <article
      className={styles.root}
      data-kind={props.kind}
      data-ds-name="WatchRow"
      data-ds-tier="pattern"
      data-ds-spec="patterns/WatchRow.md"
    >
      <span className={styles.who} data-ds-name="WatchRow.who">
        {props.who}
      </span>
      <span className={styles.what} data-ds-name="WatchRow.what">
        {props.what}
      </span>
      <span className={styles.affordance} data-ds-name="WatchRow.affordance">
        {renderAffordance(props)}
      </span>
    </article>
  );
}

function renderAffordance(row: WatchRowProps): JSX.Element {
  switch (row.kind) {
    case "suggestedAction":
      return (
        <InferredActionSelector
          selector={row.selector}
          onSelect={(optionId) => row.onSelectorOption?.(row.actionId, optionId)}
          className={styles.selector}
        />
      );

    case "openAction":
      return (
        <button
          type="button"
          className={styles.checkButton}
          aria-label={row.checkButtonLabel}
          onClick={(event) => handleMarkComplete(event, row)}
          data-ds-name="WatchRow.checkButton"
        >
          <span className={styles.checkGlyph} aria-hidden="true" />
        </button>
      );

    case "parked":
      return (
        <span className={styles.parkedLabel} data-ds-name="WatchRow.parkedLabel">
          {row.parkedLabel}
        </span>
      );

    case "aging":
      return (
        <span className={styles.agingAffordance}>
          <time className={styles.ageLabel} dateTime={row.since}>
            {row.ageLabel}
          </time>
          <span className={styles.choiceGroup}>
            {row.options.map((option) => (
              <button
                key={option.id}
                type="button"
                className={styles.choiceButton}
                onClick={(event) => handleAgingAction(event, row, option.id)}
                data-option-id={option.id}
                data-ds-name="WatchRow.agingButton"
              >
                {option.label}
              </button>
            ))}
          </span>
        </span>
      );

    default: {
      const exhaustive: never = row;
      return exhaustive;
    }
  }
}

function handleMarkComplete(
  event: MouseEvent<HTMLButtonElement>,
  row: Extract<WatchRowProps, { kind: "openAction" }>,
) {
  event.stopPropagation();
  row.onMarkComplete?.(row.actionId);
}

function handleAgingAction(
  event: MouseEvent<HTMLButtonElement>,
  row: Extract<WatchRowProps, { kind: "aging" }>,
  optionId: WatchAgingOption["id"],
) {
  event.stopPropagation();
  row.onAgingAction?.(row.actionId, optionId);
}

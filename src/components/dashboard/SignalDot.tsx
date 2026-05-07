/**
 * SignalDot — Daily Briefing Moving signal-feed bullet.
 *
 * Tinted dot + when label + what segments. The dot color marks the signal
 * source kind so a stack of bullets is scannable as a multi-channel feed.
 *
 * Spec: .docs/design/primitives/SignalDot.md
 * Contract: src/types/briefing.ts → MovingSignalViewModel + SignalDotKind
 */

import { Fragment, type MouseEvent } from "react";
import clsx from "clsx";
import type { MovingSignalViewModel, SignalDotKind } from "@/types/briefing";
import styles from "./SignalDot.module.css";

interface SignalDotProps {
  signal: MovingSignalViewModel;
  /**
   * Fires when the thread-action button is clicked. The button stops event
   * propagation internally so the parent row's link does not fire; the parent
   * is responsible for the actual navigation (router link or imperative push)
   * using the href from `signal.threadAction.href`.
   */
  onThreadAction?: (signal: MovingSignalViewModel) => void;
}

const KIND_CLASS: Record<SignalDotKind, string> = {
  meeting: styles.meeting,
  action: styles.action,
  email: styles.email,
  lifecycle: styles.lifecycle,
  "gong-call": styles.gongCall,
  "zendesk-ticket": styles.zendeskTicket,
  "slack-thread": styles.slackThread,
  "linear-issue": styles.linearIssue,
};

export function SignalDot({ signal, onThreadAction }: SignalDotProps): JSX.Element {
  const handleThreadClick = (event: MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    onThreadAction?.(signal);
  };

  return (
    <span
      className={clsx(
        styles.root,
        KIND_CLASS[signal.kind],
        signal.urgency === "overdue" && styles.overdue,
        signal.correctionState === "corrected" && styles.corrected,
        signal.correctionState === "contested" && styles.contested,
      )}
      data-kind={signal.kind}
      data-ds-name="SignalDot"
      data-ds-tier="primitive"
      data-ds-spec="primitives/SignalDot.md"
    >
      <span className={styles.dot} aria-hidden="true" />
      <span className={styles.when}>{signal.when}</span>
      <span className={styles.what}>
        {signal.whatSegments.map((seg, i) =>
          seg.emphasized ? (
            <em key={i}>{seg.text}</em>
          ) : (
            <Fragment key={i}>{seg.text}</Fragment>
          ),
        )}
      </span>
      {signal.threadAction && (
        <button
          type="button"
          className={styles.threadAction}
          onClick={handleThreadClick}
          data-ds-name="SignalDot.threadAction"
        >
          {signal.threadAction.label}
        </button>
      )}
    </span>
  );
}

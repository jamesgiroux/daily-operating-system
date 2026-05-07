/**
 * BriefingErrorState — editorial error frame when briefing data assembly fails.
 *
 * Centered single-column stage. Eyebrow + headline + optional detail + retry
 * and diagnostics affordances + optional code/service meta line. Stack-trace
 * exposure is forbidden — only typed message/detailMessage and code/service
 * meta render.
 *
 * Spec: .docs/design/patterns/BriefingErrorState.md
 */

import styles from "./BriefingErrorState.module.css";

interface BriefingErrorStateProps {
  eyebrow: string;
  message: string;
  detailMessage?: string;
  code?: string;
  service?: string;
  onRetry?: () => void;
  onDiagnostics?: () => void;
}

export function BriefingErrorState({
  eyebrow,
  message,
  detailMessage,
  code,
  service,
  onRetry,
  onDiagnostics,
}: BriefingErrorStateProps): JSX.Element {
  const showMeta = code || service;
  return (
    <section
      className={styles.root}
      role="alert"
      data-ds-name="BriefingErrorState"
      data-ds-tier="pattern"
      data-ds-spec="patterns/BriefingErrorState.md"
    >
      <p className={styles.eyebrow}>{eyebrow}</p>
      <h1 className={styles.headline}>{message}</h1>
      {detailMessage && (
        <p className={styles.detail}>{detailMessage}</p>
      )}
      <div className={styles.actions}>
        {onRetry && (
          <button
            type="button"
            className={styles.retry}
            onClick={onRetry}
            data-ds-name="BriefingErrorState.retry"
          >
            Try again
          </button>
        )}
        {onDiagnostics && (
          <button
            type="button"
            className={styles.diagnostics}
            onClick={onDiagnostics}
            data-ds-name="BriefingErrorState.diagnostics"
          >
            Diagnostics
          </button>
        )}
      </div>
      {showMeta && (
        <p className={styles.meta} data-ds-name="BriefingErrorState.meta">
          {code && <span className={styles.metaSegment}>code: {code}</span>}
          {code && service && <span className={styles.divider}>·</span>}
          {service && (
            <span className={styles.metaSegment}>service: {service}</span>
          )}
        </p>
      )}
    </section>
  );
}

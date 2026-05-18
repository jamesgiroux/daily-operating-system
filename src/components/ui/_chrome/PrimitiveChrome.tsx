/**
 * Primitive chrome — shared empty / loading / error renderers consumed
 * by all Wave 1 primitives per v1.4.3 W2 L0 Packet D §5.8.
 *
 * Surface-side derivation: for v1.4.3 W2, chrome state is derived at
 * render time from `claim_refs` presence + projection state. Producer-
 * side `render_hints.chrome_state` adoption is deferred to v1.4.4 W4
 * per §5.8 producer-vs-surface boundary (cycle-3 challenge #3 finding).
 *
 * MUST NOT be confused with src/components/editorial/EmptyState.tsx —
 * that is page-scoped editorial chrome (h2 + paragraph + buttons), not
 * primitive chrome. Different component, different scope.
 */
import styles from "./PrimitiveChrome.module.css";

export interface PrimitiveChromeProps {
  /** Optional label override. Defaults to canonical chrome vocabulary. */
  label?: string;
}

export function PrimitiveEmpty({ label }: PrimitiveChromeProps) {
  return (
    <span
      className={styles.empty}
      data-chrome="empty"
      data-ds-name="PrimitiveChrome"
      data-ds-spec="primitives/_chrome/README.md"
    >
      {label ?? "—"}
    </span>
  );
}

export function PrimitiveLoading({ label }: PrimitiveChromeProps) {
  return (
    <span
      className={styles.loading}
      data-chrome="loading"
      data-ds-name="PrimitiveChrome"
      data-ds-spec="primitives/_chrome/README.md"
    >
      {label ?? "Loading"}
    </span>
  );
}

export function PrimitiveError({ label }: PrimitiveChromeProps) {
  return (
    <span
      className={styles.error}
      data-chrome="error"
      data-ds-name="PrimitiveChrome"
      data-ds-spec="primitives/_chrome/README.md"
    >
      {label ?? "Error"}
    </span>
  );
}

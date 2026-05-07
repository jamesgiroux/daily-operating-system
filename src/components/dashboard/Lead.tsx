/**
 * Lead.tsx - Daily Briefing editorial lead headline (DOS-426, W1)
 *
 * Single-sentence opener for the briefing. The service owns the wording and
 * split; the component only renders the lead phrase plus optional punch line.
 *
 * Spec: .docs/design/patterns/Lead.md
 * Contract: src/types/briefing.ts -> LeadViewModel
 */

import type { LeadViewModel } from "@/types/briefing";
import styles from "./Lead.module.css";

interface LeadProps {
  lead: LeadViewModel;
}

export function Lead({ lead }: LeadProps): JSX.Element {
  const { headline, focusCapacity, focusBlock } = lead;

  return (
    <section
      className={styles.Lead}
      data-ds-name="Lead"
      data-ds-tier="pattern"
      data-ds-spec="patterns/Lead.md"
    >
      <h1 className={styles.headline}>
        <span>{headline.lead}</span>
        {headline.punchLine ? (
          <>
            {" "}
            <span className={styles.punchLine} data-ds-name="Lead.punchLine">
              {headline.punchLine}
            </span>
          </>
        ) : null}
      </h1>

      <div className={styles.focusMeta}>
        <p className={styles.focusCapacity}>{focusCapacity}</p>
        {focusBlock ? <p className={styles.focusBlock}>{focusBlock}</p> : null}
      </div>
    </section>
  );
}

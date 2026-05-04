/**
 * Provenance tag for intelligence items.
 * Shows a muted label indicating where an intelligence item came from.
 * Omits the tag for pty_synthesis (default, no tag needed).
 */
import clsx from "clsx";
import type { ComponentPropsWithoutRef } from "react";
import type { ItemSource } from "@/types";
import styles from "./ProvenanceTag.module.css";

export type ProvenanceTagSource = ItemSource | string;

export interface ProvenanceTagProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children"> {
  itemSource?: ProvenanceTagSource | null;
  discrepancy?: boolean;
  showSynthesized?: boolean;
}

function titleCaseSource(source: string): string {
  return source
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatSourceName(source: string): string | null {
  switch (source) {
    case "pty_synthesis":
      return null;
    case "glean":
    case "glean_chat":
      return "from Glean";
    case "salesforce":
    case "glean_crm":
      return "from Salesforce";
    case "zendesk":
    case "glean_zendesk":
      return "from Zendesk";
    case "gong":
    case "glean_gong":
      return "from Gong";
    case "email":
      return "from email";
    case "local_file":
      return "from file";
    case "transcript":
      return "from meeting";
    case "user_correction":
      return "you edited this";
    default:
      return titleCaseSource(source);
  }
}

function formatProvenance(source: ProvenanceTagSource, showSynthesized = false): string | null {
  if (typeof source === "string") {
    if (source === "pty_synthesis" && showSynthesized) return "from DailyOS";
    return formatSourceName(source);
  }

  switch (source.source) {
    case "transcript":
      return source.reference ? `from meeting ${source.reference}` : "from meeting";
    case "user_correction":
      return "you edited this";
    case "glean_crm":
      return "from Salesforce";
    case "glean_zendesk":
      return "from Zendesk";
    case "glean_gong":
      return "from Gong";
    case "glean_chat":
      return "from Glean";
    case "email":
      return "from email";
    case "local_file":
      return source.reference ? `from ${source.reference}` : "from file";
    case "pty_synthesis":
      return showSynthesized ? "from DailyOS" : null;
    default:
      return formatSourceName(source.source);
  }
}

export function ProvenanceTag({
  itemSource,
  discrepancy,
  showSynthesized = false,
  className,
  ...rest
}: ProvenanceTagProps) {
  if (!itemSource) return null;
  const label = formatProvenance(itemSource, showSynthesized);
  if (!label) return null;

  return (
    <span
      className={clsx(styles.tag, className)}
      data-ds-name="ProvenanceTag"
      data-ds-spec="primitives/ProvenanceTag.md"
      {...rest}
    >
      {discrepancy && (
        <span
          className={styles.discrepancy}
          title="Multiple sources disagree on this item"
        >
          !{" "}
        </span>
      )}
      {label}
    </span>
  );
}

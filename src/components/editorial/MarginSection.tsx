import type { ReactNode } from "react";

import shared from "@/styles/entity-detail.module.css";

interface MarginSectionProps {
  id?: string;
  label: ReactNode;
  children: ReactNode;
  reveal?: boolean;
}

/**
 * Shared margin-label layout used throughout editorial detail pages.
 * Wraps content with a left margin label and optional editorial reveal class.
 */
export function MarginSection({ id, label, children, reveal = true }: MarginSectionProps) {
  return (
    <div id={id} className={`${reveal ? "editorial-reveal " : ""}${shared.marginLabelSection}`}>
      <div className={shared.marginLabel}>{label}</div>
      <div className={shared.marginContent}>{children}</div>
    </div>
  );
}

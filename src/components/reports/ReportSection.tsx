import type { ReactNode } from "react";

interface ReportSectionProps {
  heading: string;
  children: React.ReactNode;
  className?: string;
  /** I529: Optional inline feedback controls rendered after the heading */
  feedbackSlot?: ReactNode;
}

export function ReportSection({
  heading,
  children,
  className,
  feedbackSlot,
}: ReportSectionProps) {
  return (
    <section
      className={["report-surface-section", className].filter(Boolean).join(" ")}
    >
      <h2 className="report-surface-heading">
        {heading}
        {feedbackSlot}
      </h2>
      {children}
    </section>
  );
}

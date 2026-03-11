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
      className={`report-section${className ? ` ${className}` : ""}`}
      style={{
        marginBottom: "2.5rem",
      }}
    >
      <h2
        style={{
          fontFamily: "var(--font-editorial)",
          fontSize: "1.25rem",
          fontWeight: 400,
          color: "var(--color-desk-charcoal)",
          borderBottom: "2px solid var(--color-spice-turmeric)",
          paddingBottom: "0.4rem",
          marginBottom: "1rem",
          display: "flex",
          alignItems: "center",
          gap: 8,
        }}
      >
        {heading}
        {feedbackSlot}
      </h2>
      {children}
    </section>
  );
}

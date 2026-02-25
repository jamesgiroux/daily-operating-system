interface ReportSectionProps {
  heading: string;
  children: React.ReactNode;
  className?: string;
}

export function ReportSection({
  heading,
  children,
  className,
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
        }}
      >
        {heading}
      </h2>
      {children}
    </section>
  );
}

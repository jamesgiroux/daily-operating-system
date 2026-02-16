import { forwardRef } from "react";

interface TourHighlightProps {
  active: boolean;
  children: React.ReactNode;
}

export const TourHighlight = forwardRef<HTMLDivElement, TourHighlightProps>(
  ({ active, children }, ref) => {
    return (
      <div
        ref={ref}
        style={{
          position: "relative",
          borderLeft: active
            ? "3px solid var(--color-spice-turmeric)"
            : "3px solid transparent",
          paddingLeft: active ? 16 : 16,
          opacity: active ? 1 : 0.4,
          transition: "all 0.3s ease",
        }}
      >
        <div className="pointer-events-none">{children}</div>
      </div>
    );
  },
);
TourHighlight.displayName = "TourHighlight";

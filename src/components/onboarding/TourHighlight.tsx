import { forwardRef } from "react";
import { cn } from "@/lib/utils";

interface TourHighlightProps {
  active: boolean;
  children: React.ReactNode;
}

export const TourHighlight = forwardRef<HTMLDivElement, TourHighlightProps>(
  ({ active, children }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          "relative rounded-xl transition-all duration-300",
          active ? "ring-2 ring-primary opacity-100" : "opacity-40",
        )}
      >
        <div className="pointer-events-none">{children}</div>
      </div>
    );
  },
);
TourHighlight.displayName = "TourHighlight";

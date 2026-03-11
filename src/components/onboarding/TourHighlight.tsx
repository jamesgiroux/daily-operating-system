import { forwardRef } from "react";
import styles from "./onboarding.module.css";

interface TourHighlightProps {
  active: boolean;
  children: React.ReactNode;
}

export const TourHighlight = forwardRef<HTMLDivElement, TourHighlightProps>(
  ({ active, children }, ref) => {
    return (
      <div
        ref={ref}
        className={`${styles.tourHighlight} ${active ? styles.tourHighlightActive : styles.tourHighlightInactive}`}
      >
        <div className="pointer-events-none">{children}</div>
      </div>
    );
  },
);
TourHighlight.displayName = "TourHighlight";

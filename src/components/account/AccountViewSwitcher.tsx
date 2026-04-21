/**
 * AccountViewSwitcher — fixed bottom pill bar for switching between
 * the 3 JTBD views on the account detail page.
 *
 * Props-driven: parent owns activeView state and passes onViewChange.
 * Visual reference: .mockups/account-detail-three-views.html (.mockup-switcher)
 */
import styles from "./AccountViewSwitcher.module.css";

export type AccountView = "health" | "context" | "work";

const VIEWS: { id: AccountView; label: string }[] = [
  { id: "health", label: "Health & Outlook" },
  { id: "context", label: "Context" },
  { id: "work", label: "The Work" },
];

interface Props {
  activeView: AccountView;
  onViewChange: (view: AccountView) => void;
}

export function AccountViewSwitcher({ activeView, onViewChange }: Props) {
  return (
    <nav className={styles.switcher} aria-label="Account views">
      {VIEWS.map((view) => (
        <button
          key={view.id}
          className={`${styles.tab} ${activeView === view.id ? styles.tabActive : ""}`}
          onClick={() => {
            if (activeView !== view.id) onViewChange(view.id);
          }}
          aria-current={activeView === view.id ? "page" : undefined}
        >
          {view.label}
        </button>
      ))}
    </nav>
  );
}

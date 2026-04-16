/**
 * AccountViewSwitcher — fixed bottom pill bar for switching between
 * the 3 JTBD views on the account detail page.
 *
 * DOS-112: "Health & Outlook", "Context", "The Work"
 * ADR-0083 vocabulary compliant.
 *
 * Visual reference: .mockups/account-detail-three-views.html (.mockup-switcher)
 */
import { useNavigate, useParams, useRouterState } from "@tanstack/react-router";
import styles from "./AccountViewSwitcher.module.css";

const VIEWS = [
  { id: "health", label: "Health & Outlook", path: "/health" },
  { id: "context", label: "Context", path: "/context" },
  { id: "work", label: "The Work", path: "/work" },
] as const;

export function AccountViewSwitcher() {
  const navigate = useNavigate();
  const { accountId } = useParams({ strict: false });
  const routerState = useRouterState();
  const deepestPath = routerState.matches[routerState.matches.length - 1]?.routeId ?? "";

  const activeView = deepestPath.includes("/health") ? "health"
    : deepestPath.includes("/context") ? "context"
    : deepestPath.includes("/work") ? "work"
    : "health";

  return (
    <nav className={styles.switcher} aria-label="Account views">
      {VIEWS.map((view) => (
        <button
          key={view.id}
          className={`${styles.tab} ${activeView === view.id ? styles.tabActive : ""}`}
          onClick={() => {
            if (activeView !== view.id) {
              void navigate({
                to: `/accounts/$accountId${view.path}`,
                params: { accountId: accountId! },
              });
            }
          }}
          aria-current={activeView === view.id ? "page" : undefined}
        >
          {view.label}
        </button>
      ))}
    </nav>
  );
}

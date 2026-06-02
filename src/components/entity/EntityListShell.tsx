import { type ReactNode } from "react";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { EditorialPageHeader } from "@/components/editorial/EditorialPageHeader";
import styles from "./EntityListShell.module.css";

// ─── Loading Skeleton ────────────────────────────────────────────────────────

export function EntityListSkeleton() {
  return <EditorialLoading />;
}

// ─── Error State ─────────────────────────────────────────────────────────────

export function EntityListError({ error, onRetry }: { error: string; onRetry: () => void }) {
  return <EditorialError message={error} onRetry={onRetry} />;
}

// ─── Empty State ─────────────────────────────────────────────────────────────

export function EntityListEmpty({
  title,
  message,
  children,
}: {
  title: string;
  message?: string;
  children?: ReactNode;
}) {
  return (
    <div>
      <EditorialEmpty title={title} message={message} />
      {children}
    </div>
  );
}

// ─── Page Header ─────────────────────────────────────────────────────────────

export function EntityListHeader({
  headline,
  count,
  countLabel,
  searchQuery,
  onSearchChange,
  searchPlaceholder,
  children,
}: {
  headline: string;
  count: number;
  countLabel: string;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  searchPlaceholder: string;
  children?: ReactNode;
}) {
  return (
    <EditorialPageHeader
      title={headline}
      meta={`${count} ${countLabel}`}
      scale="standard"
      width="standard"
      rule="subtle"
    >
      <div className={styles.headerContent}>
        {children}
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder={searchPlaceholder}
          className={styles.searchInput}
        />
      </div>
    </EditorialPageHeader>
  );
}

// ─── Archive Toggle ──────────────────────────────────────────────────────────

export function ArchiveToggle({
  archiveTab,
  onTabChange,
}: {
  archiveTab: "active" | "archived";
  onTabChange: (tab: "active" | "archived") => void;
}) {
  const tabs: Array<"active" | "archived"> = ["active", "archived"];
  return (
    <div className={styles.tabs}>
      {tabs.map((tab) => (
        <button
          key={tab}
          onClick={() => onTabChange(tab)}
          className={`${styles.tabButton} ${archiveTab === tab ? styles.tabButtonActive : ""}`}
        >
          {tab}
        </button>
      ))}
    </div>
  );
}

// ─── Filter Tabs ─────────────────────────────────────────────────────────────

export function FilterTabs<T extends string>({
  tabs,
  active,
  onChange,
  labelMap,
}: {
  tabs: readonly T[];
  active: T;
  onChange: (tab: T) => void;
  labelMap?: Partial<Record<T, string>>;
}) {
  return (
    <div className={styles.tabs}>
      {tabs.map((tab) => (
        <button
          key={tab}
          onClick={() => onChange(tab)}
          className={`${styles.tabButton} ${active === tab ? styles.tabButtonActive : ""}`}
        >
          {labelMap?.[tab] ?? tab}
        </button>
      ))}
    </div>
  );
}

// ─── Selection Bar ───────────────────────────────────────────────────────────

export function EntitySelectionBar({
  selectedCount,
  visibleCount,
  onSelectVisible,
  onClear,
  children,
}: {
  selectedCount: number;
  visibleCount: number;
  onSelectVisible: () => void;
  onClear: () => void;
  children?: ReactNode;
}) {
  if (selectedCount <= 0) return null;

  return (
    <div className={styles.selectionBar} role="status" aria-live="polite">
      <span className={styles.selectionCount}>
        {selectedCount} selected
      </span>
      <div className={styles.selectionActions}>
        {children}
        <button
          type="button"
          className={styles.selectionButton}
          onClick={onSelectVisible}
          disabled={visibleCount === 0}
          aria-label={`Select ${visibleCount} visible ${visibleCount === 1 ? "row" : "rows"}`}
        >
          Select visible
        </button>
        <button
          type="button"
          className={styles.selectionButton}
          onClick={onClear}
          aria-label="Clear selected rows"
        >
          Clear
        </button>
      </div>
    </div>
  );
}

 
export function EntityListEndMark(_props?: { text?: string }) {
  return <FinisMarker />;
}

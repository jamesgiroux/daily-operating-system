import { type ReactNode } from "react";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { FinisMarker } from "@/components/editorial/FinisMarker";

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
    <section style={{ paddingTop: 80, paddingBottom: 24 }}>
      <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 36,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          {headline}
        </h1>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
          {count} {countLabel}
        </span>
      </div>

      <div style={{ height: 1, background: "var(--color-rule-heavy)", marginTop: 16, marginBottom: 16 }} />

      {/* Slot for entity-specific filters (archive tabs, status tabs, etc.) */}
      {children}

      <input
        type="text"
        value={searchQuery}
        onChange={(e) => onSearchChange(e.target.value)}
        placeholder={searchPlaceholder}
        style={{
          width: "100%",
          fontFamily: "var(--font-sans)",
          fontSize: 14,
          color: "var(--color-text-primary)",
          background: "none",
          border: "none",
          borderBottom: "1px solid var(--color-rule-light)",
          padding: "8px 0",
          outline: "none",
        }}
      />
    </section>
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
    <div style={{ display: "flex", gap: 20, marginBottom: 12 }}>
      {tabs.map((tab) => (
        <button
          key={tab}
          onClick={() => onTabChange(tab)}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            fontWeight: 500,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: archiveTab === tab ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
            textDecoration: archiveTab === tab ? "underline" : "none",
            textUnderlineOffset: "4px",
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
          }}
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
    <div style={{ display: "flex", gap: 20, marginBottom: 16 }}>
      {tabs.map((tab) => (
        <button
          key={tab}
          onClick={() => onChange(tab)}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            fontWeight: 500,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: active === tab ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
            textDecoration: active === tab ? "underline" : "none",
            textUnderlineOffset: "4px",
            background: "none",
            border: "none",
            padding: 0,
            cursor: "pointer",
          }}
        >
          {labelMap?.[tab] ?? tab}
        </button>
      ))}
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function EntityListEndMark(_props?: { text?: string }) {
  return <FinisMarker />;
}

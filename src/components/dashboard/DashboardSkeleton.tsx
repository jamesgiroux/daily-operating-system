/**
 * DashboardSkeleton — Editorial loading state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 */

import { Skeleton } from "@/components/ui/skeleton";

const skeletonBg = "var(--color-rule-light)";

export function DashboardSkeleton() {
  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* Hero skeleton */}
      <div style={{ paddingTop: 80, paddingBottom: 48 }}>
        <Skeleton className="h-3 w-48 mb-4" style={{ background: skeletonBg }} />
        <Skeleton className="h-12 w-full max-w-lg mb-3" style={{ background: skeletonBg }} />
        <Skeleton className="h-12 w-80 mb-6" style={{ background: skeletonBg }} />
        <div style={{ display: "flex", gap: 8 }}>
          <Skeleton className="h-6 w-28 rounded-full" style={{ background: skeletonBg }} />
          <Skeleton className="h-6 w-36 rounded-full" style={{ background: skeletonBg }} />
        </div>
      </div>

      {/* Focus strip skeleton */}
      <div
        style={{
          borderLeft: "3px solid var(--color-rule-light)",
          borderRadius: 16,
          padding: "20px 24px",
          marginBottom: 48,
          background: "rgba(30, 37, 48, 0.02)",
        }}
      >
        <Skeleton className="h-3 w-24 mb-2" style={{ background: skeletonBg }} />
        <Skeleton className="h-5 w-full max-w-md" style={{ background: skeletonBg }} />
      </div>

      {/* Featured meeting skeleton */}
      <div style={{ marginBottom: 48 }}>
        <Skeleton className="h-6 w-32 mb-5" style={{ background: skeletonBg }} />
        <div
          style={{
            borderRadius: 16,
            borderLeft: "6px solid var(--color-rule-light)",
            padding: "28px 32px",
            background: "rgba(30, 37, 48, 0.02)",
          }}
        >
          <Skeleton className="h-4 w-32 mb-3" style={{ background: skeletonBg }} />
          <Skeleton className="h-7 w-72 mb-2" style={{ background: skeletonBg }} />
          <Skeleton className="h-3 w-48 mb-4" style={{ background: skeletonBg }} />
          <Skeleton className="h-4 w-full max-w-xl mb-2" style={{ background: skeletonBg }} />
          <Skeleton className="h-4 w-full max-w-lg" style={{ background: skeletonBg }} />
        </div>
      </div>

      {/* Schedule skeleton */}
      <div style={{ marginBottom: 48 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 20 }}>
          <Skeleton className="h-7 w-24" style={{ background: skeletonBg }} />
          <Skeleton className="h-3 w-20" style={{ background: skeletonBg }} />
        </div>
        <div style={{ height: 1, background: "var(--color-rule-heavy)", marginBottom: 20 }} />
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {[...Array(3)].map((_, i) => (
            <div
              key={i}
              style={{
                borderRadius: 16,
                borderLeft: "4px solid var(--color-rule-light)",
                padding: "20px 24px",
                background: "rgba(30, 37, 48, 0.02)",
                display: "flex",
                gap: 16,
              }}
            >
              <div style={{ width: 72, flexShrink: 0, textAlign: "right" }}>
                <Skeleton className="h-4 w-16 ml-auto" style={{ background: skeletonBg }} />
                <Skeleton className="h-3 w-10 ml-auto mt-1" style={{ background: skeletonBg }} />
              </div>
              <div style={{ width: 1, background: "var(--color-rule-light)" }} />
              <div style={{ flex: 1 }}>
                <Skeleton className="h-5 w-56 mb-2" style={{ background: skeletonBg }} />
                <Skeleton className="h-3 w-32" style={{ background: skeletonBg }} />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Loose Threads skeleton */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 20 }}>
          <Skeleton className="h-7 w-32" style={{ background: skeletonBg }} />
          <Skeleton className="h-3 w-8" style={{ background: skeletonBg }} />
        </div>
        <div style={{ height: 1, background: "var(--color-rule-heavy)", marginBottom: 16 }} />
        {[...Array(3)].map((_, i) => (
          <div
            key={i}
            style={{
              display: "flex",
              alignItems: "flex-start",
              gap: 12,
              padding: "12px 0",
              borderBottom: i < 2 ? "1px solid var(--color-rule-light)" : "none",
            }}
          >
            <Skeleton className="size-5 rounded-full shrink-0 mt-0.5" style={{ background: skeletonBg }} />
            <div style={{ flex: 1 }}>
              <Skeleton className="h-4 w-full max-w-sm mb-1" style={{ background: skeletonBg }} />
              <Skeleton className="h-3 w-40" style={{ background: skeletonBg }} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

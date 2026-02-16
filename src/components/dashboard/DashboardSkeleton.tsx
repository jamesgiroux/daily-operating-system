/**
 * DashboardSkeleton — Editorial loading state for the daily briefing.
 * Renders inside MagazinePageLayout's page container.
 */

const skeletonBg = "var(--color-rule-light)";

function Pulse({ w, h, mb, mt, ml, round }: { w?: number | string; h: number; mb?: number; mt?: number; ml?: string; round?: boolean }) {
  return (
    <div
      style={{
        width: w ?? "100%",
        height: h,
        marginBottom: mb,
        marginTop: mt,
        marginLeft: ml,
        background: skeletonBg,
        borderRadius: round ? 9999 : 2,
      }}
    />
  );
}

export function DashboardSkeleton() {
  return (
    <div className="editorial-loading" style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* Hero skeleton */}
      <div style={{ paddingTop: 80, paddingBottom: 48 }}>
        <Pulse w={192} h={12} mb={16} />
        <Pulse w="100%" h={48} mb={12} />
        <Pulse w={320} h={48} mb={24} />
        <div style={{ display: "flex", gap: 8 }}>
          <Pulse w={112} h={24} round />
          <Pulse w={144} h={24} round />
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
        <Pulse w={96} h={12} mb={8} />
        <Pulse h={20} />
      </div>

      {/* Featured meeting skeleton */}
      <div style={{ marginBottom: 48 }}>
        <Pulse w={128} h={24} mb={20} />
        <div
          style={{
            borderRadius: 16,
            borderLeft: "6px solid var(--color-rule-light)",
            padding: "28px 32px",
            background: "rgba(30, 37, 48, 0.02)",
          }}
        >
          <Pulse w={128} h={16} mb={12} />
          <Pulse w={288} h={28} mb={8} />
          <Pulse w={192} h={12} mb={16} />
          <Pulse h={16} mb={8} />
          <Pulse h={16} />
        </div>
      </div>

      {/* Schedule skeleton */}
      <div style={{ marginBottom: 48 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 20 }}>
          <Pulse w={96} h={28} />
          <Pulse w={80} h={12} />
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
                <Pulse w={64} h={16} ml="auto" />
                <Pulse w={40} h={12} ml="auto" mt={4} />
              </div>
              <div style={{ width: 1, background: "var(--color-rule-light)" }} />
              <div style={{ flex: 1 }}>
                <Pulse w={224} h={20} mb={8} />
                <Pulse w={128} h={12} />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Loose Threads skeleton */}
      <div>
        <div style={{ display: "flex", alignItems: "baseline", gap: 12, marginBottom: 20 }}>
          <Pulse w={128} h={28} />
          <Pulse w={32} h={12} />
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
            <Pulse w={20} h={20} round />
            <div style={{ flex: 1 }}>
              <Pulse h={16} mb={4} />
              <Pulse w={160} h={12} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

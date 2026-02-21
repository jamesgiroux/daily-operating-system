/**
 * DashboardSkeleton — Editorial loading state for the daily briefing.
 * Matches the current page structure: Day Frame → Schedule → Attention.
 * Follows the margin grid: 100px label column + 32px gap + content column.
 * No rounded corners. Shapes are proportioned to the content they precede.
 */

import s from "@/styles/editorial-briefing.module.css";

const skeletonBg = "var(--color-rule-light)";

function Pulse({
  w,
  h,
  mb,
  mt,
  round,
}: {
  w?: number | string;
  h: number;
  mb?: number;
  mt?: number;
  round?: boolean;
}) {
  return (
    <div
      style={{
        width: w ?? "100%",
        height: h,
        marginBottom: mb,
        marginTop: mt,
        background: skeletonBg,
        borderRadius: round ? 9999 : 2,
      }}
    />
  );
}

export function DashboardSkeleton() {
  return (
    <div className="editorial-loading">

      {/* ── Day Frame skeleton (Hero + Focus merged) ──────────────────── */}
      <div className={s.hero} style={{ paddingTop: 72 }}>
        {/* Large hero headline */}
        <Pulse w="68%" h={64} mb={20} />
        {/* Capacity / focus line */}
        <Pulse w={420} h={18} mb={8} />
        <Pulse w={300} h={18} />
      </div>

      {/* ── Schedule skeleton ─────────────────────────────────────────── */}
      <div className={s.scheduleSection}>
        <div className={s.marginGrid}>
          <div>
            <Pulse w={64} h={10} mb={4} />
            <Pulse w={48} h={10} />
          </div>
          <div className={s.marginContent}>
            <div className={s.sectionRule} />
            {[...Array(4)].map((_, i) => (
              <div
                key={i}
                style={{
                  display: "grid",
                  gridTemplateColumns: "72px 1fr",
                  gap: "0 20px",
                  padding: "14px 0",
                  borderBottom: i < 3 ? `1px solid ${skeletonBg}` : "none",
                }}
              >
                <div style={{ textAlign: "right" }}>
                  <Pulse w={52} h={14} />
                  <Pulse w={28} h={11} mt={4} />
                </div>
                <div>
                  <Pulse w={220} h={18} mb={6} />
                  <Pulse w={120} h={12} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* ── Attention skeleton (actions + emails) ─────────────────────── */}
      <div className={s.prioritiesSection}>
        <div className={s.marginGrid}>
          <div>
            <Pulse w={64} h={10} />
          </div>
          <div className={s.marginContent}>
            <div className={s.sectionRule} />
            {/* Intro line */}
            <Pulse h={15} mb={6} />
            <Pulse w="75%" h={15} mb={28} />
            {/* Action rows */}
            {[...Array(3)].map((_, i) => (
              <div
                key={i}
                style={{
                  display: "flex",
                  alignItems: "flex-start",
                  gap: 16,
                  padding: "12px 0",
                  borderBottom: i < 2 ? `1px solid ${skeletonBg}` : "none",
                }}
              >
                <Pulse w={18} h={18} round />
                <div style={{ flex: 1 }}>
                  <Pulse h={15} mb={5} />
                  <Pulse w={150} h={12} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

    </div>
  );
}

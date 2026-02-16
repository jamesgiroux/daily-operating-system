/**
 * DashboardSkeleton — Editorial loading state for the daily briefing.
 * Matches the margin grid layout: left label placeholders + right content.
 * Renders inside MagazinePageLayout's page container.
 */

import s from "@/styles/editorial-briefing.module.css";

const skeletonBg = "var(--color-rule-light)";

function Pulse({ w, h, mb, mt, round }: { w?: number | string; h: number; mb?: number; mt?: number; round?: boolean }) {
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
      {/* Hero skeleton */}
      <div className={s.hero}>
        <Pulse w="75%" h={72} mb={28} />
        <Pulse w={480} h={20} mb={8} />
        <Pulse w={360} h={20} />
      </div>

      {/* Focus skeleton — margin grid */}
      <div className={s.focusSection}>
        <div className={s.marginGrid}>
          <div>
            <Pulse w={56} h={10} />
          </div>
          <div className={s.marginContent}>
            <div style={{ borderLeft: `3px solid ${skeletonBg}`, paddingLeft: 28, paddingTop: 20, paddingBottom: 20 }}>
              <Pulse h={22} mb={8} />
              <Pulse w={240} h={22} />
            </div>
            <div style={{ paddingLeft: 28, marginTop: 14 }}>
              <Pulse w={200} h={12} />
            </div>
          </div>
        </div>
      </div>

      {/* Lead story skeleton — margin grid */}
      <div className={s.leadStory}>
        <div className={s.marginGrid}>
          <div>
            <Pulse w={72} h={10} />
          </div>
          <div className={s.marginContent}>
            <div className={s.sectionRule} />
            <Pulse w={400} h={28} mb={12} />
            <div style={{ display: "flex", gap: 6, marginBottom: 24 }}>
              <Pulse w={96} h={12} />
              <Pulse w={48} h={12} />
              <Pulse w={112} h={12} />
            </div>
            <Pulse h={18} mb={8} />
            <Pulse h={18} mb={8} />
            <Pulse w="80%" h={18} mb={32} />
            {/* Prep grid placeholder */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "28px 40px" }}>
              <div>
                <Pulse w={64} h={10} mb={12} />
                <Pulse h={14} mb={6} />
                <Pulse h={14} mb={6} />
                <Pulse w="80%" h={14} />
              </div>
              <div>
                <Pulse w={48} h={10} mb={12} />
                <Pulse h={14} mb={6} />
                <Pulse h={14} />
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Schedule skeleton — margin grid */}
      <div className={s.scheduleSection}>
        <div className={s.marginGrid}>
          <div>
            <Pulse w={64} h={10} mb={4} />
            <Pulse w={56} h={10} />
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
                  borderBottom: i < 3 ? "1px solid var(--color-rule-light)" : "none",
                }}
              >
                <div style={{ textAlign: "right" }}>
                  <Pulse w={52} h={14} />
                  <Pulse w={28} h={11} mt={2} />
                </div>
                <div>
                  <Pulse w={240} h={18} mb={4} />
                  <Pulse w={128} h={13} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Priorities skeleton — margin grid */}
      <div className={s.prioritiesSection}>
        <div className={s.marginGrid}>
          <div>
            <Pulse w={64} h={10} />
          </div>
          <div className={s.marginContent}>
            <div className={s.sectionRule} />
            <Pulse h={16} mb={8} />
            <Pulse w="80%" h={16} mb={28} />
            {[...Array(3)].map((_, i) => (
              <div
                key={i}
                style={{
                  display: "flex",
                  alignItems: "flex-start",
                  gap: 16,
                  padding: "12px 0",
                  borderBottom: i < 2 ? "1px solid var(--color-rule-light)" : "none",
                }}
              >
                <Pulse w={18} h={18} round />
                <div style={{ flex: 1 }}>
                  <Pulse h={15} mb={4} />
                  <Pulse w={160} h={13} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

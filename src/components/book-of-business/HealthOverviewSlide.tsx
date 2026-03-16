/**
 * HealthOverviewSlide — Portfolio health tier breakdown with ARR weights.
 * Slide 2: risk tiers, ARR-weighted totals, renewal counts.
 */
import type { BookOfBusinessContent } from "@/types/reports";
import { formatArr } from "@/lib/utils";

interface HealthOverviewSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function HealthOverviewSlide({ content }: HealthOverviewSlideProps) {
  const h = content.healthOverview;
  const totalCount = h.healthyCount + h.mediumCount + h.highRiskCount;
  const totalArr = h.healthyArr + h.mediumArr + h.highRiskArr;

  const tiers = [
    { label: "Healthy", count: h.healthyCount, arr: h.healthyArr, color: "var(--color-garden-sage)", weight: "100%" },
    { label: "Watch", count: h.mediumCount, arr: h.mediumArr, color: "var(--color-spice-saffron)", weight: "75%" },
    { label: "At-Risk", count: h.highRiskCount, arr: h.highRiskArr, color: "var(--color-spice-terracotta)", weight: "40%" },
  ];

  return (
    <section
      id="health-overview"
      style={{
        scrollMarginTop: 60,
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        scrollSnapAlign: "start",
      }}
    >
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: "0.12em",
          color: "var(--color-spice-turmeric)",
          marginBottom: 24,
        }}
      >
        Portfolio Health Overview
      </div>

      {/* Health tier bars */}
      <div style={{ maxWidth: 700, marginBottom: 40 }}>
        {tiers.map((tier) => {
          const pct = totalArr > 0 ? (tier.arr / totalArr) * 100 : 0;
          return (
            <div key={tier.label} style={{ marginBottom: 20 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 6 }}>
                <span style={{ fontFamily: "var(--font-sans)", fontSize: 15, fontWeight: 500, color: tier.color }}>
                  {tier.label}
                </span>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-secondary)" }}>
                  {tier.count} accounts · ${formatArr(tier.arr)} · weight {tier.weight}
                </span>
              </div>
              <div style={{ height: 8, background: "var(--color-rule-light)", borderRadius: 4 }}>
                <div
                  style={{
                    height: "100%",
                    width: `${pct}%`,
                    background: tier.color,
                    borderRadius: 4,
                    transition: "width 0.3s ease",
                  }}
                />
              </div>
            </div>
          );
        })}
      </div>

      {/* Summary metrics */}
      <div style={{ display: "flex", gap: 48, marginBottom: 32 }}>
        <div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 4 }}>
            Total Accounts
          </div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 28, color: "var(--color-text-primary)" }}>
            {totalCount}
          </div>
        </div>
        <div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 4 }}>
            Secure ARR
          </div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 28, color: "var(--color-garden-sage)" }}>
            ${formatArr(h.secureArr)}
          </div>
        </div>
      </div>

      {/* Renewal counts */}
      <div style={{ display: "flex", gap: 48 }}>
        <div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 4 }}>
            Renewals (90d)
          </div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 22, color: "var(--color-text-primary)" }}>
            {h.renewals90d} · ${formatArr(h.renewals90dArr)}
          </div>
        </div>
        <div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 4 }}>
            Renewals (180d)
          </div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 22, color: "var(--color-text-primary)" }}>
            {h.renewals180d} · ${formatArr(h.renewals180dArr)}
          </div>
        </div>
      </div>
    </section>
  );
}

/**
 * VitalsStrip — inline horizontal strip of key account metrics with dot separators.
 * Mockup: all items on one line, mono 12px, border-top AND border-bottom (rule-heavy),
 * padding 14px 0, ARR highlighted in turmeric, health in saffron.
 */
import type { AccountDetail } from "@/types";
import { formatArr, formatShortDate } from "@/lib/utils";

interface VitalsStripProps {
  detail: AccountDetail;
}

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round(
      (renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
    );
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    return `Renewal in ${diffDays}d`;
  } catch {
    return dateStr;
  }
}

interface VitalDisplay {
  text: string;
  highlight?: "turmeric" | "saffron";
}

const healthColorMap: Record<string, "saffron" | undefined> = {
  yellow: "saffron",
};

export function VitalsStrip({ detail }: VitalsStripProps) {
  const vitals: VitalDisplay[] = [];

  if (detail.arr != null) {
    vitals.push({ text: `$${formatArr(detail.arr)} ARR`, highlight: "turmeric" });
  }
  if (detail.health) {
    vitals.push({
      text: `${detail.health.charAt(0).toUpperCase() + detail.health.slice(1)} Health`,
      highlight: healthColorMap[detail.health],
    });
  }
  if (detail.lifecycle) {
    vitals.push({ text: detail.lifecycle });
  }
  if (detail.renewalDate) {
    vitals.push({ text: formatRenewalCountdown(detail.renewalDate) });
  }
  if (detail.nps != null) {
    vitals.push({ text: `NPS ${detail.nps}` });
  }
  if (detail.signals?.meetingFrequency30d != null) {
    vitals.push({ text: `${detail.signals.meetingFrequency30d} meetings / 30d` });
  }
  if (detail.contractStart) {
    vitals.push({ text: `Contract: ${formatShortDate(detail.contractStart)}` });
  }

  if (vitals.length === 0) return null;

  const highlightColor: Record<string, string> = {
    turmeric: "var(--color-spice-turmeric)",
    saffron: "var(--color-spice-saffron)",
  };

  return (
    <div
      style={{
        marginTop: 48,
        borderTop: "1px solid var(--color-rule-heavy)",
        borderBottom: "1px solid var(--color-rule-heavy)",
        padding: "14px 0",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 24,
          flexWrap: "wrap",
        }}
      >
        {vitals.map((v, i) => (
          <span key={i} style={{ display: "flex", alignItems: "center", gap: 24 }}>
            {i > 0 && (
              <span
                style={{
                  width: 3,
                  height: 3,
                  borderRadius: "50%",
                  background: "var(--color-text-tertiary)",
                  flexShrink: 0,
                }}
              />
            )}
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: v.highlight ? highlightColor[v.highlight] : "var(--color-text-secondary)",
                whiteSpace: "nowrap",
              }}
            >
              {v.text}
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}

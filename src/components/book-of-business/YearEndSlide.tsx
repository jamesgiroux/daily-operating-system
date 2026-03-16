/**
 * YearEndSlide — Year-end outlook metrics + landing scenarios.
 * Slides 8+9 combined: editable financial metrics and best/expected/worst scenarios.
 */
import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent, YearEndOutlook, ScenarioRow, LandingScenarios } from "@/types/reports";

interface YearEndSlideProps {
  content: BookOfBusinessContent;
  onUpdate: (content: BookOfBusinessContent) => void;
}

const METRIC_LABELS: { key: keyof YearEndOutlook; label: string }[] = [
  { key: "startingArr", label: "Starting ARR" },
  { key: "atRiskArr", label: "At-Risk ARR" },
  { key: "committedExpansion", label: "Committed Expansion" },
  { key: "expectedChurn", label: "Expected Churn" },
  { key: "projectedEoyArr", label: "Projected EOY ARR" },
];

const SCENARIO_KEYS: { key: keyof LandingScenarios; label: string }[] = [
  { key: "best", label: "Best Case" },
  { key: "expected", label: "Expected" },
  { key: "worst", label: "Worst Case" },
];

const SCENARIO_FIELDS: { key: keyof ScenarioRow; label: string }[] = [
  { key: "keyAssumptions", label: "Key Assumptions" },
  { key: "attrition", label: "Attrition" },
  { key: "expansion", label: "Expansion" },
  { key: "notes", label: "Notes" },
];

export function YearEndSlide({ content, onUpdate }: YearEndSlideProps) {
  const updateMetric = (key: keyof YearEndOutlook, value: string) => {
    const n = parseFloat(value.replace(/[^0-9.]/g, ""));
    if (!isNaN(n)) {
      onUpdate({ ...content, yearEndOutlook: { ...content.yearEndOutlook, [key]: n } });
    }
  };

  const updateScenario = (scenario: keyof LandingScenarios, field: keyof ScenarioRow, value: string) => {
    onUpdate({
      ...content,
      landingScenarios: {
        ...content.landingScenarios,
        [scenario]: { ...content.landingScenarios[scenario], [field]: value },
      },
    });
  };

  return (
    <section
      id="year-end"
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
      {/* — Year-End Outlook — */}
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-spice-turmeric)", marginBottom: 32 }}>
        Year-End Outlook
      </div>

      <div style={{ display: "flex", gap: 48, flexWrap: "wrap", marginBottom: 56, maxWidth: 900 }}>
        {METRIC_LABELS.map(({ key, label }) => (
          <div key={key} style={{ minWidth: 140 }}>
            <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: "var(--color-text-tertiary)", marginBottom: 6 }}>
              {label}
            </div>
            <EditableText
              value={`$${formatArr(content.yearEndOutlook[key])}`}
              onChange={(v) => updateMetric(key, v)}
              multiline={false}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 22,
                fontWeight: 600,
                color: key === "projectedEoyArr" ? "var(--color-spice-turmeric)" : key === "expectedChurn" || key === "atRiskArr" ? "var(--color-spice-terracotta)" : "var(--color-text-primary)",
              }}
            />
          </div>
        ))}
      </div>

      {/* — Landing Scenarios — */}
      <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.12em", color: "var(--color-text-secondary)", marginBottom: 24 }}>
        Landing Scenarios
      </div>

      {/* Column headers */}
      <div style={{ display: "grid", gridTemplateColumns: "100px 1fr 1fr 1fr", gap: 16, paddingBottom: 8, borderBottom: "2px solid var(--color-rule-heavy)", maxWidth: 900 }}>
        <div />
        {SCENARIO_KEYS.map(({ label }) => (
          <div key={label} style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
            {label}
          </div>
        ))}
      </div>

      {/* Scenario rows */}
      {SCENARIO_FIELDS.map(({ key: fieldKey, label: fieldLabel }) => (
        <div key={fieldKey} style={{ display: "grid", gridTemplateColumns: "100px 1fr 1fr 1fr", gap: 16, padding: "12px 0", borderBottom: "1px solid var(--color-rule-light)", maxWidth: 900, alignItems: "baseline" }}>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.08em", color: "var(--color-text-tertiary)" }}>
            {fieldLabel}
          </div>
          {SCENARIO_KEYS.map(({ key: scenarioKey }) => (
            <EditableText
              key={scenarioKey}
              value={content.landingScenarios[scenarioKey][fieldKey]}
              onChange={(v) => updateScenario(scenarioKey, fieldKey, v)}
              multiline={false}
              placeholder={`${fieldLabel}...`}
              style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-primary)" }}
            />
          ))}
        </div>
      ))}
    </section>
  );
}

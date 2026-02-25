import { useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReportRow, ReportType } from "@/types/reports";

interface ReportShellProps {
  report: ReportRow | null;
  entityId: string;
  entityType: string;
  reportType: ReportType;
  title: string;
  onReportGenerated?: (report: ReportRow) => void;
  children: React.ReactNode;
}

const GENERATING_PHASES = [
  { key: "gathering", label: "Gathering intelligence", detail: "Reading entity data, signals, and meeting history" },
  { key: "analyzing", label: "Analyzing context", detail: "Synthesizing insights across connected data" },
  { key: "writing", label: "Writing report", detail: "Producing structured output" },
];

const GENERATING_QUOTES = [
  "Good data is the foundation of good decisions.",
  "The goal is to turn data into information, and information into insight.",
  "In God we trust; all others bring data.",
];

export function ReportShell({
  report,
  entityId,
  entityType,
  reportType,
  title,
  onReportGenerated,
  children,
}: ReportShellProps) {
  const [generating, setGenerating] = useState(false);
  const [genSeconds, setGenSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const shellConfig = useMemo(
    () => ({
      folioLabel: title,
      atmosphereColor: "olive" as const,
      activePage: "accounts" as const,
      backLink: {
        label: "Back",
        onClick: () => window.history.back(),
      },
    }),
    [title],
  );

  useRegisterMagazineShell(shellConfig);

  const handleGenerate = useCallback(async () => {
    setGenerating(true);
    setGenSeconds(0);
    setError(null);

    const timer = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const result = await invoke<ReportRow>("generate_report", {
        entityId,
        entityType,
        reportType,
      });
      onReportGenerated?.(result);
    } catch (err) {
      setError(String(err));
    } finally {
      clearInterval(timer);
      setGenerating(false);
    }
  }, [entityId, entityType, reportType, onReportGenerated]);

  if (generating) {
    const phaseIndex = Math.min(
      Math.floor(genSeconds / 10),
      GENERATING_PHASES.length - 1,
    );
    return (
      <GeneratingProgress
        title={`Generating ${title}`}
        accentColor="var(--color-garden-sage)"
        phases={GENERATING_PHASES}
        currentPhaseKey={GENERATING_PHASES[phaseIndex].key}
        quotes={GENERATING_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  return (
    <div className="report-shell">
      {report?.isStale && (
        <div
          className="report-stale-banner"
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.5rem",
            padding: "0.5rem 1rem",
            background: "var(--color-spice-turmeric)",
            color: "white",
            fontSize: "0.85rem",
            borderBottom: "1px solid var(--color-paper-linen)",
          }}
        >
          <AlertTriangle size={14} />
          <span>Intelligence has updated — this report may be outdated.</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleGenerate}
            style={{ marginLeft: "auto", color: "white" }}
          >
            <RefreshCw size={12} style={{ marginRight: "0.25rem" }} />
            Regenerate
          </Button>
        </div>
      )}

      {error && (
        <div
          style={{
            padding: "1rem",
            background: "#fff5f5",
            color: "var(--color-spice-terracotta)",
            fontSize: "0.875rem",
            borderBottom: "1px solid var(--color-paper-linen)",
          }}
        >
          Error: {error}
        </div>
      )}

      {!report && !generating && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            padding: "4rem",
            gap: "1rem",
            color: "var(--color-desk-charcoal)",
            opacity: 0.7,
          }}
        >
          <p
            style={{
              fontFamily: "var(--font-editorial)",
              fontSize: "1.25rem",
            }}
          >
            No {title} yet.
          </p>
          <Button onClick={handleGenerate}>Generate {title}</Button>
        </div>
      )}

      {report && children}

      {report && (
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "1rem 2rem",
            borderTop: "1px solid var(--color-paper-linen)",
            fontSize: "0.75rem",
            color: "var(--color-desk-charcoal)",
            opacity: 0.6,
          }}
        >
          <span>
            Generated {new Date(report.generatedAt).toLocaleDateString()}
          </span>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <Button variant="ghost" size="sm" onClick={handleGenerate}>
              <RefreshCw size={12} style={{ marginRight: "0.25rem" }} />
              Regenerate
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => window.print()}
            >
              Export PDF
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

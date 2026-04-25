import { useState, useCallback, useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReportRow, ReportType } from "@/types/reports";
import styles from "./report-shell.module.css";

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
  { key: "gathering", label: "Gathering context", detail: "Reading account data, updates, and meeting history" },
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
  const navigate = useNavigate();
  const [generating, setGenerating] = useState(false);
  const [genSeconds, setGenSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const shellConfig = useMemo(
    () => {
      if (entityType === "user") {
        return {
          folioLabel: title,
          atmosphereColor: "olive" as const,
          activePage: "me" as const,
          breadcrumbs: [
            { label: "Me", onClick: () => navigate({ to: "/me" }) },
            { label: title },
          ],
        };
      }

      if (entityType === "project") {
        return {
          folioLabel: title,
          atmosphereColor: "olive" as const,
          activePage: "projects" as const,
          breadcrumbs: [
            { label: "Projects", onClick: () => navigate({ to: "/projects" }) },
            {
              label: "Project",
              onClick: () => navigate({ to: "/projects/$projectId", params: { projectId: entityId } }),
            },
            { label: title },
          ],
        };
      }

      if (entityType === "person") {
        return {
          folioLabel: title,
          atmosphereColor: "olive" as const,
          activePage: "people" as const,
          breadcrumbs: [
            { label: "People", onClick: () => navigate({ to: "/people" }) },
            {
              label: "Person",
              onClick: () => navigate({ to: "/people/$personId", params: { personId: entityId } }),
            },
            { label: title },
          ],
        };
      }

      return {
        folioLabel: title,
        atmosphereColor: "olive" as const,
        activePage: "accounts" as const,
        breadcrumbs: [
          { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
          {
            label: "Account",
            onClick: () => navigate({ to: "/accounts/$accountId", params: { accountId: entityId } }),
          },
          { label: title },
        ],
      };
    },
    [entityId, entityType, navigate, title],
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
        <div className={styles.staleBanner}>
          <AlertTriangle size={14} />
          <span>Context has updated — this report may be outdated.</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleGenerate}
            className={styles.staleBannerButton}
          >
            <RefreshCw size={12} className={styles.refreshIcon} />
            Regenerate
          </Button>
        </div>
      )}

      {error && (
        <div className={styles.errorBanner}>
          Error: {error}
        </div>
      )}

      {!report && !generating && (
        <div className={styles.emptyState}>
          <p className={styles.emptyTitle}>
            No {title} yet.
          </p>
          <Button onClick={handleGenerate}>Generate {title}</Button>
        </div>
      )}

      {report && children}

      {report && (
        <div className={styles.footer}>
          <span>
            Generated {new Date(report.generatedAt).toLocaleDateString()}
          </span>
          <div className={styles.footerActions}>
            <Button variant="ghost" size="sm" onClick={handleGenerate}>
              <RefreshCw size={12} className={styles.refreshIcon} />
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

/**
 * ReportPage — Generic report renderer.
 * Fetches report from DB by entity + type, renders via ReportShell.
 * Handles both entity-scoped routes (/accounts/$accountId/reports/$reportType)
 * and user-scoped routes (/me/reports/$reportType).
 */
import { useState, useEffect, useCallback } from "react";
import { useParams } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { ReportShell } from "@/components/reports/ReportShell";
import { SwotReport } from "@/components/reports/SwotReport";
import { AccountHealthReport } from "@/components/reports/AccountHealthReport";
import { EbrQbrReport } from "@/components/reports/EbrQbrReport";
import { REPORT_TYPE_LABELS } from "@/types/reports";
import type { ReportRow, ReportType, SwotContent, AccountHealthContent, EbrQbrContent } from "@/types/reports";
import type { UserEntity } from "@/types";
import styles from "./report-page.module.css";

export default function ReportPage() {
  const { accountId, projectId, personId, reportType } = useParams({
    strict: false,
  });
  const [report, setReport] = useState<ReportRow | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [userEntityId, setUserEntityId] = useState<string | null>(null);

  // Determine if this is the /me/reports route (no entity params)
  const isUserReport = !accountId && !projectId && !personId;

  // Fetch user entity ID for /me/reports routes
  useEffect(() => {
    if (isUserReport) {
      invoke<UserEntity>("get_user_entity")
        .then((ue) => {
          if (ue) setUserEntityId(String(ue.id));
        })
        .catch(() => {});
    }
  }, [isUserReport]);

  // Determine entity from route params (or user entity for /me routes)
  const entityId = accountId ?? projectId ?? personId ?? userEntityId ?? "";
  const entityType = accountId
    ? "account"
    : projectId
      ? "project"
      : isUserReport
        ? "user"
        : "person";
  const rt = (reportType ?? "swot") as ReportType;
  const title = REPORT_TYPE_LABELS[rt] ?? "Report";

  const fetchReport = useCallback(async () => {
    if (!entityId) return;
    setLoading(true);
    try {
      const result = await invoke<ReportRow | null>("get_report", {
        entityId,
        entityType,
        reportType: rt,
      });
      setReport(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [entityId, entityType, rt]);

  useEffect(() => {
    fetchReport();
  }, [fetchReport]);

  if (loading) {
    return (
      <div className={styles.loadingState}>
        Loading…
      </div>
    );
  }

  if (error) {
    return (
      <div className={styles.errorState}>
        Error: {error}
      </div>
    );
  }

  const renderContent = () => {
    if (!report) return null;
    try {
      const content = JSON.parse(report.contentJson) as unknown;
      switch (rt) {
        case "swot":
          return <SwotReport content={content as SwotContent} />;
        case "account_health":
          return <AccountHealthReport content={content as AccountHealthContent} />;
        case "ebr_qbr":
          return <EbrQbrReport content={content as EbrQbrContent} />;
        default:
          return (
            <pre className={styles.preBlock}>
              {JSON.stringify(content, null, 2)}
            </pre>
          );
      }
    } catch {
      return (
        <div className={styles.parseError}>Error parsing report content.</div>
      );
    }
  };

  return (
    <ReportShell
      report={report}
      entityId={entityId}
      entityType={entityType}
      reportType={rt}
      title={title}
      onReportGenerated={(newReport) => setReport(newReport)}
    >
      {renderContent()}
    </ReportShell>
  );
}

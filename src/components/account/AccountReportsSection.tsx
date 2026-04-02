import { useNavigate } from "@tanstack/react-router";
import { FileText } from "lucide-react";
import { getAccountReports } from "@/lib/report-config";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

import shared from "@/styles/entity-detail.module.css";
import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountReportsSectionProps {
  accountId: string;
  presetId: string | undefined;
}

export function AccountReportsSection({ accountId, presetId }: AccountReportsSectionProps) {
  const navigate = useNavigate();

  return (
    <div id="reports" className={`editorial-reveal ${shared.marginLabelSection}`}>
      <div className={shared.marginLabel}>Reports</div>
      <div className={shared.marginContent}>
        <ChapterHeading title="Reports" />
        <div className={styles.reportsChapter}>
          {getAccountReports(presetId).map((item) => {
            const handleClick = () => {
              if (item.reportType === "risk_briefing") {
                navigate({ to: "/accounts/$accountId/reports/risk_briefing", params: { accountId } } as any);
              } else if (item.reportType === "account_health") {
                navigate({ to: "/accounts/$accountId/reports/account_health", params: { accountId } } as any);
              } else if (item.reportType === "ebr_qbr") {
                navigate({ to: "/accounts/$accountId/reports/ebr_qbr", params: { accountId } } as any);
              } else {
                navigate({ to: "/accounts/$accountId/reports/$reportType", params: { accountId, reportType: item.reportType } });
              }
            };
            return (
              <button
                key={item.label}
                onClick={handleClick}
                className={styles.reportRow}
              >
                <FileText size={16} strokeWidth={1.5} className={styles.reportIcon} />
                <span className={styles.reportName}>{item.label}</span>
                <span className={styles.reportAction}>View</span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

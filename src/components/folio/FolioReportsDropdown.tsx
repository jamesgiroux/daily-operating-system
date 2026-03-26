/**
 * FolioReportsDropdown.tsx
 *
 * Self-contained reports dropdown for the FolioBar actions slot.
 * Owns its own open/close state, reads preset for label customization,
 * and navigates to report pages. No parent state leaks into the volatile ref.
 */

import { useState, useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useActivePreset } from "@/hooks/useActivePreset";
import { getAccountReports } from "@/lib/report-config";
import styles from "./FolioReportsDropdown.module.css";

interface FolioReportsDropdownProps {
  accountId: string;
}

export function FolioReportsDropdown({ accountId }: FolioReportsDropdownProps) {
  const navigate = useNavigate();
  const preset = useActivePreset();
  const [open, setOpen] = useState(false);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick() { setOpen(false); }
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, [open]);

  const reports = getAccountReports(preset?.id);

  return (
    <div className={styles.wrapper}>
      <button
        onClick={(e) => { e.stopPropagation(); setOpen(o => !o); }}
        className={styles.button}
      >
        Reports {open ? "\u25b4" : "\u25be"}
      </button>
      {open && (
        <div className={styles.dropdown}>
          {reports.map((item) => (
            <button
              key={item.label}
              onClick={() => {
                setOpen(false);
                if (item.reportType === "risk_briefing") {
                  navigate({ to: "/accounts/$accountId/reports/risk_briefing", params: { accountId } } as any);
                } else if (item.reportType === "account_health") {
                  navigate({ to: "/accounts/$accountId/reports/account_health", params: { accountId } } as any);
                } else if (item.reportType === "ebr_qbr") {
                  navigate({ to: "/accounts/$accountId/reports/ebr_qbr", params: { accountId } } as any);
                } else {
                  navigate({ to: "/accounts/$accountId/reports/$reportType", params: { accountId, reportType: item.reportType } });
                }
              }}
              className={styles.item}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

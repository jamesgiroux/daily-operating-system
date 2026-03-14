import { formatArr, formatShortDate } from "@/lib/utils";
import type { BookOfBusinessContent } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface SnapshotSlideProps {
  content: BookOfBusinessContent;
  onAddSpotlight?: () => void;
  canAddSpotlight?: boolean;
}

function healthClass(band: string | null | undefined): string {
  switch (band) {
    case "healthy":
      return `${s.healthPill} ${s.healthHealthy}`;
    case "watch":
      return `${s.healthPill} ${s.healthWatch}`;
    case "at-risk":
      return `${s.healthPill} ${s.healthRisk}`;
    default:
      return s.healthPill;
  }
}

function formatHealth(band: string | null | undefined): string {
  if (!band) return "Unknown";
  if (band === "at-risk") return "At Risk";
  return band.charAt(0).toUpperCase() + band.slice(1);
}

export function SnapshotSlide({ content, onAddSpotlight, canAddSpotlight = false }: SnapshotSlideProps) {
  return (
    <section id="snapshot" className={s.slideTight}>
      <div className={s.sectionHeader}>
        <div>
          <div className={s.overline}>Account Snapshot</div>
          <p className={s.subtitle}>
            Portfolio facts are regenerated from the workspace. These rows stay read-only.
          </p>
        </div>
        <div className={s.sectionActions}>
          {onAddSpotlight && (
            <button
              type="button"
              className={`${s.button} ${s.buttonPrimary}`}
              onClick={onAddSpotlight}
              disabled={!canAddSpotlight}
            >
              Add Spotlight
            </button>
          )}
          <div className={s.sourceNote}>Data-sourced</div>
        </div>
      </div>

      {content.accountSnapshot.length === 0 ? (
        <div className={s.emptyBlock}>
          <p className={s.emptyMessage}>No active accounts are available in this portfolio yet.</p>
        </div>
      ) : (
        <div className={s.tableWrap}>
          <table className={s.table}>
            <thead className={s.tableHead}>
              <tr>
                <th>Account</th>
                <th>ARR</th>
                <th>Health</th>
                <th>Trend</th>
                <th>Lifecycle</th>
                <th>Renewal</th>
                <th>Key Contact</th>
                <th>Meetings (90d)</th>
              </tr>
            </thead>
            <tbody>
              {content.accountSnapshot.map((account) => (
                <tr key={account.accountId} className={s.tableRow}>
                  <td className={s.tableName}>{account.accountName}</td>
                  <td>{account.arr != null ? `$${formatArr(account.arr)}` : "—"}</td>
                  <td>
                    <span className={healthClass(account.healthBand)}>{formatHealth(account.healthBand)}</span>
                  </td>
                  <td>{account.healthTrend ?? "—"}</td>
                  <td>{account.lifecycle ?? "—"}</td>
                  <td>{account.renewalDate ? formatShortDate(account.renewalDate) : "—"}</td>
                  <td>{account.keyContact ?? "—"}</td>
                  <td>{account.meetingCount90d}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

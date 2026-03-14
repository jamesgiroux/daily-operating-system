import { formatArr, formatShortDate } from "@/lib/utils";
import type { AccountSnapshotRow, BookOfBusinessContent } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface AppendixSlideProps {
  content: BookOfBusinessContent;
}

function healthTone(band: string | null | undefined): string {
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

function appendixAccounts(content: BookOfBusinessContent): AccountSnapshotRow[] {
  const deepDiveIds = new Set(content.deepDives.map((item) => item.accountId));
  return content.accountSnapshot.filter((account) => !deepDiveIds.has(account.accountId));
}

export function AppendixSlide({ content }: AppendixSlideProps) {
  const appendix = appendixAccounts(content);

  return (
    <section id="appendix" className={s.slideTight}>
      <div className={s.sectionHeader}>
        <div>
          <div className={s.overline}>Appendix</div>
          <p className={s.subtitle}>
            Lightweight account cards for the rest of the portfolio.
          </p>
        </div>
        <div className={s.sourceNote}>Data-sourced</div>
      </div>

      {appendix.length === 0 ? (
        <div className={s.emptyBlock}>
          <p className={s.emptyMessage}>Every active account is already represented in the spotlight section.</p>
        </div>
      ) : (
        <div className={s.appendixGrid}>
          {appendix.map((account) => (
            <article key={account.accountId} className={s.appendixCard}>
              <div className={s.appendixHead}>
                <h3 className={s.appendixTitle}>{account.accountName}</h3>
                <span className={healthTone(account.healthBand)}>
                  {account.healthBand ? account.healthBand.replace("-", " ") : "unknown"}
                </span>
              </div>

              <div className={s.appendixMeta}>
                <div>
                  <span className={s.appendixMetaLabel}>ARR</span>
                  <span className={s.appendixValue}>
                    {account.arr != null ? `$${formatArr(account.arr)}` : "—"}
                  </span>
                </div>
                <div>
                  <span className={s.appendixMetaLabel}>Lifecycle</span>
                  <span className={s.appendixValue}>{account.lifecycle ?? "—"}</span>
                </div>
                <div>
                  <span className={s.appendixMetaLabel}>Renewal</span>
                  <span className={s.appendixValue}>
                    {account.renewalDate ? formatShortDate(account.renewalDate) : "—"}
                  </span>
                </div>
                <div>
                  <span className={s.appendixMetaLabel}>Key Contact</span>
                  <span className={s.appendixValue}>{account.keyContact ?? "—"}</span>
                </div>
                <div>
                  <span className={s.appendixMetaLabel}>Meetings (90d)</span>
                  <span className={s.appendixValue}>{account.meetingCount90d}</span>
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

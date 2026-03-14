import { EditableText } from "@/components/ui/EditableText";
import { formatArr } from "@/lib/utils";
import type { BookOfBusinessContent } from "@/types/reports";
import s from "./BookOfBusinessSlides.module.css";

interface CoverSlideProps {
  content: BookOfBusinessContent;
  reportLabel: string;
  isStale?: boolean;
  onRegenerate?: () => void;
  generating?: boolean;
  onUpdate: (content: BookOfBusinessContent) => void;
}

export function CoverSlide({
  content,
  reportLabel,
  isStale,
  onRegenerate,
  generating,
  onUpdate,
}: CoverSlideProps) {
  return (
    <div className={s.slide}>
      <div className={s.overline}>{content.periodLabel || reportLabel}</div>
      <h1 className={s.title}>{reportLabel}</h1>

      {isStale && (
        <div className={s.staleBanner}>
          <span>Account intelligence changed after this review was generated.</span>
          <button
            type="button"
            onClick={onRegenerate}
            disabled={generating}
            className={`${s.button} ${s.buttonPrimary} ${s.staleAction}`}
          >
            {generating ? "Generating" : "Regenerate"}
          </button>
        </div>
      )}

      <div className={s.vitalsStrip}>
        <VitalStat label="Accounts" value={String(content.totalAccounts)} />
        <VitalStat
          label="Total ARR"
          value={content.totalArr != null ? `$${formatArr(content.totalArr)}` : "—"}
        />
        <VitalStat
          label="At-Risk ARR"
          value={content.atRiskArr != null ? `$${formatArr(content.atRiskArr)}` : "—"}
          danger={(content.atRiskArr ?? 0) > 0}
        />
        <VitalStat
          label="Upcoming Renewals"
          value={`${content.upcomingRenewals}${content.upcomingRenewalsArr != null ? ` ($${formatArr(content.upcomingRenewalsArr)})` : ""}`}
          small
        />
      </div>

      <EditableText
        as="p"
        value={content.executiveSummary}
        onChange={(value) => onUpdate({ ...content, executiveSummary: value })}
        multiline
        placeholder="Summarize the state of the book."
        className={s.heroSummary}
      />
    </div>
  );
}

function VitalStat({
  label,
  value,
  danger,
  small,
}: {
  label: string;
  value: string;
  danger?: boolean;
  small?: boolean;
}) {
  return (
    <div className={s.vitalCard}>
      <span className={s.vitalLabel}>{label}</span>
      <span
        className={[
          s.vitalValue,
          danger ? s.vitalValueDanger : "",
          small ? s.vitalValueSmall : "",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        {value}
      </span>
    </div>
  );
}

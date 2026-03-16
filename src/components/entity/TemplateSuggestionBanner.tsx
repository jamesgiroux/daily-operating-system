import type { SuccessPlanTemplate } from "@/types";
import s from "./TemplateSuggestionBanner.module.css";

interface TemplateSuggestionBannerProps {
  template: SuccessPlanTemplate;
  onView: () => void;
  onDismiss: () => void;
}

export function TemplateSuggestionBanner({
  template,
  onView,
  onDismiss,
}: TemplateSuggestionBannerProps) {
  return (
    <div className={s.banner}>
      <div>
        <div className={s.bannerTitle}>A success plan template is available.</div>
        <div className={s.bannerText}>
          {template.name} matches this account&apos;s current stage.
        </div>
      </div>
      <div className={s.bannerActions}>
        <button className={s.bannerAction} onClick={onView}>
          View
        </button>
        <button className={s.bannerActionMuted} onClick={onDismiss}>
          Dismiss
        </button>
      </div>
    </div>
  );
}

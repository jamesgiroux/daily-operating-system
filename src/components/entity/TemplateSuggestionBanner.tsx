import type { SuccessPlanTemplate } from "@/types";
import { Button } from "@/components/ui/button";
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
        <Button variant="outline" size="sm" className={s.bannerAction} onClick={onView}>
          View
        </Button>
        <Button variant="ghost" size="sm" className={s.bannerActionMuted} onClick={onDismiss}>
          Dismiss
        </Button>
      </div>
    </div>
  );
}

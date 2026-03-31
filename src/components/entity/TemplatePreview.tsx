import type { SuccessPlanTemplate } from "@/types";
import { Button } from "@/components/ui/button";
import s from "./TemplatePreview.module.css";

interface TemplatePreviewProps {
  templates: SuccessPlanTemplate[];
  onApply: (templateId: string) => void;
  onClose: () => void;
}

const EVENT_LABELS: Record<string, string> = {
  kickoff: "Kickoff",
  go_live: "Go-Live",
  onboarding_complete: "Onboarding Complete",
  ebr_completed: "EBR Completed",
  qbr_completed: "QBR Completed",
  renewal: "Renewal",
  contract_signed: "Contract Signed",
  escalation: "Escalation",
  escalation_resolved: "Escalation Resolved",
  champion_change: "Champion Change",
  executive_sponsor_change: "Executive Sponsor Change",
  health_review: "Health Review",
  pilot_start: "Pilot Start",
};

function formatEventType(signal: string): string {
  return EVENT_LABELS[signal] ?? signal.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function formatTargetDate(offsetDays: number): string {
  const target = new Date();
  target.setDate(target.getDate() + offsetDays);
  return target.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function TemplatePreview({
  templates,
  onApply,
  onClose,
}: TemplatePreviewProps) {
  return (
    <div className={s.panel}>
      <div className={s.panelHeader}>
        <div className={s.panelTitle}>Templates</div>
        <Button variant="ghost" size="sm" className={s.closeButton} onClick={onClose}>
          Close
        </Button>
      </div>
      {templates.map((template) => (
        <div key={template.id} className={s.templateCard}>
          <div className={s.templateHead}>
            <div>
              <div className={s.templateName}>{template.name}</div>
              <div className={s.templateDescription}>{template.description}</div>
            </div>
            <Button variant="outline" size="sm" className={s.templateApply} onClick={() => onApply(template.id)}>
              Apply
            </Button>
          </div>
          <ul className={s.objectivesList}>
            {template.objectives.map((objective) => (
              <li key={objective.title} className={s.objectiveItem}>
                <span className={s.objectiveTitle}>{objective.title}</span>
                {objective.description && (
                  <span className={s.objectiveDescription}>{objective.description}</span>
                )}
                {objective.milestones.length > 0 && (
                  <ul className={s.milestonesList}>
                    {objective.milestones.map((milestone) => (
                      <li key={milestone.title} className={s.milestoneItem}>
                        <span className={s.milestoneTitle}>{milestone.title}</span>
                        <span className={s.milestoneMeta}>
                          Target: ~{formatTargetDate(milestone.offsetDays)}
                        </span>
                        {milestone.autoDetectSignal && (
                          <span className={s.autoDetectLabel}>
                            ⚡ {formatEventType(milestone.autoDetectSignal)}
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

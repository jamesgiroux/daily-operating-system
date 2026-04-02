import { Button } from "@/components/ui/button";

import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountRolloverPromptProps {
  renewalDate: string;
  onRenewed: () => void;
  onChurned: () => void;
  onDismiss: () => void;
}

export function AccountRolloverPrompt({
  renewalDate,
  onRenewed,
  onChurned,
  onDismiss,
}: AccountRolloverPromptProps) {
  const isPast = new Date(renewalDate) < new Date();
  if (!isPast) return null;

  return (
    <div className={styles.rolloverPrompt}>
      <span>Renewal date has passed — what happened?</span>
      <div className={styles.rolloverActions}>
        <Button
          variant="outline"
          size="sm"
          onClick={onRenewed}
          className={styles.rolloverButton}
        >
          Renewed
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onChurned}
          className={styles.rolloverButton}
        >
          Churned
        </Button>
        <button
          onClick={onDismiss}
          className={styles.rolloverDismiss}
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}

/**
 * FirstAccountStep.tsx — Wizard step 5: add customer accounts (I57).
 *
 * Optional but encouraged. Seeds the system with accounts
 * so briefings have something to work with on first run.
 * Supports adding multiple accounts before continuing.
 */

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import styles from "../onboarding.module.css";

interface FirstAccountStepProps {
  onNext: () => void;
  onSkip: () => void;
}

export function FirstAccountStep({ onNext, onSkip }: FirstAccountStepProps) {
  const [name, setName] = useState("");
  const [accounts, setAccounts] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  function addAccount() {
    const trimmed = name.trim();
    if (!trimmed) return;
    if (accounts.some((a) => a.toLowerCase() === trimmed.toLowerCase())) return;
    setAccounts((prev) => [...prev, trimmed]);
    setName("");
  }

  function removeAccount(index: number) {
    setAccounts((prev) => prev.filter((_, i) => i !== index));
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      addAccount();
    }
  }

  async function handleContinue() {
    // If user typed something but didn't add it, include it
    const trimmed = name.trim();
    const allAccounts = [...accounts];
    if (trimmed && !allAccounts.some((a) => a.toLowerCase() === trimmed.toLowerCase())) {
      allAccounts.push(trimmed);
    }

    if (allAccounts.length === 0) return;

    setSaving(true);
    let successes = 0;

    for (const accountName of allAccounts) {
      try {
        await invoke("create_account", {
          name: accountName,
          parentId: null,
          accountType: "customer",
        });
        successes++;
      } catch (e) {
        console.error(`Create account failed for "${accountName}":`, e);
      }
    }

    if (successes === 0) {
      // All failed — don't advance
      setSaving(false);
      return;
    }

    try {
      await invoke("set_wizard_step", { step: "first-account" });
    } catch (e) {
      console.error("set_wizard_step failed:", e);
    }

    setSaving(false);
    onNext();
  }

  const canContinue = accounts.length > 0 || name.trim().length > 0;

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Your accounts"
        epigraph="Add your customer accounts to get started. Briefings get smarter with context."
      />

      <div className={styles.ruleSection}>
        <p className={`${styles.bodyText} ${styles.mb16}`}>
          Accounts track customer relationships, health, and context.
          When you meet with someone from an account, your briefing will
          include account history and recent updates.
        </p>

        {/* Chips for added accounts */}
        {accounts.length > 0 && (
          <div className={`${styles.flexWrap} ${styles.mb12}`}>
            {accounts.map((account, index) => (
              <span key={account} className={styles.accountChipSimple}>
                {account}
                <button
                  type="button"
                  className={styles.chipRemoveButton}
                  onClick={() => removeAccount(index)}
                  aria-label={`Remove ${account}`}
                >
                  <X size={12} />
                </button>
              </span>
            ))}
          </div>
        )}

        <label className={styles.fieldLabel}>
          Account name
        </label>
        <div className={`${styles.flexRow} ${styles.gap8}`}>
          <Input
            type="text"
            placeholder="e.g. Acme Corp"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={handleKeyDown}
            className={`${styles.editorialInput} ${styles.flex1}`}
            autoFocus
          />
          <Button
            variant="outline"
            size="sm"
            onClick={addAccount}
            disabled={!name.trim()}
            aria-label="Add account"
          >
            <Plus className="size-4" />
            Add
          </Button>
        </div>
      </div>

      {/* Continue / Skip */}
      <div className={styles.flexBetween}>
        <button
          className={styles.skipButton}
          onClick={() => {
            invoke("set_wizard_step", { step: "first-account" }).catch(() => {});
            onSkip();
          }}
        >
          Skip
        </button>
        <Button onClick={handleContinue} disabled={saving || !canContinue}>
          {saving ? "Creating..." : "Continue"}
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

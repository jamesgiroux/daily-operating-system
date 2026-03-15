/**
 * FirstAccountStep.tsx — Wizard step 5: add customer accounts (I57).
 *
 * Optional but encouraged. Seeds the system with accounts
 * so briefings have something to work with on first run.
 * Supports adding multiple accounts before continuing.
 */

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X, Loader2, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { DiscoveredAccount, OnboardingImportResult } from "@/types";
import styles from "../onboarding.module.css";

interface FirstAccountStepProps {
  onNext: () => void;
  onSkip: () => void;
  gleanConnected?: boolean;
  discoveredAccounts?: DiscoveredAccount[];
  discoveryLoading?: boolean;
}

export function FirstAccountStep({
  onNext,
  onSkip,
  gleanConnected,
  discoveredAccounts,
  discoveryLoading,
}: FirstAccountStepProps) {
  const [name, setName] = useState("");
  const [accounts, setAccounts] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  // Discovery selection state
  const [selectedDiscovered, setSelectedDiscovered] = useState<Set<string>>(new Set());
  const [importResult, setImportResult] = useState<OnboardingImportResult | null>(null);

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

  const importableAccounts = (discoveredAccounts ?? []).filter((a) => !a.alreadyInDailyos);

  function toggleDiscovered(name: string) {
    setSelectedDiscovered((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  function selectAllDiscovered() {
    setSelectedDiscovered(new Set(importableAccounts.map((a) => a.name)));
  }

  async function handleImportDiscovered(names: string[]) {
    if (names.length === 0) return;
    setSaving(true);
    try {
      const result = await invoke<OnboardingImportResult>("onboarding_import_accounts", {
        accountNames: names,
      });
      setImportResult(result);
      await invoke("set_wizard_step", { step: "first-account" }).catch(() => {});
      // Auto-advance after brief delay to show result
      setTimeout(() => onNext(), 1000);
    } catch (e) {
      console.error("Import failed:", e);
    } finally {
      setSaving(false);
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

  // Show discovery branch when Glean connected and accounts found
  const showDiscovery = gleanConnected && (discoveredAccounts ?? []).length > 0;

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Your accounts"
        epigraph={
          showDiscovery
            ? "We found these accounts from your company tools. Select the ones you manage."
            : "Add your customer accounts to get started. Briefings get smarter with context."
        }
      />

      {/* Discovery loading state */}
      {gleanConnected && discoveryLoading && (
        <div className={`${styles.flexRowMd} ${styles.pt8}`}>
          <Loader2 size={18} className={`animate-spin ${styles.tertiaryText}`} />
          <span className={`${styles.bodyText} ${styles.tertiaryText}`}>
            Discovering accounts from your company tools...
          </span>
        </div>
      )}

      {/* Import result feedback */}
      {importResult && (
        <div className={`${styles.flexRowMd} ${styles.pt8}`}>
          <Check size={16} className={styles.sageColor} />
          <span className={styles.bodyText}>
            Added {importResult.created} account{importResult.created !== 1 ? "s" : ""}.
            {importResult.failed.length > 0 && ` ${importResult.failed.length} failed.`}
          </span>
        </div>
      )}

      {/* Discovery results */}
      {showDiscovery && !importResult && (
        <div className={styles.ruleSection}>
          <div className={`${styles.flexBetween} ${styles.mb12}`}>
            <div className={styles.sectionLabel}>
              Discovered accounts ({importableAccounts.length} new)
            </div>
            {importableAccounts.length > 0 && (
              <button
                className={styles.skipButton}
                onClick={selectAllDiscovered}
                type="button"
              >
                Select all
              </button>
            )}
          </div>

          <div className={`${styles.flexCol} ${styles.gap4}`}>
            {(discoveredAccounts ?? []).map((account) => (
              <label key={account.name} className={styles.discoveryRow}>
                {account.alreadyInDailyos ? (
                  <span className={styles.alreadyAddedBadge}>Added</span>
                ) : (
                  <input
                    type="checkbox"
                    checked={selectedDiscovered.has(account.name)}
                    onChange={() => toggleDiscovered(account.name)}
                  />
                )}
                <span className={styles.bodyText}>{account.name}</span>
                {account.myRole && (
                  <span className={styles.tertiaryText}> — {account.myRole}</span>
                )}
                {account.source && (
                  <span className={styles.discoveryEvidence}>{account.source}</span>
                )}
              </label>
            ))}
          </div>

          {/* Import buttons */}
          {importableAccounts.length > 0 && (
            <div className={`${styles.flexEnd} ${styles.gap8} ${styles.mt12}`}>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleImportDiscovered(importableAccounts.map((a) => a.name))}
                disabled={saving}
              >
                Add All ({importableAccounts.length})
              </Button>
              <Button
                size="sm"
                onClick={() => handleImportDiscovered([...selectedDiscovered])}
                disabled={saving || selectedDiscovered.size === 0}
              >
                {saving ? (
                  <Loader2 className="mr-2 size-4 animate-spin" />
                ) : null}
                Add Selected ({selectedDiscovered.size})
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Manual add — always available */}
      <div className={styles.ruleSection}>
        {showDiscovery && (
          <div className={`${styles.sectionLabel} ${styles.mb8}`}>
            Or add manually
          </div>
        )}
        {!showDiscovery && (
          <p className={`${styles.bodyText} ${styles.mb16}`}>
            Accounts track customer relationships, health, and context.
            When you meet with someone from an account, your briefing will
            include account history and recent updates.
          </p>
        )}

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
            autoFocus={!showDiscovery}
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

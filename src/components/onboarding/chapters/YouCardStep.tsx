/**
 * YouCardStep.tsx
 *
 * Simplified wizard step for basic user info.
 * Extracted from AboutYou.tsx — only 4 essential fields:
 * Name, Company, Title, Email Domain(s).
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import styles from "../onboarding.module.css";

export interface YouCardFormData {
  name: string;
  company: string;
  title: string;
  domains: string[];
}

interface YouCardStepProps {
  formData: YouCardFormData;
  onFormChange: (data: YouCardFormData) => void;
  onNext: () => void;
  onSkip: () => void;
  gleanConnected?: boolean;
}

export function YouCardStep({ formData, onFormChange, onNext, onSkip, gleanConnected }: YouCardStepProps) {
  const { email } = useGoogleAuth();
  const [domainInput, setDomainInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [prefilled, setPrefilled] = useState(false);

  const { name, company, title, domains } = formData;

  // Pre-populate name from Google profile if available
  useEffect(() => {
    if (email && !name) {
      // Try to extract name from email prefix as fallback
      // The real name comes from Google profile — handled by set_user_profile
    }
  }, [email, name]);

  // Glean pre-fill: fetch profile from org directory
  useEffect(() => {
    if (!gleanConnected || prefilled) return;
    // Only pre-fill if form is mostly empty
    if (name || company || title) return;

    let cancelled = false;
    invoke<{ name: string | null; title: string | null; department: string | null; company: string | null } | null>(
      "onboarding_prefill_profile"
    )
      .then((suggestion) => {
        if (cancelled || !suggestion) return;
        const updates: Partial<YouCardFormData> = {};
        if (suggestion.name && !name) updates.name = suggestion.name;
        if (suggestion.company && !company) updates.company = suggestion.company;
        if (suggestion.title && !title) updates.title = suggestion.title;
        if (Object.keys(updates).length > 0) {
          onFormChange({ ...formData, ...updates });
          setPrefilled(true);
        }
      })
      .catch(() => {}); // Non-fatal

    return () => { cancelled = true; };
  }, [gleanConnected]); // eslint-disable-line react-hooks/exhaustive-deps

  // Pre-fill first domain from Google email
  useEffect(() => {
    if (email && domains.length === 0) {
      const at = email.indexOf("@");
      if (at !== -1) {
        onFormChange({ ...formData, domains: [email.slice(at + 1)] });
      }
    }
  }, [email]); // eslint-disable-line react-hooks/exhaustive-deps

  function addDomain() {
    const d = domainInput.trim().toLowerCase();
    if (d && !domains.includes(d)) {
      onFormChange({ ...formData, domains: [...domains, d] });
      setDomainInput("");
    }
  }

  function removeDomain(d: string) {
    onFormChange({ ...formData, domains: domains.filter((x) => x !== d) });
  }

  async function handleContinue() {
    setSaving(true);
    try {
      await invoke("set_user_profile", {
        name: name.trim() || null,
        company: company.trim() || null,
        title: title.trim() || null,
        focus: null,
        domains: domains.length > 0 ? domains : null,
      });

      // Create internal organization if company + domains provided
      const trimmedCompany = company.trim();
      if (trimmedCompany && domains.length > 0) {
        try {
          await invoke("create_internal_organization", {
            companyName: trimmedCompany,
            domains,
            teamName: "Core Team",
            colleagues: [],
          });
        } catch {
          // Non-fatal
        }
      }

      // Persist wizard step
      await invoke("set_wizard_step", { step: "youcard" });
    } catch (e) {
      console.error("set_user_profile failed:", e); // Expected: best-effort profile save during onboarding
    } finally {
      setSaving(false);
    }
    onNext();
  }

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="About you"
        epigraph="A little context helps tailor your briefings. Everything here is optional."
      />

      {prefilled && (
        <p className={styles.helperText}>
          Pre-filled from your company directory. Edit anything that needs updating.
        </p>
      )}

      <div className={`${styles.flexCol} ${styles.gap20}`}>
        {/* Name */}
        <div>
          <label className={styles.fieldLabel}>Your name</label>
          <Input
            type="text"
            placeholder="e.g. Alex Chen"
            value={name}
            onChange={(e) => onFormChange({ ...formData, name: e.target.value })}
            className={styles.editorialInput}
          />
        </div>

        {/* Company */}
        <div>
          <label className={styles.fieldLabel}>Company</label>
          <Input
            type="text"
            placeholder="e.g. Acme Inc."
            value={company}
            onChange={(e) => onFormChange({ ...formData, company: e.target.value })}
            className={styles.editorialInput}
          />
        </div>

        {/* Title */}
        <div>
          <label className={styles.fieldLabel}>Title</label>
          <Input
            type="text"
            placeholder="e.g. Customer Success Manager"
            value={title}
            onChange={(e) => onFormChange({ ...formData, title: e.target.value })}
            className={styles.editorialInput}
          />
        </div>

        {/* Domains */}
        <div>
          <label className={styles.fieldLabel}>Company email domains</label>
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder="e.g. acme.com"
              value={domainInput}
              onChange={(e) => setDomainInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addDomain()}
              className={styles.editorialInput}
            />
            <Button
              variant="outline"
              size="icon"
              onClick={addDomain}
              disabled={!domainInput.trim()}
            >
              <Plus className="size-4" />
            </Button>
          </div>
          {domains.length > 0 && (
            <div className={`${styles.flexWrap} ${styles.mt8}`}>
              {domains.map((d) => (
                <span key={d} className={styles.domainChip}>
                  @{d}
                  <button
                    onClick={() => removeDomain(d)}
                    className={styles.ghostButton}
                  >
                    <X size={12} />
                  </button>
                </span>
              ))}
            </div>
          )}
          <p className={styles.helperText}>
            Helps distinguish internal meetings from external ones.
          </p>
        </div>
      </div>

      {/* Continue / Skip */}
      <div className={styles.flexBetween}>
        <button
          className={styles.skipButton}
          onClick={() => {
            invoke("set_wizard_step", { step: "youcard" }).catch(() => {});
            onSkip();
          }}
        >
          Skip
        </button>
        <Button onClick={handleContinue} disabled={saving}>
          {saving ? "Saving..." : "Continue"}
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

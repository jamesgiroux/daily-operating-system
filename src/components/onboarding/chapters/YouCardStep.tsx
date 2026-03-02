/**
 * YouCardStep.tsx
 *
 * Simplified wizard step for basic user info (I57).
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
}

function FieldLabel({ children }: { children: React.ReactNode }) {
  return (
    <label
      style={{
        fontFamily: "var(--font-sans)",
        fontSize: 13,
        fontWeight: 500,
        color: "var(--color-text-secondary)",
        display: "block",
        marginBottom: 6,
      }}
    >
      {children}
    </label>
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--color-paper-warm-white)",
  border: "1px solid var(--color-desk-charcoal)",
  borderRadius: 4,
};

export function YouCardStep({ formData, onFormChange, onNext, onSkip }: YouCardStepProps) {
  const { email } = useGoogleAuth();
  const [domainInput, setDomainInput] = useState("");
  const [saving, setSaving] = useState(false);

  const { name, company, title, domains } = formData;

  // Pre-populate name from Google profile if available
  useEffect(() => {
    if (email && !name) {
      // Try to extract name from email prefix as fallback
      // The real name comes from Google profile — handled by set_user_profile
    }
  }, [email, name]);

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
      console.error("set_user_profile failed:", e);
    } finally {
      setSaving(false);
    }
    onNext();
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="About you"
        epigraph="A little context helps tailor your briefings. Everything here is optional."
      />

      <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
        {/* Name */}
        <div>
          <FieldLabel>Your name</FieldLabel>
          <Input
            type="text"
            placeholder="e.g. Alex Chen"
            value={name}
            onChange={(e) => onFormChange({ ...formData, name: e.target.value })}
            style={inputStyle}
          />
        </div>

        {/* Company */}
        <div>
          <FieldLabel>Company</FieldLabel>
          <Input
            type="text"
            placeholder="e.g. Acme Inc."
            value={company}
            onChange={(e) => onFormChange({ ...formData, company: e.target.value })}
            style={inputStyle}
          />
        </div>

        {/* Title */}
        <div>
          <FieldLabel>Title</FieldLabel>
          <Input
            type="text"
            placeholder="e.g. Customer Success Manager"
            value={title}
            onChange={(e) => onFormChange({ ...formData, title: e.target.value })}
            style={inputStyle}
          />
        </div>

        {/* Domains */}
        <div>
          <FieldLabel>Company email domains</FieldLabel>
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder="e.g. acme.com"
              value={domainInput}
              onChange={(e) => setDomainInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addDomain()}
              style={inputStyle}
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
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 8 }}>
              {domains.map((d) => (
                <span
                  key={d}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                    border: "1px solid var(--color-rule-heavy)",
                    borderRadius: 4,
                    padding: "4px 10px",
                    color: "var(--color-text-primary)",
                  }}
                >
                  @{d}
                  <button
                    onClick={() => removeDomain(d)}
                    style={{
                      color: "var(--color-text-tertiary)",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: 0,
                      lineHeight: 1,
                    }}
                  >
                    <X size={12} />
                  </button>
                </span>
              ))}
            </div>
          )}
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              margin: "4px 0 0",
            }}
          >
            Helps distinguish internal meetings from external ones.
          </p>
        </div>
      </div>

      {/* Continue / Skip */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <button
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            letterSpacing: "0.04em",
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
          }}
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

import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X, ChevronDown, ChevronUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import styles from "../onboarding.module.css";

interface ColleagueRow {
  _key: number;
  name: string;
  email: string;
}

export interface AboutYouFormData {
  name: string;
  company: string;
  title: string;
  domains: string[];
  focus: string;
  colleagues: ColleagueRow[];
}

interface AboutYouProps {
  formData: AboutYouFormData;
  onFormChange: (data: AboutYouFormData) => void;
  onNext: () => void;
}

export function AboutYou({ formData, onFormChange, onNext }: AboutYouProps) {
  const { email } = useGoogleAuth();

  // Transient UI state stays local
  const [domainInput, setDomainInput] = useState("");
  const [showTeammates, setShowTeammates] = useState(false);
  const [saving, setSaving] = useState(false);
  const nextKey = useRef(0);
  const [suggestions, setSuggestions] = useState<Array<{ name: string; email: string; messageCount: number }>>([]);

  // Destructure lifted state for convenient access
  const { name, company, title, domains, focus, colleagues } = formData;

  // Fetch frequent correspondents from Gmail
  useEffect(() => {
    if (!email) return;
    invoke<Array<{ name: string; email: string; messageCount: number }>>(
      "get_frequent_correspondents",
      { userEmail: email }
    )
      .then((result) => setSuggestions(result))
      .catch((err) => console.error("get_frequent_correspondents failed:", err));
  }, [email]);

  const filteredSuggestions = suggestions.filter(
    (s) => !formData.colleagues.some((c) => c.email.toLowerCase() === s.email.toLowerCase())
  );

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

  function addColleague() {
    const key = nextKey.current++;
    onFormChange({
      ...formData,
      colleagues: [...colleagues, { _key: key, name: "", email: "" }],
    });
    if (!showTeammates) setShowTeammates(true);
  }

  function updateColleague(index: number, next: Partial<ColleagueRow>) {
    onFormChange({
      ...formData,
      colleagues: colleagues.map((row, i) => (i === index ? { ...row, ...next } : row)),
    });
  }

  function removeColleague(index: number) {
    onFormChange({
      ...formData,
      colleagues: colleagues.filter((_, i) => i !== index),
    });
  }

  async function handleContinue() {
    setSaving(true);
    try {
      // Save user profile
      await invoke("set_user_profile", {
        name: name.trim() || null,
        company: company.trim() || null,
        title: title.trim() || null,
        focus: focus.trim() || null,
        domains: domains.length > 0 ? domains : null,
      });

      // Create internal organization if company + domains provided
      const trimmedCompany = company.trim();
      if (trimmedCompany && domains.length > 0) {
        const cleanedColleagues = colleagues
          .map((c) => ({ name: c.name.trim(), email: c.email.trim() }))
          .filter((c) => c.name && c.email);

        try {
          await invoke("create_internal_organization", {
            companyName: trimmedCompany,
            domains,
            teamName: "Core Team",
            colleagues: cleanedColleagues,
          });
        } catch {
          // Non-fatal — org may already exist or setup may be incomplete
        }
      }
    } catch (e) {
      console.error("set_user_profile failed:", e);
    } finally {
      setSaving(false);
    }
    onNext();
  }

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="About you"
        epigraph="A little context helps DailyOS tailor your briefings. Everything here is optional."
      />

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
          <label className={styles.fieldLabel}>Company domains</label>
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
                  {d}
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
            Add every domain your company uses for email (e.g. acme.com and acme.io).
            Helps DailyOS tell internal meetings from external ones.
          </p>
        </div>

        {/* Focus / priorities */}
        <div>
          <label className={styles.fieldLabel}>Current priorities</label>
          <textarea
            className={styles.editorialTextarea}
            placeholder="e.g. Driving renewals for Q2, onboarding three new accounts"
            value={focus}
            onChange={(e) => onFormChange({ ...formData, focus: e.target.value })}
          />
          <p className={styles.helperText}>Share what you're focused on. This helps AI tailor your briefings.</p>
        </div>

        {/* Teammates — collapsible section */}
        <div className={styles.ruleSection}>
          <button
            onClick={() => {
              setShowTeammates(!showTeammates);
              if (!showTeammates && colleagues.length === 0) addColleague();
            }}
            className={styles.toggleButton}
          >
            <label className={styles.fieldLabel}>Closest teammates</label>
            {showTeammates ? (
              <ChevronUp size={14} className={styles.tertiaryText} />
            ) : (
              <ChevronDown size={14} className={styles.tertiaryText} />
            )}
          </button>
          <p className={styles.helperText}>
            People you work with daily. DailyOS uses this to distinguish internal from external attendees.
          </p>

          {showTeammates && (
            <div className={`${styles.flexCol} ${styles.gap12} ${styles.mt12}`}>
              {filteredSuggestions.length > 0 && (
                <div className={styles.mb12}>
                  <p className={styles.sectionLabel}>
                    Suggested from Gmail
                  </p>
                  <div className={styles.flexWrap}>
                    {filteredSuggestions.map((s) => (
                      <button
                        key={s.email}
                        onClick={() => {
                          const key = nextKey.current++;
                          onFormChange({
                            ...formData,
                            colleagues: [...formData.colleagues, { _key: key, name: s.name, email: s.email }],
                          });
                        }}
                        className={styles.suggestionChip}
                      >
                        <Plus size={12} />
                        {s.name || s.email}
                      </button>
                    ))}
                  </div>
                </div>
              )}
              {colleagues.map((row, idx) => (
                <div
                  key={row._key}
                  className={`${styles.flexRow} ${styles.gap8}`}
                >
                  <Input
                    placeholder="Name"
                    value={row.name}
                    onChange={(e) => updateColleague(idx, { name: e.target.value })}
                    className={styles.editorialInput}
                  />
                  <Input
                    placeholder="Email"
                    value={row.email}
                    onChange={(e) => updateColleague(idx, { email: e.target.value })}
                    className={styles.editorialInput}
                  />
                  <button
                    onClick={() => removeColleague(idx)}
                    className={styles.removeButton}
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
              <Button
                variant="outline"
                size="sm"
                onClick={addColleague}
                className={styles.selfStart}
              >
                <Plus className="mr-1 size-3" />
                Add another
              </Button>
            </div>
          )}
        </div>
      </div>

      <div className="flex justify-end">
        <Button onClick={handleContinue} disabled={saving}>
          {saving ? "Saving..." : "Continue"}
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

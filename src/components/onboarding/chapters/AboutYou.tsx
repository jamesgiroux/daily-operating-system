import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X, ChevronDown, ChevronUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";

interface AboutYouProps {
  onNext: () => void;
}

interface ColleagueRow {
  _key: number;
  name: string;
  email: string;
}

/** Editorial form label */
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

/** Editorial helper text */
function HelperText({ children }: { children: React.ReactNode }) {
  return (
    <p
      style={{
        fontFamily: "var(--font-sans)",
        fontSize: 12,
        color: "var(--color-text-tertiary)",
        margin: "4px 0 0",
      }}
    >
      {children}
    </p>
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--color-paper-warm-white)",
  border: "1px solid var(--color-desk-charcoal)",
  borderRadius: 4,
};

export function AboutYou({ onNext }: AboutYouProps) {
  const { email } = useGoogleAuth();

  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [title, setTitle] = useState("");
  const [domains, setDomains] = useState<string[]>([]);
  const [domainInput, setDomainInput] = useState("");
  const [focus, setFocus] = useState("");
  const [colleagues, setColleagues] = useState<ColleagueRow[]>([]);
  const [showTeammates, setShowTeammates] = useState(false);
  const [saving, setSaving] = useState(false);
  const nextKey = useRef(0);

  // Pre-fill first domain from Google email
  useEffect(() => {
    if (email && domains.length === 0) {
      const at = email.indexOf("@");
      if (at !== -1) {
        setDomains([email.slice(at + 1)]);
      }
    }
  }, [email]); // eslint-disable-line react-hooks/exhaustive-deps

  function addDomain() {
    const d = domainInput.trim().toLowerCase();
    if (d && !domains.includes(d)) {
      setDomains([...domains, d]);
      setDomainInput("");
    }
  }

  function removeDomain(d: string) {
    setDomains(domains.filter((x) => x !== d));
  }

  function addColleague() {
    setColleagues((prev) => [...prev, { _key: nextKey.current++, name: "", email: "" }]);
    if (!showTeammates) setShowTeammates(true);
  }

  function updateColleague(index: number, next: Partial<ColleagueRow>) {
    setColleagues((prev) => prev.map((row, i) => (i === index ? { ...row, ...next } : row)));
  }

  function removeColleague(index: number) {
    setColleagues((prev) => prev.filter((_, i) => i !== index));
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
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="About you"
        epigraph="A little context helps DailyOS tailor your briefings. Everything here is optional."
      />

      <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
        {/* Name */}
        <div>
          <FieldLabel>Your name</FieldLabel>
          <Input
            type="text"
            placeholder="e.g. Alex Chen"
            value={name}
            onChange={(e) => setName(e.target.value)}
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
            onChange={(e) => setCompany(e.target.value)}
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
            onChange={(e) => setTitle(e.target.value)}
            style={inputStyle}
          />
        </div>

        {/* Domains */}
        <div>
          <FieldLabel>Company domains</FieldLabel>
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
                  {d}
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
          <HelperText>
            Add every domain your company uses for email (e.g. acme.com and acme.io).
            Helps DailyOS tell internal meetings from external ones.
          </HelperText>
        </div>

        {/* Focus / priorities */}
        <div>
          <FieldLabel>Current priorities</FieldLabel>
          <textarea
            style={{
              width: "100%",
              minHeight: 80,
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              background: "var(--color-paper-warm-white)",
              border: "1px solid var(--color-desk-charcoal)",
              borderRadius: 4,
              padding: "8px 12px",
              color: "var(--color-text-primary)",
              resize: "vertical",
              outline: "none",
            }}
            placeholder="e.g. Driving renewals for Q2, onboarding three new accounts"
            value={focus}
            onChange={(e) => setFocus(e.target.value)}
          />
          <HelperText>Share what you're focused on. This helps AI tailor your briefings.</HelperText>
        </div>

        {/* Teammates — collapsible section */}
        <div
          style={{
            borderTop: "1px solid var(--color-rule-light)",
            paddingTop: 20,
          }}
        >
          <button
            onClick={() => {
              setShowTeammates(!showTeammates);
              if (!showTeammates && colleagues.length === 0) addColleague();
            }}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              width: "100%",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
            }}
          >
            <FieldLabel>Closest teammates</FieldLabel>
            {showTeammates ? (
              <ChevronUp size={14} style={{ color: "var(--color-text-tertiary)" }} />
            ) : (
              <ChevronDown size={14} style={{ color: "var(--color-text-tertiary)" }} />
            )}
          </button>
          <HelperText>
            People you work with daily. DailyOS uses this to distinguish internal from external attendees.
          </HelperText>

          {showTeammates && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12, marginTop: 12 }}>
              {colleagues.map((row, idx) => (
                <div
                  key={row._key}
                  style={{
                    display: "flex",
                    gap: 8,
                    alignItems: "center",
                  }}
                >
                  <Input
                    placeholder="Name"
                    value={row.name}
                    onChange={(e) => updateColleague(idx, { name: e.target.value })}
                    style={inputStyle}
                  />
                  <Input
                    placeholder="Email"
                    value={row.email}
                    onChange={(e) => updateColleague(idx, { email: e.target.value })}
                    style={inputStyle}
                  />
                  <button
                    onClick={() => removeColleague(idx)}
                    style={{
                      color: "var(--color-text-tertiary)",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: 4,
                      flexShrink: 0,
                    }}
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
              <Button
                variant="outline"
                size="sm"
                onClick={addColleague}
                style={{ alignSelf: "flex-start" }}
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

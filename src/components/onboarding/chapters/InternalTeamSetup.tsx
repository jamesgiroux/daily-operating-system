import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Building2, Plus, Users, UserPlus, Link } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { PersonListItem } from "@/types";

interface ColleagueInput {
  _key: number;
  name: string;
  email: string;
  title?: string;
}

interface LinkedPerson {
  id: string;
  name: string;
  email: string;
  role?: string;
}

interface SetupStatus {
  required: boolean;
  prefill: {
    company?: string;
    domains: string[];
    title?: string;
    suggestedTeamName: string;
    suggestedColleagues: ColleagueInput[];
  };
}

interface InternalTeamSetupProps {
  onNext: () => void;
}

/** Mono uppercase step indicator */
function StepLabel({ step, total }: { step: number; total: number }) {
  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 10,
        fontWeight: 500,
        textTransform: "uppercase" as const,
        letterSpacing: "0.1em",
        color: "var(--color-text-tertiary)",
      }}
    >
      Step {step} of {total}
    </div>
  );
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

const inputStyle: React.CSSProperties = {
  background: "var(--color-paper-warm-white)",
  border: "1px solid var(--color-desk-charcoal)",
  borderRadius: 4,
};

export function InternalTeamSetup({ onNext }: InternalTeamSetupProps) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [step, setStep] = useState(0);

  const [company, setCompany] = useState("");
  const [domainInput, setDomainInput] = useState("");
  const [domains, setDomains] = useState<string[]>([]);
  const [title, setTitle] = useState("");
  const [teamName, setTeamName] = useState("");
  const [colleagues, setColleagues] = useState<ColleagueInput[]>([]);
  const [linkedPeople, setLinkedPeople] = useState<LinkedPerson[]>([]);
  const [existingPeople, setExistingPeople] = useState<PersonListItem[]>([]);
  const [peopleSearch, setPeopleSearch] = useState("");
  const nextKey = useRef(0);

  useEffect(() => {
    async function load() {
      try {
        const [status, people] = await Promise.all([
          invoke<SetupStatus>("get_internal_team_setup_status"),
          invoke<PersonListItem[]>("get_people", { relationship: null }).catch((err) => {
            console.error("get_people failed:", err);
            return [];
          }),
        ]);
        if (!status.required) {
          onNext();
          return;
        }
        setCompany(status.prefill.company ?? "");
        setDomains(status.prefill.domains ?? []);
        setTitle(status.prefill.title ?? "");
        setTeamName(status.prefill.suggestedTeamName || "Core Team");
        const keyed = (status.prefill.suggestedColleagues ?? []).map((c) => ({
          ...c,
          _key: nextKey.current++,
        }));
        setColleagues(keyed);
        setExistingPeople(people);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [onNext]);

  const canAdvance = useMemo(() => {
    if (step === 0) return company.trim().length > 0 && domains.length > 0;
    if (step === 1) return teamName.trim().length > 0;
    return true;
  }, [company, domains.length, step, teamName]);

  function addDomain() {
    const normalized = domainInput.trim().toLowerCase();
    if (!normalized || domains.includes(normalized)) return;
    setDomains((prev) => [...prev, normalized]);
    setDomainInput("");
  }

  function updateColleague(index: number, next: Partial<ColleagueInput>) {
    setColleagues((prev) => prev.map((row, i) => (i === index ? { ...row, ...next } : row)));
  }

  function removeColleague(index: number) {
    setColleagues((prev) => prev.filter((_, i) => i !== index));
  }

  function addColleague() {
    setColleagues((prev) => [...prev, { _key: nextKey.current++, name: "", email: "", title: "" }]);
  }

  function linkExistingPerson(person: PersonListItem) {
    if (linkedPeople.some((lp) => lp.id === person.id)) return;
    setLinkedPeople((prev) => [
      ...prev,
      { id: person.id, name: person.name, email: person.email, role: person.role },
    ]);
    setPeopleSearch("");
  }

  function unlinkPerson(id: string) {
    setLinkedPeople((prev) => prev.filter((p) => p.id !== id));
  }

  const filteredPeople = existingPeople.filter((p) => {
    if (linkedPeople.some((lp) => lp.id === p.id)) return false;
    if (!peopleSearch.trim()) return false;
    const q = peopleSearch.toLowerCase();
    return p.name.toLowerCase().includes(q) || p.email.toLowerCase().includes(q);
  });

  async function handleCreate() {
    setSaving(true);
    try {
      const cleaned = colleagues
        .map((c) => ({
          name: c.name.trim(),
          email: c.email.trim(),
          title: c.title?.trim() || undefined,
        }))
        .filter((c) => c.name && c.email);

      await invoke("create_internal_organization", {
        companyName: company.trim(),
        domains,
        teamName: teamName.trim(),
        colleagues: cleaned,
        existingPersonIds: linkedPeople.map((lp) => lp.id),
      });
      onNext();
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <div style={{ height: 224 }} />;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="Internal Team Setup"
        epigraph={`Create your internal organization under Internal/{company || "{Company}"} and seed your first team.`}
      />

      <StepLabel step={step + 1} total={4} />

      {step === 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
          <div>
            <FieldLabel>Company name</FieldLabel>
            <Input value={company} onChange={(e) => setCompany(e.target.value)} placeholder="Acme Inc" style={inputStyle} />
          </div>
          <div>
            <FieldLabel>Company domains</FieldLabel>
            <div className="flex gap-2">
              <Input
                value={domainInput}
                onChange={(e) => setDomainInput(e.target.value)}
                placeholder="acme.com"
                onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); addDomain(); } }}
                style={inputStyle}
              />
              <Button type="button" variant="outline" onClick={addDomain}>Add</Button>
            </div>
            {domains.length > 0 && (
              <div style={{ display: "flex", flexWrap: "wrap", gap: 8, paddingTop: 8 }}>
                {domains.map((domain) => (
                  <button
                    key={domain}
                    type="button"
                    onClick={() => setDomains((prev) => prev.filter((d) => d !== domain))}
                    style={{
                      border: "1px solid var(--color-rule-heavy)",
                      borderRadius: 4,
                      padding: "2px 8px",
                      fontSize: 12,
                      color: "var(--color-text-secondary)",
                      background: "none",
                      cursor: "pointer",
                    }}
                    title="Remove"
                  >
                    {domain}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {step === 1 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
          <div>
            <FieldLabel>Your title</FieldLabel>
            <Input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Engineering Manager" style={inputStyle} />
          </div>
          <div>
            <FieldLabel>Immediate team</FieldLabel>
            <Input value={teamName} onChange={(e) => setTeamName(e.target.value)} placeholder="Core Team" style={inputStyle} />
          </div>
        </div>
      )}

      {step === 2 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
          {/* Link existing people */}
          {existingPeople.length > 0 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <p style={{ fontSize: 13, fontWeight: 500, color: "var(--color-text-secondary)", display: "flex", alignItems: "center", gap: 6, margin: 0 }}>
                <Link size={14} /> Link existing people
              </p>
              <Input
                placeholder="Search people by name or email..."
                value={peopleSearch}
                onChange={(e) => setPeopleSearch(e.target.value)}
                style={inputStyle}
              />
              {filteredPeople.length > 0 && (
                <div style={{ maxHeight: 144, overflowY: "auto" }}>
                  {filteredPeople.slice(0, 8).map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      style={{
                        display: "flex",
                        width: "100%",
                        alignItems: "center",
                        justifyContent: "space-between",
                        padding: "8px 12px",
                        fontSize: 14,
                        borderBottom: "1px solid var(--color-rule-light)",
                        background: "none",
                        border: "none",
                        borderBottomStyle: "solid",
                        borderBottomWidth: 1,
                        borderBottomColor: "var(--color-rule-light)",
                        cursor: "pointer",
                        textAlign: "left",
                      }}
                      onClick={() => linkExistingPerson(p)}
                    >
                      <span>{p.name} <span style={{ color: "var(--color-text-tertiary)" }}>({p.email})</span></span>
                      <Plus size={14} style={{ color: "var(--color-text-tertiary)" }} />
                    </button>
                  ))}
                </div>
              )}
              {linkedPeople.length > 0 && (
                <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
                  {linkedPeople.map((lp) => (
                    <button
                      key={lp.id}
                      type="button"
                      onClick={() => unlinkPerson(lp.id)}
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        border: "1px solid var(--color-rule-heavy)",
                        borderRadius: 4,
                        padding: "2px 10px",
                        fontSize: 12,
                        background: "none",
                        cursor: "pointer",
                      }}
                      title="Remove"
                    >
                      {lp.name}
                      <span style={{ color: "var(--color-text-tertiary)" }}>&times;</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* Add new people */}
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <p style={{ fontSize: 13, fontWeight: 500, color: "var(--color-text-secondary)", display: "flex", alignItems: "center", gap: 6, margin: 0 }}>
                <UserPlus size={14} /> Add new teammates
              </p>
              <Button type="button" size="sm" variant="outline" onClick={addColleague}>
                <Plus className="mr-1 size-4" /> Add
              </Button>
            </div>
            {colleagues.length === 0 && linkedPeople.length === 0 && (
              <p
                style={{
                  fontFamily: "var(--font-serif)",
                  fontStyle: "italic",
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                  margin: 0,
                }}
              >
                No teammates added yet.
              </p>
            )}
            {colleagues.map((row, idx) => (
              <div
                key={row._key}
                style={{
                  borderTop: "1px solid var(--color-rule-light)",
                  paddingTop: 12,
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: 8,
                }}
              >
                <Input placeholder="Name" value={row.name} onChange={(e) => updateColleague(idx, { name: e.target.value })} style={inputStyle} />
                <Input placeholder="Email" value={row.email} onChange={(e) => updateColleague(idx, { email: e.target.value })} style={inputStyle} />
                <Input placeholder="Title" value={row.title ?? ""} onChange={(e) => updateColleague(idx, { title: e.target.value })} style={inputStyle} />
                <Button type="button" variant="ghost" size="sm" onClick={() => removeColleague(idx)}>
                  Remove
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}

      {step === 3 && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-light)",
            paddingTop: 20,
            display: "flex",
            flexDirection: "column",
            gap: 12,
            fontSize: 14,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Building2 size={16} style={{ color: "var(--color-text-tertiary)" }} />
            <span style={{ color: "var(--color-text-primary)" }}>{company}</span>
          </div>
          <div style={{ color: "var(--color-text-secondary)" }}>Domains: {domains.join(", ")}</div>
          <div style={{ color: "var(--color-text-secondary)" }}>Team: {teamName}</div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Users size={16} style={{ color: "var(--color-text-tertiary)" }} />
            <span style={{ color: "var(--color-text-primary)" }}>
              {linkedPeople.length + colleagues.filter((c) => c.name && c.email).length} teammates
              {linkedPeople.length > 0 && ` (${linkedPeople.length} existing)`}
            </span>
          </div>
        </div>
      )}

      <div className="flex justify-between">
        <Button variant="ghost" disabled={step === 0 || saving} onClick={() => setStep((s) => Math.max(0, s - 1))}>
          Back
        </Button>
        {step < 3 ? (
          <Button disabled={!canAdvance} onClick={() => setStep((s) => s + 1)}>
            Continue
            <ArrowRight className="ml-2 size-4" />
          </Button>
        ) : (
          <Button onClick={handleCreate} disabled={saving || !canAdvance}>
            {saving ? "Creating..." : "Create Internal Organization"}
            <ArrowRight className="ml-2 size-4" />
          </Button>
        )}
      </div>
    </div>
  );
}

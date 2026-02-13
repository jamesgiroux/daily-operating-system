import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Building2, Plus, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface ColleagueInput {
  name: string;
  email: string;
  title?: string;
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

  useEffect(() => {
    async function load() {
      try {
        const status = await invoke<SetupStatus>("get_internal_team_setup_status");
        if (!status.required) {
          onNext();
          return;
        }
        setCompany(status.prefill.company ?? "");
        setDomains(status.prefill.domains ?? []);
        setTitle(status.prefill.title ?? "");
        setTeamName(status.prefill.suggestedTeamName || "Core Team");
        setColleagues(status.prefill.suggestedColleagues ?? []);
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
    setColleagues((prev) => [...prev, { name: "", email: "", title: "" }]);
  }

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
      });
      onNext();
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <div className="h-56" />;
  }

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">Internal Team Setup</h2>
        <p className="text-sm text-muted-foreground">
          Create your internal organization under <code>Internal/{"{Company}"}</code> and seed your first team.
        </p>
      </div>

      <div className="text-xs text-muted-foreground">Step {step + 1} of 4</div>

      {step === 0 && (
        <div className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Company name</label>
            <Input
              value={company}
              onChange={(e) => setCompany(e.target.value)}
              placeholder="Acme Inc"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Company domains</label>
            <div className="flex gap-2">
              <Input
                value={domainInput}
                onChange={(e) => setDomainInput(e.target.value)}
                placeholder="acme.com"
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    addDomain();
                  }
                }}
              />
              <Button type="button" variant="outline" onClick={addDomain}>Add</Button>
            </div>
            {domains.length > 0 && (
              <div className="flex flex-wrap gap-2 pt-1">
                {domains.map((domain) => (
                  <button
                    key={domain}
                    type="button"
                    onClick={() => setDomains((prev) => prev.filter((d) => d !== domain))}
                    className="rounded-full border px-2 py-0.5 text-xs text-muted-foreground hover:text-foreground"
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
        <div className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Your title</label>
            <Input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Engineering Manager"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Immediate team</label>
            <Input
              value={teamName}
              onChange={(e) => setTeamName(e.target.value)}
              placeholder="Core Team"
            />
          </div>
        </div>
      )}

      {step === 2 && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <p className="text-sm text-muted-foreground">Add initial teammates (name, email, title).</p>
            <Button type="button" size="sm" variant="outline" onClick={addColleague}>
              <Plus className="mr-1 size-4" /> Add
            </Button>
          </div>
          {colleagues.length === 0 && (
            <div className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">
              No teammates added yet.
            </div>
          )}
          {colleagues.map((row, idx) => (
            <div key={`${row.email}-${idx}`} className="grid grid-cols-1 gap-2 rounded-md border p-3 md:grid-cols-3">
              <Input
                placeholder="Name"
                value={row.name}
                onChange={(e) => updateColleague(idx, { name: e.target.value })}
              />
              <Input
                placeholder="Email"
                value={row.email}
                onChange={(e) => updateColleague(idx, { email: e.target.value })}
              />
              <div className="flex gap-2">
                <Input
                  placeholder="Title"
                  value={row.title ?? ""}
                  onChange={(e) => updateColleague(idx, { title: e.target.value })}
                />
                <Button type="button" variant="ghost" size="sm" onClick={() => removeColleague(idx)}>
                  Remove
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {step === 3 && (
        <div className="space-y-3 rounded-lg border bg-muted/30 p-4 text-sm">
          <div className="flex items-center gap-2"><Building2 className="size-4" /> <span>{company}</span></div>
          <div className="text-muted-foreground">Domains: {domains.join(", ")}</div>
          <div className="text-muted-foreground">Team: {teamName}</div>
          <div className="flex items-center gap-2"><Users className="size-4" /> <span>{colleagues.filter((c) => c.name && c.email).length} teammates</span></div>
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

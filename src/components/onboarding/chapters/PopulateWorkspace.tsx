import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { EntityMode } from "@/types";

interface PopulateWorkspaceProps {
  entityMode: EntityMode;
  onNext: () => void;
}

const COPY = {
  account: {
    title: "Add your accounts",
    subtitle:
      "The customers, clients, or partners you work with most. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Acme Corp",
    prompt: "Start with 3\u20135 \u2014 you can always add more later.",
  },
  project: {
    title: "Add your projects",
    subtitle:
      "The initiatives, features, or campaigns you\u2019re actively working on. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Q2 Platform Migration",
    prompt: "Start with 3\u20135 \u2014 you can always add more later.",
  },
  both: {
    title: "Add your accounts and projects",
    subtitle:
      "The companies you work with and the initiatives you\u2019re driving. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Acme Corp",
    prompt: "Start with a few of each \u2014 you can always add more later.",
  },
};

/** Mono uppercase section label */
function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 10,
        fontWeight: 500,
        textTransform: "uppercase" as const,
        letterSpacing: "0.1em",
        color: "var(--color-text-tertiary)",
        marginBottom: 8,
      }}
    >
      {children}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--color-paper-warm-white)",
  border: "1px solid var(--color-desk-charcoal)",
  borderRadius: 4,
};

export function PopulateWorkspace({ entityMode, onNext }: PopulateWorkspaceProps) {
  const [accounts, setAccounts] = useState<string[]>([]);
  const [projects, setProjects] = useState<string[]>([]);
  const [accountInput, setAccountInput] = useState("");
  const [projectInput, setProjectInput] = useState("");

  const copy = COPY[entityMode];
  const showAccounts = entityMode === "account" || entityMode === "both";
  const showProjects = entityMode === "project" || entityMode === "both";
  const hasEntries = accounts.length > 0 || projects.length > 0;

  function addAccount() {
    const name = accountInput.trim();
    if (name && !accounts.includes(name)) {
      setAccounts([...accounts, name]);
      setAccountInput("");
    }
  }

  function addProject() {
    const name = projectInput.trim();
    if (name && !projects.includes(name)) {
      setProjects([...projects, name]);
      setProjectInput("");
    }
  }

  function removeAccount(name: string) {
    setAccounts(accounts.filter((a) => a !== name));
  }

  function removeProject(name: string) {
    setProjects(projects.filter((p) => p !== name));
  }

  async function handleContinue() {
    try {
      await invoke("populate_workspace", { accounts, projects });
    } catch (e) {
      console.error("populate_workspace failed:", e);
    }
    onNext();
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading title={copy.title} epigraph={copy.subtitle} />

      {/* Account input */}
      {showAccounts && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {entityMode === "both" && <SectionLabel>Accounts</SectionLabel>}
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder={entityMode === "both" ? "e.g. Acme Corp" : copy.placeholder}
              value={accountInput}
              onChange={(e) => setAccountInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addAccount()}
              style={inputStyle}
            />
            <Button variant="outline" size="icon" onClick={addAccount} disabled={!accountInput.trim()}>
              <Plus className="size-4" />
            </Button>
          </div>
          {accounts.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
              {accounts.map((name) => (
                <span
                  key={name}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    border: "1px solid var(--color-spice-turmeric)",
                    borderRadius: 4,
                    padding: "4px 10px",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {name}
                  <button
                    onClick={() => removeAccount(name)}
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
        </div>
      )}

      {/* Project input */}
      {showProjects && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {entityMode === "both" && <SectionLabel>Projects</SectionLabel>}
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder={entityMode === "both" ? "e.g. Q2 Platform Migration" : copy.placeholder}
              value={projectInput}
              onChange={(e) => setProjectInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addProject()}
              style={inputStyle}
            />
            <Button variant="outline" size="icon" onClick={addProject} disabled={!projectInput.trim()}>
              <Plus className="size-4" />
            </Button>
          </div>
          {projects.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
              {projects.map((name) => (
                <span
                  key={name}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    border: "1px solid var(--color-garden-olive)",
                    borderRadius: 4,
                    padding: "4px 10px",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {name}
                  <button
                    onClick={() => removeProject(name)}
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
        </div>
      )}

      <p style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>{copy.prompt}</p>

      <div className="flex justify-end">
        <Button onClick={handleContinue} disabled={!hasEntries}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
    prompt: "Start with 3\u20135 \u2014 you can always add more from Settings.",
  },
  project: {
    title: "Add your projects",
    subtitle:
      "The initiatives, features, or campaigns you\u2019re actively working on. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Q2 Platform Migration",
    prompt: "Start with 3\u20135 \u2014 you can always add more from Settings.",
  },
  both: {
    title: "Add your accounts and projects",
    subtitle:
      "The companies you work with and the initiatives you\u2019re driving. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Acme Corp",
    prompt: "Start with a few of each \u2014 you can always add more from Settings.",
  },
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
      await invoke("populate_workspace", {
        accounts,
        projects,
      });
    } catch (e) {
      console.error("populate_workspace failed:", e);
    }
    onNext();
  }

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">{copy.title}</h2>
        <p className="text-sm text-muted-foreground">{copy.subtitle}</p>
      </div>

      {/* Account input */}
      {showAccounts && (
        <div className="space-y-3">
          {entityMode === "both" && (
            <p className="text-sm font-medium">Accounts</p>
          )}
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder={
                entityMode === "both"
                  ? "e.g. Acme Corp"
                  : copy.placeholder
              }
              value={accountInput}
              onChange={(e) => setAccountInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addAccount()}
            />
            <Button
              variant="outline"
              size="icon"
              onClick={addAccount}
              disabled={!accountInput.trim()}
            >
              <Plus className="size-4" />
            </Button>
          </div>
          {accounts.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {accounts.map((name) => (
                <span
                  key={name}
                  className="inline-flex items-center gap-1.5 rounded-md border bg-muted/50 px-2.5 py-1 text-sm"
                >
                  {name}
                  <button
                    onClick={() => removeAccount(name)}
                    className="text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <X className="size-3" />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Project input */}
      {showProjects && (
        <div className="space-y-3">
          {entityMode === "both" && (
            <p className="text-sm font-medium">Projects</p>
          )}
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder={
                entityMode === "both"
                  ? "e.g. Q2 Platform Migration"
                  : copy.placeholder
              }
              value={projectInput}
              onChange={(e) => setProjectInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addProject()}
            />
            <Button
              variant="outline"
              size="icon"
              onClick={addProject}
              disabled={!projectInput.trim()}
            >
              <Plus className="size-4" />
            </Button>
          </div>
          {projects.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {projects.map((name) => (
                <span
                  key={name}
                  className="inline-flex items-center gap-1.5 rounded-md border bg-muted/50 px-2.5 py-1 text-sm"
                >
                  {name}
                  <button
                    onClick={() => removeProject(name)}
                    className="text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <X className="size-3" />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      <p className="text-xs text-muted-foreground">{copy.prompt}</p>

      <div className="flex justify-end">
        <Button onClick={handleContinue} disabled={!hasEntries}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

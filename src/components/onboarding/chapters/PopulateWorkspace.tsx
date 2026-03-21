import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { EntityMode } from "@/types";
import styles from "../onboarding.module.css";

export interface PopulateFormData {
  accounts: string[];
  projects: string[];
}

interface PopulateWorkspaceProps {
  entityMode: EntityMode;
  formData: PopulateFormData;
  onFormChange: (data: PopulateFormData) => void;
  onNext: () => void;
}

const COPY = {
  account: {
    title: "Add your accounts",
    subtitle:
      "The customers, clients, or partners you work with most. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Acme Corp",
    prompt: "Start with 3–5 — you can always add more later.",
  },
  project: {
    title: "Add your projects",
    subtitle:
      "The initiatives, features, or campaigns you're actively working on. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Q2 Platform Migration",
    prompt: "Start with 3–5 — you can always add more later.",
  },
  both: {
    title: "Add your accounts and projects",
    subtitle:
      "The companies you work with and the initiatives you're driving. DailyOS uses these to connect your meetings to the right context.",
    placeholder: "e.g. Acme Corp",
    prompt: "Start with a few of each — you can always add more later.",
  },
};

export function PopulateWorkspace({ entityMode, formData, onFormChange, onNext }: PopulateWorkspaceProps) {
  // Transient input state stays local
  const [accountInput, setAccountInput] = useState("");
  const [projectInput, setProjectInput] = useState("");

  const { accounts, projects } = formData;

  const copy = COPY[entityMode];
  const showAccounts = entityMode === "account" || entityMode === "both";
  const showProjects = entityMode === "project" || entityMode === "both";
  const hasEntries = accounts.length > 0 || projects.length > 0;

  function addAccount() {
    const name = accountInput.trim();
    if (name && !accounts.includes(name)) {
      onFormChange({ ...formData, accounts: [...accounts, name] });
      setAccountInput("");
    }
  }

  function addProject() {
    const name = projectInput.trim();
    if (name && !projects.includes(name)) {
      onFormChange({ ...formData, projects: [...projects, name] });
      setProjectInput("");
    }
  }

  function removeAccount(name: string) {
    onFormChange({ ...formData, accounts: accounts.filter((a) => a !== name) });
  }

  function removeProject(name: string) {
    onFormChange({ ...formData, projects: projects.filter((p) => p !== name) });
  }

  async function handleContinue() {
    try {
      await invoke("populate_workspace", { accounts, projects });
    } catch (e) {
      console.error("populate_workspace failed:", e); // Expected: best-effort workspace population
    }
    onNext();
  }

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading title={copy.title} epigraph={copy.subtitle} />

      {/* Account input */}
      {showAccounts && (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          {entityMode === "both" && <div className={styles.sectionLabel}>Accounts</div>}
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder={entityMode === "both" ? "e.g. Acme Corp" : copy.placeholder}
              value={accountInput}
              onChange={(e) => setAccountInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addAccount()}
              className={styles.editorialInput}
            />
            <Button variant="outline" size="icon" onClick={addAccount} disabled={!accountInput.trim()}>
              <Plus className="size-4" />
            </Button>
          </div>
          {accounts.length > 0 && (
            <div className={styles.flexWrap}>
              {accounts.map((name) => (
                <span key={name} className={styles.accountChip}>
                  {name}
                  <button
                    onClick={() => removeAccount(name)}
                    className={styles.ghostButton}
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
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          {entityMode === "both" && <div className={styles.sectionLabel}>Projects</div>}
          <div className="flex gap-2">
            <Input
              type="text"
              placeholder={entityMode === "both" ? "e.g. Q2 Platform Migration" : copy.placeholder}
              value={projectInput}
              onChange={(e) => setProjectInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addProject()}
              className={styles.editorialInput}
            />
            <Button variant="outline" size="icon" onClick={addProject} disabled={!projectInput.trim()}>
              <Plus className="size-4" />
            </Button>
          </div>
          {projects.length > 0 && (
            <div className={styles.flexWrap}>
              {projects.map((name) => (
                <span key={name} className={styles.projectChip}>
                  {name}
                  <button
                    onClick={() => removeProject(name)}
                    className={styles.ghostButton}
                  >
                    <X size={12} />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>
      )}

      <p className={styles.hintText}>{copy.prompt}</p>

      <div className="flex justify-end">
        <Button onClick={handleContinue} disabled={!hasEntries}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

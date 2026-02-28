/**
 * FirstAccountStep.tsx — Wizard step 5: add your first account (I57).
 *
 * Optional but encouraged. Seeds the system with one entity
 * so briefings have something to work with on first run.
 */

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

interface FirstAccountStepProps {
  onNext: () => void;
  onSkip: () => void;
}

const inputStyle: React.CSSProperties = {
  background: "var(--color-paper-warm-white)",
  border: "1px solid var(--color-desk-charcoal)",
  borderRadius: 4,
};

export function FirstAccountStep({ onNext, onSkip }: FirstAccountStepProps) {
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);

  async function handleCreate() {
    const trimmed = name.trim();
    if (!trimmed) return;

    setSaving(true);
    try {
      await invoke("create_account", {
        name: trimmed,
        parentId: null,
        accountType: "customer",
      });
      await invoke("set_wizard_step", { step: "first-account" });
    } catch (e) {
      console.error("Create account failed:", e);
    } finally {
      setSaving(false);
    }
    onNext();
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="Your first account"
        epigraph="Add one customer account to get started. Briefings get smarter with context."
      />

      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
        }}
      >
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            margin: "0 0 16px",
          }}
        >
          Accounts track customer relationships, health, and context.
          When you meet with someone from this account, your briefing will
          include account history and recent updates.
        </p>

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
          Account name
        </label>
        <Input
          type="text"
          placeholder="e.g. Acme Corp"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          style={inputStyle}
          autoFocus
        />
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
            invoke("set_wizard_step", { step: "first-account" }).catch(() => {});
            onSkip();
          }}
        >
          Skip
        </button>
        <Button onClick={handleCreate} disabled={saving || !name.trim()}>
          {saving ? "Creating..." : "Continue"}
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

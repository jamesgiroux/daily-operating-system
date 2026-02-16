import { useState } from "react";
import {
  ArrowRight,
  Loader2,
  Sparkles,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { invoke } from "@tauri-apps/api/core";

interface ClaudeCodeProps {
  workspacePath: string;
  onNext: (installed: boolean) => void;
}

interface ClaudeStatus {
  installed: boolean;
  authenticated: boolean;
}

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

/** Inline code block */
function CodeBlock({ children }: { children: React.ReactNode }) {
  return (
    <code
      style={{
        display: "block",
        fontFamily: "var(--font-mono)",
        fontSize: 12,
        background: "var(--color-paper-linen)",
        borderRadius: 4,
        padding: "8px 12px",
        marginTop: 8,
        color: "var(--color-text-primary)",
        whiteSpace: "pre",
      }}
    >
      {children}
    </code>
  );
}

export function ClaudeCode({ workspacePath, onNext }: ClaudeCodeProps) {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  const [checking, setChecking] = useState(false);

  async function checkStatus() {
    setChecking(true);
    try {
      const result = await invoke<ClaudeStatus>("check_claude_status");
      setStatus(result);
    } catch {
      setStatus({ installed: false, authenticated: false });
    } finally {
      setChecking(false);
    }
  }

  // Auto-check on first render
  if (status === null && !checking) {
    checkStatus();
  }

  const isReady = status?.installed && status?.authenticated;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="The AI engine behind your briefings"
      />

      {/* What Claude Code does */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
        }}
      >
        <SectionLabel>
          <Sparkles size={12} style={{ display: "inline", verticalAlign: "-1px", marginRight: 6 }} />
          Claude Code powers DailyOS intelligence
        </SectionLabel>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            margin: 0,
          }}
        >
          Claude Code generates your briefing narrative, enriches email summaries with
          recommended actions, and processes inbox files with AI classification. Without it,
          DailyOS still delivers your schedule, actions, and meeting preps — but AI summaries
          and enrichment won't be available.
        </p>
      </div>

      {/* Status display */}
      {checking && !status && (
        <div style={{ display: "flex", alignItems: "center", gap: 12, paddingTop: 8 }}>
          <Loader2 size={18} className="animate-spin" style={{ color: "var(--color-text-tertiary)" }} />
          <span style={{ fontSize: 14, color: "var(--color-text-tertiary)" }}>
            Checking for Claude Code...
          </span>
        </div>
      )}

      {status && isReady && (
        <div style={{ display: "flex", alignItems: "center", gap: 12, paddingTop: 8 }}>
          <div
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: "var(--color-garden-sage)",
              flexShrink: 0,
            }}
          />
          <div>
            <SectionLabel>Ready</SectionLabel>
            <p style={{ fontSize: 14, color: "var(--color-text-primary)", margin: 0 }}>
              Claude Code is installed and authenticated
            </p>
          </div>
        </div>
      )}

      {status && status.installed && !status.authenticated && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div
            style={{
              borderTop: "1px solid var(--color-rule-light)",
              paddingTop: 20,
            }}
          >
            <SectionLabel>
              <span style={{ color: "var(--color-spice-terracotta)" }}>Action needed</span>
            </SectionLabel>
            <p style={{ fontSize: 14, color: "var(--color-text-secondary)", margin: 0, marginBottom: 4 }}>
              Claude Code is installed but not signed in. Open your terminal and authenticate:
            </p>
            <CodeBlock>
              cd {workspacePath}{"\n"}claude login
            </CodeBlock>
            <p style={{ fontSize: 12, color: "var(--color-text-tertiary)", margin: "8px 0 0" }}>
              Running from your workspace directory scopes Claude's access to just that folder.
            </p>
          </div>
          <Button
            variant="outline"
            className="w-full"
            onClick={checkStatus}
            disabled={checking}
          >
            {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
            Re-check
          </Button>
        </div>
      )}

      {status && !status.installed && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          <div
            style={{
              borderTop: "1px solid var(--color-rule-light)",
              paddingTop: 20,
            }}
          >
            <SectionLabel>
              <span style={{ color: "var(--color-spice-terracotta)" }}>Not found</span>
            </SectionLabel>
            <p style={{ fontSize: 14, color: "var(--color-text-secondary)", margin: 0, marginBottom: 4 }}>
              Install Claude Code from your terminal:
            </p>
            <CodeBlock>npm install -g @anthropic-ai/claude-code</CodeBlock>
            <p style={{ fontSize: 12, color: "var(--color-text-tertiary)", margin: "12px 0 4px" }}>
              Then navigate to your workspace and sign in:
            </p>
            <CodeBlock>
              cd {workspacePath}{"\n"}claude login
            </CodeBlock>
          </div>
          <Button
            variant="outline"
            className="w-full"
            onClick={checkStatus}
            disabled={checking}
          >
            {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
            Re-check
          </Button>
        </div>
      )}

      {/* Continue / skip */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        {!isReady && status && (
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
            onClick={() => onNext(false)}
          >
            Skip — AI features will be limited
          </button>
        )}
        <div style={{ marginLeft: "auto" }}>
          <Button onClick={() => onNext(isReady ?? false)} disabled={checking && !status}>
            Continue
            <ArrowRight className="ml-2 size-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}

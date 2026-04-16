import { useState, useEffect, useRef } from "react";
import {
  ArrowRight,
  Loader2,
  Sparkles,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import styles from "../onboarding.module.css";

interface ClaudeCodeProps {
  workspacePath: string;
  onNext: (installed: boolean) => void;
  onSkip?: () => void;
}

interface ClaudeStatus {
  installed: boolean;
  authenticated: boolean;
  nodeInstalled: boolean;
}

export function ClaudeCode({ workspacePath, onNext, onSkip }: ClaudeCodeProps) {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installMessage, setInstallMessage] = useState<string | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);
  const [isDevMode, setIsDevMode] = useState(false);

  // Listen for install progress events from the backend (DOS-65)
  useEffect(() => {
    const unlisten = listen<{ step: string; status: string; message: string }>(
      "install-claude-progress",
      (event) => {
        const { step, status: evtStatus, message } = event.payload;
        if (evtStatus === "error") {
          setInstallError(message);
          setInstallMessage(null);
        } else if (step === "complete") {
          setInstallMessage(null);
          setInstallError(null);
        } else {
          setInstallMessage(message);
          setInstallError(null);
        }
      },
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // Check if dev sandbox is active — enables skip button
  useEffect(() => {
    if (import.meta.env.DEV) {
      invoke<{ isDevDbMode?: boolean }>("dev_get_state")
        .then((s) => setIsDevMode(s.isDevDbMode === true))
        .catch(() => {});
    }
  }, []);

  async function checkStatus(clearCache = false) {
    setChecking(true);
    try {
      if (clearCache) {
        await invoke("clear_claude_status_cache");
      }
      const result = await invoke<ClaudeStatus>("check_claude_status");
      setStatus(result);
    } catch {
      setStatus({ installed: false, authenticated: false, nodeInstalled: false });
    } finally {
      setChecking(false);
    }
  }

  // Auto-check on first render
  if (status === null && !checking) {
    checkStatus();
  }

  const isReady = status?.installed && status?.authenticated;

  // Auto-advance after 1 second when ready
  const autoAdvanced = useRef(false);
  useEffect(() => {
    if (isReady && !autoAdvanced.current) {
      autoAdvanced.current = true;
      const timer = setTimeout(() => onNext(true), 1000);
      return () => clearTimeout(timer);
    }
  }, [isReady, onNext]);

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="The AI engine behind your briefings"
      />

      {/* What Claude Code does */}
      <div className={styles.ruleSection}>
        <div className={styles.sectionLabel}>
          <Sparkles size={12} className={styles.iconInline} />
          Claude Code powers DailyOS intelligence
        </div>
        <p className={styles.bodyText}>
          Claude Code generates your briefing narrative, analyzes email summaries with
          recommended actions, and processes inbox files with AI classification. Without it,
          DailyOS still delivers your schedule, actions, and meeting preps — but AI summaries
          and analysis won't be available.
        </p>
      </div>

      {/* Status display */}
      {checking && !status && (
        <div className={`${styles.flexRowMd} ${styles.pt8}`}>
          <Loader2 size={18} className={`animate-spin ${styles.tertiaryText}`} />
          <span className={`${styles.bodyText} ${styles.tertiaryText}`}>
            Checking for Claude Code...
          </span>
        </div>
      )}

      {status && isReady && (
        <div className={`${styles.flexRowMd} ${styles.pt8}`}>
          <div className={styles.statusDot} />
          <div>
            <div className={styles.sectionLabel}>Ready</div>
            <p className={`${styles.bodyText} ${styles.primaryText} ${styles.noMargin}`}>
              Claude Code is installed and authenticated
            </p>
          </div>
        </div>
      )}

      {status && status.installed && !status.authenticated && (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          <div className={styles.ruleSection}>
            <div className={styles.sectionLabel}>
              <span className={styles.dangerColor}>Action needed</span>
            </div>
            <p className={`${styles.actionText} ${styles.mb4}`}>
              Claude Code is installed but not signed in. Open your terminal and authenticate:
            </p>
            <code className={styles.codeBlock}>
              cd {workspacePath}{"\n"}claude login
            </code>
            <p className={styles.installHint}>
              Running from your workspace directory scopes Claude's access to just that folder.
            </p>
          </div>
          <Button
            variant="outline"
            className="w-full"
            onClick={() => checkStatus(true)}
            disabled={checking}
          >
            {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
            Re-check
          </Button>
        </div>
      )}

      {/* Node available — one-click install */}
      {status && !status.installed && status.nodeInstalled && (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          <div className={styles.ruleSection}>
            <div className={styles.sectionLabel}>
              <span className={styles.dangerColor}>Not found</span>
            </div>
            <p className={`${styles.actionText} ${styles.mb4}`}>
              Click below to install Claude Code automatically.
            </p>
          </div>
          {installError && (
            <div className={styles.ruleSection}>
              <p className={`${styles.actionText} ${styles.dangerColor}`}>
                {installError}
              </p>
            </div>
          )}
          <Button
            className="w-full"
            onClick={async () => {
              setInstalling(true);
              setInstallError(null);
              setInstallMessage(null);
              try {
                await invoke("install_claude_cli");
                checkStatus(true);
              } catch {
                checkStatus(true);
              } finally {
                setInstalling(false);
              }
            }}
            disabled={checking || installing}
          >
            {installing && <Loader2 className="mr-2 size-4 animate-spin" />}
            {installing
              ? installMessage ?? "Installing..."
              : installError
                ? "Try Again"
                : "Install Claude Code"}
          </Button>
          <Button
            variant="outline"
            className="w-full"
            onClick={() => checkStatus(true)}
            disabled={checking || installing}
          >
            Re-check
          </Button>
        </div>
      )}

      {/* Node not available — auto-install Node.js + Claude Code (DOS-65) */}
      {status && !status.installed && !status.nodeInstalled && (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          <div className={styles.ruleSection}>
            <div className={styles.sectionLabel}>
              <span className={styles.dangerColor}>Not found</span>
            </div>
            <p className={`${styles.actionText} ${styles.mb4}`}>
              Click below to install Node.js and Claude Code automatically.
              You will be prompted for your admin password.
            </p>
          </div>
          {installError && (
            <div className={styles.ruleSection}>
              <p className={`${styles.actionText} ${styles.dangerColor}`}>
                {installError}
              </p>
            </div>
          )}
          <Button
            className="w-full"
            onClick={async () => {
              setInstalling(true);
              setInstallError(null);
              setInstallMessage(null);
              try {
                await invoke("install_claude_cli");
                checkStatus(true);
              } catch {
                checkStatus(true);
              } finally {
                setInstalling(false);
              }
            }}
            disabled={checking || installing}
          >
            {installing && <Loader2 className="mr-2 size-4 animate-spin" />}
            {installing
              ? installMessage ?? "Installing..."
              : installError
                ? "Try Again"
                : "Install Claude Code"}
          </Button>
          <Button
            variant="outline"
            className="w-full"
            onClick={() => checkStatus(true)}
            disabled={checking || installing}
          >
            {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
            Re-check
          </Button>
        </div>
      )}

      {/* Continue / Skip */}
      <div className={`${styles.flexEnd} ${styles.gap8}`}>
        {!isReady && (onSkip ? (
          <Button variant="outline" onClick={onSkip}>
            Skip — connect later in Settings
            <ArrowRight className="ml-2 size-4" />
          </Button>
        ) : isDevMode ? (
          <Button variant="outline" onClick={() => onNext(false)}>
            Skip (Dev Mode)
            <ArrowRight className="ml-2 size-4" />
          </Button>
        ) : null)}
        <Button onClick={() => onNext(isReady ?? false)} disabled={!isReady || (checking && !status)}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

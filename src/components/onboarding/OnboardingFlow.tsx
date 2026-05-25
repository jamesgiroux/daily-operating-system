/**
 * OnboardingFlow.tsx — Single-screen first-run setup.
 *
 * Three tiles, one CTA: Connect Google · Install Claude Code · Pick a role.
 * Everything else (you-card, first account, briefing prime) happens lazily
 * inside the app — no front-loaded setup beyond what DailyOS needs to work.
 */

import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { homeDir, join } from "@tauri-apps/api/path";
import { toast } from "sonner";
import {
  Mail,
  Terminal,
  Briefcase,
  ArrowRight,
  Loader2,
} from "lucide-react";

import type { EntityMode } from "@/types";
import { Button } from "@/components/ui/button";
import { AtmosphereLayer } from "@/components/layout/AtmosphereLayer";
import { FolioBar } from "@/components/layout/FolioBar";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";

import styles from "./onboarding.module.css";

interface OnboardingFlowProps {
  onComplete: () => void;
}

interface ClaudeStatus {
  installed: boolean;
  authenticated: boolean;
  nodeInstalled: boolean;
}

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [googleReady, setGoogleReady] = useState(false);
  const [claudeReady, setClaudeReady] = useState(false);
  const [roleReady, setRoleReady] = useState(false);
  const [isDevMode, setIsDevMode] = useState(false);

  const readyCount = [googleReady, claudeReady, roleReady].filter(Boolean).length;
  const allReady = readyCount === 3;
  const canStart = allReady || isDevMode;

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    invoke<{ isDevDbMode?: boolean }>("dev_get_state")
      .then((s) => setIsDevMode(s.isDevDbMode === true))
      .catch(() => {});
  }, []);

  // Auto-create workspace at default path (dev-aware) — fires on Claude ready and on completion
  const autoCreateWorkspace = useCallback(async () => {
    try {
      const existing = await invoke<{ workspacePath?: string }>("get_config")
        .then((c) => c.workspacePath)
        .catch(() => null);
      if (existing) return;

      const home = await homeDir();
      const isDevDb = import.meta.env.DEV
        ? await invoke<{ isDevDbMode?: boolean }>("dev_get_state")
            .then((s) => s.isDevDbMode === true)
            .catch(() => false)
        : false;
      const dirName = isDevDb ? "DailyOS-dev" : "DailyOS";
      const absPath = await join(home, "Documents", dirName);
      await invoke("set_workspace_path", { path: absPath });
    } catch (e) {
      console.error("Auto-create workspace failed:", e); // Expected: best-effort workspace creation
    }
  }, []);

  async function handleStart() {
    try {
      await autoCreateWorkspace();
      await invoke("set_lock_timeout", { minutes: null }).catch(() => {});
      await invoke("set_wizard_completed");
      // Trigger immediate calendar poll if Google is connected
      try {
        const authStatus = await invoke<{ status: string }>("get_google_auth_status");
        if (authStatus.status === "authenticated") {
          invoke("run_workflow", { workflowId: "today" }).catch(() => {});
        }
      } catch {
        // Non-fatal
      }
    } catch (e) {
      console.error("Wizard completion failed:", e); // Expected: best-effort wizard completion
    }
    onComplete();
  }

  async function handleSkip() {
    try {
      await autoCreateWorkspace();
      await invoke("set_lock_timeout", { minutes: null }).catch(() => {});
    } catch {
      // Non-fatal
    }
    onComplete();
  }

  return (
    <div className={styles.wrapper}>
      <AtmosphereLayer color="turmeric" />
      <FolioBar publicationLabel="Setup" />

      <div className={styles.contentColumn}>
        <div className={`${styles.flexCol} ${styles.gap32}`}>

          {/* Brand mark */}
          <div className={styles.brandMark}>
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 433 407" width="40" height="40" aria-hidden="true">
              <path d="M159 407 161 292 57 355 0 259 102 204 0 148 57 52 161 115 159 0H273L271 115L375 52L433 148L331 204L433 259L375 355L271 292L273 407Z" fill="currentColor"/>
            </svg>
          </div>

          {/* Hero */}
          <div className={`${styles.flexCol} ${styles.gap12}`}>
            <h1 className={styles.heroHeadline}>Three things, then you're in.</h1>
            <p className={styles.bodyTextConstrained}>
              DailyOS works best with your calendar, your email, and the AI that
              writes your briefings. Pick a role so we know how to prep your day.
              Everything else you'll set up as you go.
            </p>
          </div>

          {/* Tiles */}
          <div className={styles.tileStack}>
            <GoogleTile onReadyChange={setGoogleReady} />
            <ClaudeTile
              onReadyChange={setClaudeReady}
              onJustReady={autoCreateWorkspace}
            />
            <RoleTile onReadyChange={setRoleReady} />
          </div>

          {/* CTA row */}
          <div className={styles.tileCtaRow}>
            <span className={styles.tileProgress}>
              {isDevMode && !allReady ? `${readyCount} of 3 ready · dev mode` : `${readyCount} of 3 ready`}
            </span>
            <div className={styles.tileCtaActions}>
              <button type="button" className={styles.skipButton} onClick={handleSkip}>
                Skip setup
              </button>
              <Button size="lg" onClick={handleStart} disabled={!canStart}>
                Start DailyOS
                <ArrowRight className="ml-2 size-4" />
              </Button>
            </div>
          </div>

        </div>
      </div>
    </div>
  );
}

/* ─── Tiles ───────────────────────────────────────────────────────────────── */

interface TileProps {
  onReadyChange: (ready: boolean) => void;
}

function GoogleTile({ onReadyChange }: TileProps) {
  const { status, connect, loading } = useGoogleAuth();
  const isConnected = status.status === "authenticated";

  useEffect(() => {
    onReadyChange(isConnected);
  }, [isConnected, onReadyChange]);

  const connectedEmail = status.status === "authenticated" ? status.email : "";

  return (
    <div className={`${styles.tile} ${isConnected ? styles.tileDone : ""}`}>
      <div className={styles.tileIcon}>
        <Mail size={18} strokeWidth={1.8} />
      </div>
      <div className={styles.tileBody}>
        <p className={styles.tileLabel}>01 · Connect</p>
        <p className={styles.tileName}>Google</p>
        <p className={styles.tileMeta}>
          Calendar + Gmail — meeting prep and email triage. Everything processes locally.
        </p>
      </div>
      {isConnected ? (
        <span className={`${styles.tileStatus} ${styles.tileStatusDone}`}>
          <span className={styles.tileDot} />
          {connectedEmail}
        </span>
      ) : (
        <Button onClick={connect} disabled={loading}>
          {loading ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
          Connect
        </Button>
      )}
    </div>
  );
}

interface ClaudeTileProps extends TileProps {
  onJustReady: () => void;
}

function ClaudeTile({ onReadyChange, onJustReady }: ClaudeTileProps) {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installMessage, setInstallMessage] = useState<string | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);
  const justReadyFired = useRef(false);

  const isReady = !!(status?.installed && status?.authenticated);

  useEffect(() => {
    onReadyChange(isReady);
    if (isReady && !justReadyFired.current) {
      justReadyFired.current = true;
      onJustReady();
    }
  }, [isReady, onReadyChange, onJustReady]);

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

  const checkStatus = useCallback(async (clearCache = false) => {
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
  }, []);

  useEffect(() => {
    checkStatus();
  }, [checkStatus]);

  async function handleInstall() {
    setInstalling(true);
    setInstallError(null);
    setInstallMessage(null);
    try {
      await invoke("install_claude_cli");
      await checkStatus(true);
    } catch {
      await checkStatus(true);
    } finally {
      setInstalling(false);
    }
  }

  // Determine right-column content
  let rightCol: React.ReactNode = null;
  if (checking && !status) {
    rightCol = (
      <span className={styles.tileStatus}>
        <Loader2 size={14} className="animate-spin" />
        Checking…
      </span>
    );
  } else if (isReady) {
    rightCol = (
      <span className={`${styles.tileStatus} ${styles.tileStatusDone}`}>
        <span className={styles.tileDot} />
        Ready
      </span>
    );
  } else if (status && status.installed && !status.authenticated) {
    rightCol = (
      <Button variant="outline" onClick={() => checkStatus(true)} disabled={checking}>
        {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
        Re-check
      </Button>
    );
  } else if (status && !status.installed) {
    rightCol = (
      <Button onClick={handleInstall} disabled={installing || checking}>
        {installing && <Loader2 className="mr-2 size-4 animate-spin" />}
        {installing ? installMessage ?? "Installing…" : installError ? "Try again" : "Install"}
      </Button>
    );
  }

  // Auth instructions only when installed-but-not-authed
  const showAuthInstructions = !!(status && status.installed && !status.authenticated);

  return (
    <div className={`${styles.tile} ${isReady ? styles.tileDone : ""} ${showAuthInstructions ? styles.tileWide : ""}`}>
      <div className={styles.tileIcon}>
        <Terminal size={18} strokeWidth={1.8} />
      </div>
      <div className={styles.tileBody}>
        <p className={styles.tileLabel}>02 · Install</p>
        <p className={styles.tileName}>Claude Code</p>
        <p className={styles.tileMeta}>
          The local AI engine that writes your briefings and analyzes mail.
        </p>
        {showAuthInstructions && (
          <div className={styles.tileAction}>
            <code className={styles.codeBlock}>
              cd ~/Documents/DailyOS{"\n"}claude login
            </code>
            <p className={styles.tileActionHint}>
              Run this in Terminal, then click Re-check.
            </p>
            {rightCol}
          </div>
        )}
        {installError && !showAuthInstructions && (
          <p className={`${styles.tileMeta} ${styles.dangerColor}`}>{installError}</p>
        )}
      </div>
      {!showAuthInstructions && rightCol}
    </div>
  );
}

function RoleTile({ onReadyChange }: TileProps) {
  const [presets, setPresets] = useState<[string, string, string][]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const isReady = selected !== null;

  useEffect(() => {
    onReadyChange(isReady);
  }, [isReady, onReadyChange]);

  useEffect(() => {
    invoke<[string, string, string][]>("get_available_presets")
      .then(setPresets)
      .catch((err) => {
        console.error("get_available_presets failed:", err); // Expected: background init on mount
        setPresets([]);
      });

    // Restore existing role selection if user re-enters the wizard
    invoke<{ defaultEntityMode: string; id?: string } | null>("get_active_preset")
      .then((preset) => {
        if (preset && typeof preset === "object" && "id" in preset && preset.id) {
          setSelected(preset.id);
        }
      })
      .catch(() => {
        // Non-fatal
      });
  }, []);

  async function handleSelect(presetId: string) {
    if (saving) return;
    const previous = selected;
    setSelected(presetId);
    setSaving(true);
    try {
      await invoke("set_role", { role: presetId });
      // Read back the entity mode for any downstream consumers (parity with old EntityMode chapter)
      await invoke<{ defaultEntityMode: string } | null>("get_active_preset")
        .then((preset) => (preset?.defaultEntityMode ?? "account") as EntityMode)
        .catch(() => "account");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set role");
      setSelected(previous);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className={`${styles.tile} ${styles.tileWide} ${isReady ? styles.tileDone : ""}`}>
      <div className={styles.tileIcon}>
        <Briefcase size={18} strokeWidth={1.8} />
      </div>
      <div className={styles.tileBody}>
        <p className={styles.tileLabel}>03 · Pick your role</p>
        <p className={styles.tileName}>
          {isReady && presets.length > 0
            ? presets.find(([id]) => id === selected)?.[1] ?? "Selected"
            : "What's your role?"}
        </p>
        <p className={styles.tileMeta}>
          Shapes your vitals, vocabulary, and briefing prep. Change anytime in Settings.
        </p>
        {presets.length === 0 ? (
          <p className={`${styles.tileActionHint} ${styles.tileAction}`}>Loading roles…</p>
        ) : (
          <div className={styles.roleMiniGrid}>
            {presets.map(([id, name]) => (
              <button
                key={id}
                type="button"
                className={`${styles.roleMiniChip} ${selected === id ? styles.roleMiniChipSelected : ""}`}
                aria-pressed={selected === id}
                disabled={saving && selected !== id}
                onClick={() => handleSelect(id)}
              >
                {name}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * GleanConnect.tsx — Wizard step: Connect company knowledge via Glean.
 *
 * Optional connector. When connected, enables account discovery and
 * profile pre-fill from enterprise tools (Salesforce, Zendesk, Gong, Slack).
 */

import { useState, useEffect, useRef } from "react";
import { Globe, ArrowRight, Loader2, Shield } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { useGleanAuth } from "@/hooks/useGleanAuth";
import styles from "../onboarding.module.css";

interface GleanConnectProps {
  onNext: (gleanConnected: boolean) => void;
  onSkip: () => void;
}

export function GleanConnect({ onNext, onSkip }: GleanConnectProps) {
  const {
    status: authStatus,
    connect: connectGlean,
    loading: authLoading,
    error,
    clearError,
  } = useGleanAuth();
  const [endpoint, setEndpoint] = useState("");
  const isConnected = authStatus.status === "authenticated";

  // Auto-advance after 1.5s when connected
  const autoAdvanced = useRef(false);
  useEffect(() => {
    if (isConnected && !autoAdvanced.current) {
      autoAdvanced.current = true;
      const timer = setTimeout(() => onNext(true), 1500);
      return () => clearTimeout(timer);
    }
  }, [isConnected, onNext]);

  function handleConnect() {
    const trimmed = endpoint.trim();
    if (!trimmed) return;
    clearError();
    connectGlean(trimmed);
  }

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Connect your company knowledge"
      />

      {/* Value prop */}
      <div className={styles.ruleSection}>
        <div className={styles.sectionLabel}>
          <Globe size={12} className={styles.iconInline} />
          Enterprise Intelligence
        </div>
        <p className={styles.bodyText}>
          Glean connects DailyOS to your company's tools — Salesforce, Zendesk, Gong,
          Slack, and more. With Glean, your briefings include real CRM data, support
          ticket history, and call insights instead of just calendar context.
        </p>
      </div>

      {/* Auth status / connect form */}
      {isConnected ? (
        <div className={`${styles.flexRowMd} ${styles.pt8}`}>
          <div className={styles.statusDot} />
          <div>
            <p className={styles.connectedLabel}>Connected</p>
            <p className={styles.connectedEmail}>
              {authStatus.status === "authenticated" ? authStatus.email : ""}
            </p>
          </div>
        </div>
      ) : (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          <div>
            <label className={styles.fieldLabel}>Glean MCP endpoint</label>
            <Input
              type="text"
              placeholder="e.g. https://your-company.glean.com/mcp"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleConnect()}
              className={styles.editorialInput}
              disabled={authLoading}
            />
            <p className={styles.helperText}>
              Ask your IT team for the Glean MCP endpoint URL.
            </p>
          </div>

          {error && (
            <p className={`${styles.bodyText} ${styles.dangerColor}`}>
              {error}
            </p>
          )}

          <Button
            size="lg"
            className="w-full"
            onClick={handleConnect}
            disabled={authLoading || !endpoint.trim()}
          >
            {authLoading ? (
              <Loader2 className="mr-2 size-4 animate-spin" />
            ) : (
              <Globe className="mr-2 size-4" />
            )}
            Connect Glean
          </Button>
        </div>
      )}

      {/* Privacy note */}
      <div className={styles.flexRowStart}>
        <Shield size={12} className={`${styles.mt2} ${styles.flexShrink0} ${styles.tertiaryText}`} />
        <span className={styles.privacyNote}>
          Glean queries run through your company's SSO. DailyOS stores results locally.
        </span>
      </div>

      {/* Continue / skip */}
      <div className={`${styles.flexEnd} ${styles.gap8}`}>
        <Button variant="outline" onClick={onSkip}>
          Skip — connect later in Settings
          <ArrowRight className="ml-2 size-4" />
        </Button>
        {isConnected && (
          <Button onClick={() => onNext(true)}>
            Continue
            <ArrowRight className="ml-2 size-4" />
          </Button>
        )}
      </div>
    </div>
  );
}

import {
  Mail,
  Calendar,
  Loader2,
  ArrowRight,
  Shield,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import styles from "../onboarding.module.css";

interface GoogleConnectProps {
  onNext: () => void;
}

export function GoogleConnect({ onNext }: GoogleConnectProps) {
  const { status: authStatus, connect: connectGoogle, loading: authLoading } = useGoogleAuth();
  const isConnected = authStatus.status === "authenticated";

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Every meeting, prepared. Every email, triaged."
      />

      {/* Calendar explanation */}
      <div className={styles.ruleSection}>
        <div className={styles.sectionLabel}>
          <Calendar size={12} className={styles.iconInline} />
          Calendar Intelligence
        </div>
        <p className={styles.bodyText}>
          DailyOS reads your calendar overnight. For each meeting, it builds a prep: relationship
          history, open action items, talking points, risks. The lifecycle:{" "}
          <span className={styles.monoAccent}>
            Prep &rarr; Meeting &rarr; Capture &rarr; Next Prep
          </span>
          . Each meeting feeds the next.
        </p>
      </div>

      {/* Email explanation */}
      <div className={styles.ruleSection}>
        <div className={styles.sectionLabel}>
          <Mail size={12} className={styles.iconInline} />
          Email Triage
        </div>
        <p className={styles.bodyText}>
          DailyOS triages your email by priority. Important emails surface first. Each gets an AI
          summary and a recommended action. You scan and decide — no inbox-zero required.
        </p>
      </div>

      {/* Auth status / button */}
      {isConnected ? (
        <div className={`${styles.flexRowMd} ${styles.pt8}`}>
          <div className={styles.statusDot} />
          <div>
            <p className={styles.connectedLabel}>
              Connected
            </p>
            <p className={styles.connectedEmail}>
              {authStatus.status === "authenticated" ? authStatus.email : ""}
            </p>
          </div>
        </div>
      ) : (
        <Button
          size="lg"
          className="w-full"
          onClick={connectGoogle}
          disabled={authLoading}
        >
          {authLoading ? (
            <Loader2 className="mr-2 size-4 animate-spin" />
          ) : (
            <Mail className="mr-2 size-4" />
          )}
          Connect Google Calendar & Gmail
        </Button>
      )}

      {/* Privacy note */}
      <div className={styles.flexRowStart}>
        <Shield size={12} className={`${styles.mt2} ${styles.flexShrink0} ${styles.tertiaryText}`} />
        <span className={styles.privacyNote}>
          Everything processes locally. Your data never leaves your machine.
        </span>
      </div>

      {/* Continue / skip */}
      <div className="flex justify-end">
        <Button onClick={onNext}>
          {isConnected ? "Continue" : "Skip — connect later in Settings"}
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}

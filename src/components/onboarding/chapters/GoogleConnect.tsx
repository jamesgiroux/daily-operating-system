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

interface GoogleConnectProps {
  onNext: () => void;
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

export function GoogleConnect({ onNext }: GoogleConnectProps) {
  const { status: authStatus, connect: connectGoogle, loading: authLoading } = useGoogleAuth();
  const isConnected = authStatus.status === "authenticated";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="Every meeting, prepared. Every email, triaged."
      />

      {/* Calendar explanation */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
        }}
      >
        <SectionLabel>
          <Calendar size={12} style={{ display: "inline", verticalAlign: "-1px", marginRight: 6 }} />
          Calendar Intelligence
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
          DailyOS reads your calendar overnight. For each meeting, it builds a prep: relationship
          history, open action items, talking points, risks. The lifecycle:{" "}
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-spice-turmeric)",
            }}
          >
            Prep &rarr; Meeting &rarr; Capture &rarr; Next Prep
          </span>
          . Each meeting feeds the next.
        </p>
      </div>

      {/* Email explanation */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
        }}
      >
        <SectionLabel>
          <Mail size={12} style={{ display: "inline", verticalAlign: "-1px", marginRight: 6 }} />
          Email Triage
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
          DailyOS triages your email by priority. Important emails surface first. Each gets an AI
          summary and a recommended action. You scan and decide — no inbox-zero required.
        </p>
      </div>

      {/* Auth status / button */}
      {isConnected ? (
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
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              Connected
            </p>
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                margin: "2px 0 0",
              }}
            >
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
      <div style={{ display: "flex", alignItems: "flex-start", gap: 8 }}>
        <Shield size={12} style={{ marginTop: 2, flexShrink: 0, color: "var(--color-text-tertiary)" }} />
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
          }}
        >
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

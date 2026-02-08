import {
  Mail,
  Calendar,
  Check,
  Loader2,
  ArrowRight,
  Shield,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";

interface GoogleConnectProps {
  onNext: () => void;
}

export function GoogleConnect({ onNext }: GoogleConnectProps) {
  const { status: authStatus, connect: connectGoogle, loading: authLoading } = useGoogleAuth();
  const isConnected = authStatus.status === "authenticated";

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">
          Every meeting, prepared. Every email, triaged.
        </h2>
      </div>

      {/* Calendar explanation */}
      <div className="rounded-lg border bg-muted/30 p-4 space-y-3">
        <div className="flex items-center gap-2">
          <Calendar className="size-4 text-primary" />
          <span className="text-sm font-medium">Calendar Intelligence</span>
        </div>
        <p className="text-sm text-muted-foreground leading-relaxed">
          DailyOS reads your calendar overnight. For each meeting, it builds a prep: relationship
          history, open action items, talking points, risks. The lifecycle:{" "}
          <span className="font-medium text-foreground">
            Prep &rarr; Meeting &rarr; Capture &rarr; Next Prep
          </span>
          . Each meeting feeds the next.
        </p>
      </div>

      {/* Email explanation */}
      <div className="rounded-lg border bg-muted/30 p-4 space-y-3">
        <div className="flex items-center gap-2">
          <Mail className="size-4 text-primary" />
          <span className="text-sm font-medium">Email Triage</span>
        </div>
        <p className="text-sm text-muted-foreground leading-relaxed">
          DailyOS triages your email by priority. Important emails surface first. Each gets an AI
          summary and a recommended action. You scan and decide — no inbox-zero required.
        </p>
      </div>

      {/* Auth status / button */}
      {isConnected ? (
        <div className="flex items-center gap-3 rounded-lg border bg-muted/30 p-4">
          <div className="flex size-8 items-center justify-center rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400">
            <Check className="size-4" />
          </div>
          <div>
            <p className="text-sm font-medium">Connected</p>
            <p className="text-xs text-muted-foreground">
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
      <div className="flex items-start gap-2 text-xs text-muted-foreground">
        <Shield className="mt-0.5 size-3 shrink-0" />
        <span>Everything processes locally. Your data never leaves your machine.</span>
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

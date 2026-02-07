import { Sunrise, Mail } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { PageEmpty } from "@/components/PageState";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import type { GoogleAuthStatus } from "@/types";

interface DashboardEmptyProps {
  message: string;
  onGenerate?: () => void;
  googleAuth?: GoogleAuthStatus;
}

export function DashboardEmpty({ message, onGenerate, googleAuth }: DashboardEmptyProps) {
  const { connect, loading: authLoading } = useGoogleAuth();
  const isUnauthed = googleAuth?.status === "notconfigured";

  return (
    <PageEmpty
      icon={Sunrise}
      title="No briefing yet"
      message={message}
      action={
        <div className="flex flex-col items-center gap-4">
          {onGenerate && (
            <Button
              onClick={onGenerate}
              className="bg-primary text-primary-foreground"
            >
              Generate Briefing
            </Button>
          )}
          {isUnauthed && (
            <Card className="w-full max-w-md border-dashed">
              <CardContent className="flex items-center gap-3 pt-6">
                <Mail className="size-5 shrink-0 text-muted-foreground" />
                <div className="flex-1">
                  <p className="text-sm font-medium">Connect Google</p>
                  <p className="text-xs text-muted-foreground">
                    Add calendar and email for a complete briefing
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={connect}
                  disabled={authLoading}
                >
                  Connect
                </Button>
              </CardContent>
            </Card>
          )}
        </div>
      }
      footnote="Grab a coffee — your day will be ready soon"
    />
  );
}

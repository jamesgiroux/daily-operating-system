import { Sunrise } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PageEmpty } from "@/components/PageState";

interface DashboardEmptyProps {
  message: string;
  onGenerate?: () => void;
}

export function DashboardEmpty({ message, onGenerate }: DashboardEmptyProps) {
  return (
    <PageEmpty
      icon={Sunrise}
      title="No briefing yet"
      message={message}
      action={
        onGenerate && (
          <Button
            onClick={onGenerate}
            className="bg-primary text-primary-foreground"
          >
            Generate Briefing
          </Button>
        )
      }
      footnote="Grab a coffee — your day will be ready soon"
    />
  );
}

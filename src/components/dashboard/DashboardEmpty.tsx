import { Sunrise } from "lucide-react";
import { PageEmpty } from "@/components/PageState";

interface DashboardEmptyProps {
  message: string;
}

export function DashboardEmpty({ message }: DashboardEmptyProps) {
  return (
    <PageEmpty
      icon={Sunrise}
      title="No briefing yet"
      message={message}
      footnote="Grab a coffee — your day will be ready soon"
    />
  );
}

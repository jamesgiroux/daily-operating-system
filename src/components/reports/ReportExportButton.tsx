import { Printer } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ReportExportButtonProps {
  className?: string;
}

export function ReportExportButton({ className }: ReportExportButtonProps) {
  return (
    <Button
      variant="outline"
      size="sm"
      className={className}
      onClick={() => window.print()}
      style={{ gap: "0.35rem" }}
    >
      <Printer size={14} />
      Export PDF
    </Button>
  );
}

import { PageError } from "@/components/PageState";

interface DashboardErrorProps {
  message: string;
  onRetry: () => void;
}

export function DashboardError({ message, onRetry }: DashboardErrorProps) {
  return <PageError message={message} onRetry={onRetry} />;
}

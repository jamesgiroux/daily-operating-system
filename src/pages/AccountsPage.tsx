import { PageEmpty } from "@/components/PageState";
import { Building2 } from "lucide-react";

export default function AccountsPage() {
  return (
    <main className="flex-1 overflow-hidden">
      <PageEmpty
        icon={Building2}
        title="Accounts coming soon"
        message="Account health, renewal tracking, and engagement signals will live here."
      />
    </main>
  );
}

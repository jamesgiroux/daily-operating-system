import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Building2 } from "lucide-react";

export default function AccountsPage() {
  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">Accounts</h1>
            <p className="text-sm text-muted-foreground">
              Customer accounts and health overview
            </p>
          </div>

          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12 text-center">
              <Building2 className="mb-4 size-12 text-muted-foreground/40" />
              <p className="text-lg font-medium">Coming in Phase 2</p>
              <p className="text-sm text-muted-foreground">
                Account health, renewal tracking, and engagement signals will live here.
              </p>
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    </main>
  );
}

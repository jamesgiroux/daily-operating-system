import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { History, AlertCircle } from "lucide-react";
import type { ProcessingLogEntry } from "@/types";

export default function HistoryPage() {
  const [entries, setEntries] = useState<ProcessingLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      try {
        const result = await invoke<ProcessingLogEntry[]>(
          "get_processing_history",
          { limit: 50 },
        );
        setEntries(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12" />
          ))}
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">
              Processing History
            </h1>
            <p className="text-sm text-muted-foreground">
              Recent inbox processing activity
            </p>
          </div>

          {error ? (
            <div className="flex items-center gap-2 rounded-md border border-destructive p-4 text-destructive">
              <AlertCircle className="size-5" />
              <p className="text-sm">{error}</p>
            </div>
          ) : entries.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-16 text-center">
              <History className="mb-3 size-10 text-muted-foreground/50" />
              <p className="text-sm text-muted-foreground">
                No processing history yet.
              </p>
              <p className="text-xs text-muted-foreground">
                Files processed from the inbox will appear here.
              </p>
            </div>
          ) : (
            <div className="rounded-md border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="px-4 py-2 text-left font-medium">File</th>
                    <th className="px-4 py-2 text-left font-medium">
                      Classification
                    </th>
                    <th className="px-4 py-2 text-left font-medium">Status</th>
                    <th className="px-4 py-2 text-left font-medium">
                      Destination
                    </th>
                    <th className="px-4 py-2 text-left font-medium">Time</th>
                    <th className="px-4 py-2 text-left font-medium">Error</th>
                  </tr>
                </thead>
                <tbody>
                  {entries.map((entry) => (
                    <tr key={entry.id} className="border-b last:border-0">
                      <td className="max-w-[200px] truncate px-4 py-2 font-mono text-xs">
                        {entry.filename}
                      </td>
                      <td className="px-4 py-2">
                        <Badge variant="outline" className="text-xs">
                          {entry.classification}
                        </Badge>
                      </td>
                      <td className="px-4 py-2">
                        <Badge
                          variant={
                            entry.status === "routed"
                              ? "default"
                              : entry.status === "error"
                                ? "destructive"
                                : "secondary"
                          }
                          className="text-xs"
                        >
                          {entry.status}
                        </Badge>
                      </td>
                      <td className="max-w-[200px] truncate px-4 py-2 font-mono text-xs text-muted-foreground">
                        {entry.destinationPath || "-"}
                      </td>
                      <td className="whitespace-nowrap px-4 py-2 text-xs text-muted-foreground">
                        {formatTimestamp(entry.createdAt)}
                      </td>
                      <td className="max-w-[200px] truncate px-4 py-2 text-xs text-destructive">
                        {entry.errorMessage || ""}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return ts;
  }
}

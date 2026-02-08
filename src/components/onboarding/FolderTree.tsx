import { Folder, FileText, Inbox, Archive } from "lucide-react";
import { cn } from "@/lib/utils";

interface FolderEntry {
  name: string;
  annotation: string;
  icon: React.ElementType;
  indent?: boolean;
}

interface FolderTreeProps {
  entityMode: string;
}

export function FolderTree({ entityMode }: FolderTreeProps) {
  const folders: FolderEntry[] = [
    {
      name: "_today/",
      annotation: "Your daily briefing — updated automatically each morning",
      icon: FileText,
    },
    {
      name: "_inbox/",
      annotation: "Drop files here — transcripts, notes, docs — AI processes them",
      icon: Inbox,
    },
    {
      name: "_archive/",
      annotation: "Yesterday's briefings — the system maintains your history",
      icon: Archive,
    },
  ];

  if (entityMode === "account" || entityMode === "both") {
    folders.push({
      name: "Accounts/",
      annotation: "One folder per account with context that enriches meeting prep",
      icon: Folder,
    });
  }
  if (entityMode === "project" || entityMode === "both") {
    folders.push({
      name: "Projects/",
      annotation: "One folder per project with context and tracking",
      icon: Folder,
    });
  }

  return (
    <div className="rounded-lg border bg-muted/30 p-4">
      <div className="mb-3 flex items-center gap-2">
        <Folder className="size-4 text-primary" />
        <span className="font-mono text-sm font-medium">~/Documents/DailyOS/</span>
      </div>
      <div className="space-y-2 pl-6">
        {folders.map((entry) => {
          const Icon = entry.icon;
          return (
            <div key={entry.name} className="flex items-start gap-3">
              <Icon className={cn("mt-0.5 size-4 shrink-0 text-muted-foreground")} />
              <div className="min-w-0">
                <span className="font-mono text-sm">{entry.name}</span>
                <p className="text-xs text-muted-foreground">{entry.annotation}</p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

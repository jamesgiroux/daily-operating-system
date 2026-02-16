import { Folder, FileText, Inbox, Archive } from "lucide-react";

interface FolderEntry {
  name: string;
  annotation: string;
  icon: React.ElementType;
  iconColor: string;
}

interface FolderTreeProps {
  entityMode: string;
  rootPath?: string;
}

export function FolderTree({ entityMode, rootPath }: FolderTreeProps) {
  const displayPath = rootPath
    ? rootPath.replace(/^\/Users\/[^/]+/, "~").replace(/\/?$/, "/")
    : "~/Documents/DailyOS/";
  const folders: FolderEntry[] = [
    {
      name: "_today/",
      annotation: "Your daily briefing — updated automatically each morning",
      icon: FileText,
      iconColor: "var(--color-spice-turmeric)",
    },
    {
      name: "_inbox/",
      annotation: "Drop files here — transcripts, notes, docs — AI processes them",
      icon: Inbox,
      iconColor: "var(--color-spice-turmeric)",
    },
    {
      name: "_archive/",
      annotation: "Yesterday's briefings — the system maintains your history",
      icon: Archive,
      iconColor: "var(--color-spice-turmeric)",
    },
  ];

  if (entityMode === "account" || entityMode === "both") {
    folders.push({
      name: "Accounts/",
      annotation: "One folder per account with context that enriches meeting prep",
      icon: Folder,
      iconColor: "var(--color-spice-turmeric)",
    });
  }
  if (entityMode === "project" || entityMode === "both") {
    folders.push({
      name: "Projects/",
      annotation: "One folder per project with context and tracking",
      icon: Folder,
      iconColor: "var(--color-garden-olive)",
    });
  }

  return (
    <div style={{ paddingTop: 20 }}>
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 16,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
          <Folder size={16} style={{ color: "var(--color-spice-turmeric)" }} />
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              fontWeight: 500,
              color: "var(--color-spice-turmeric)",
            }}
          >
            {displayPath}
          </span>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 12, paddingLeft: 24 }}>
          {folders.map((entry) => {
            const Icon = entry.icon;
            return (
              <div key={entry.name} style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
                <Icon
                  size={16}
                  style={{
                    marginTop: 2,
                    flexShrink: 0,
                    color: entry.iconColor,
                  }}
                />
                <div>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 13,
                      color: "var(--color-text-primary)",
                    }}
                  >
                    {entry.name}
                  </span>
                  <p
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 12,
                      color: "var(--color-text-secondary)",
                      margin: "2px 0 0",
                    }}
                  >
                    {entry.annotation}
                  </p>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

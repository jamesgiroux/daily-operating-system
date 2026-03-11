import { Folder, FileText, Inbox, Archive } from "lucide-react";
import styles from "./onboarding.module.css";

interface FolderEntry {
  name: string;
  annotation: string;
  icon: React.ElementType;
  iconColorClass: string;
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
      iconColorClass: styles.accentColor,
    },
    {
      name: "_inbox/",
      annotation: "Drop files here — transcripts, notes, docs — AI processes them",
      icon: Inbox,
      iconColorClass: styles.accentColor,
    },
    {
      name: "_archive/",
      annotation: "Yesterday's briefings — the system maintains your history",
      icon: Archive,
      iconColorClass: styles.accentColor,
    },
  ];

  if (entityMode === "account" || entityMode === "both") {
    folders.push({
      name: "Accounts/",
      annotation: "One folder per account with context that shapes meeting briefings",
      icon: Folder,
      iconColorClass: styles.accentColor,
    });
  }
  if (entityMode === "project" || entityMode === "both") {
    folders.push({
      name: "Projects/",
      annotation: "One folder per project with context and tracking",
      icon: Folder,
      iconColorClass: styles.oliveColor,
    });
  }

  return (
    <div className={styles.folderTreeRoot}>
      <div className={styles.folderTreeInner}>
        <div className={styles.folderTreeHeader}>
          <Folder size={16} className={styles.accentColor} />
          <span className={styles.monoPath}>
            {displayPath}
          </span>
        </div>
        <div className={styles.folderTreeList}>
          {folders.map((entry) => {
            const Icon = entry.icon;
            return (
              <div key={entry.name} className={styles.folderEntry}>
                <Icon
                  size={16}
                  className={`${styles.folderIcon} ${styles.flexShrink0} ${entry.iconColorClass}`}
                />
                <div>
                  <span className={styles.monoText}>
                    {entry.name}
                  </span>
                  <p className={styles.folderAnnotation}>
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

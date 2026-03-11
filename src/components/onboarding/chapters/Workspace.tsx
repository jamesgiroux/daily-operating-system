import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { toast } from "sonner";
import { FolderOpen, Loader2, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FolderTree } from "@/components/onboarding/FolderTree";
import styles from "../onboarding.module.css";

interface WorkspaceProps {
  entityMode: string;
  onNext: (path: string) => void;
}

export function Workspace({ entityMode, onNext }: WorkspaceProps) {
  const [homePath, setHomePath] = useState("");
  const [saving, setSaving] = useState(false);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  useEffect(() => {
    homeDir().then(setHomePath).catch((err) => console.error("homeDir failed:", err));
  }, []);

  const defaultWorkspacePath = homePath
    ? `${homePath.endsWith("/") ? homePath : homePath + "/"}Documents/DailyOS`
    : "";
  const defaultWorkspaceDisplay = "~/Documents/DailyOS";

  async function handleWorkspacePath(path: string) {
    setSaving(true);
    try {
      await invoke("set_workspace_path", { path });
      setSelectedPath(path);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set workspace");
    } finally {
      setSaving(false);
    }
  }

  async function handleChooseWorkspace() {
    const selected = await open({
      directory: true,
      title: "Choose workspace directory",
    });
    if (selected) {
      await handleWorkspacePath(selected);
    }
  }

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Your files, on your machine"
        epigraph="Everything DailyOS creates lives in a folder you control. Briefings, meeting notes, actions — plain files you can open, search, or move anywhere."
      />

      {selectedPath ? (
        <FolderTree entityMode={entityMode} rootPath={selectedPath} />
      ) : (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          <Button
            className="w-full justify-between"
            onClick={() => defaultWorkspacePath && handleWorkspacePath(defaultWorkspacePath)}
            disabled={saving || !defaultWorkspacePath}
          >
            <div className="flex items-center gap-2">
              <FolderOpen className="size-4" />
              <span>Use default location</span>
            </div>
            <span className={styles.monoTinyPath}>
              {defaultWorkspaceDisplay}
            </span>
          </Button>

          {/* "or" divider — short centered rule */}
          <div className={styles.orDivider}>
            <div className={styles.orDividerRule} />
            <span className={styles.orDividerText}>
              or
            </span>
            <div className={styles.orDividerRule} />
          </div>

          <Button
            variant="outline"
            className="w-full"
            onClick={handleChooseWorkspace}
            disabled={saving}
          >
            {saving ? (
              <Loader2 className="mr-2 size-4 animate-spin" />
            ) : (
              <FolderOpen className="mr-2 size-4" />
            )}
            Choose a different folder
          </Button>
        </div>
      )}

      {selectedPath && (
        <>
          <p className={styles.workspaceHint}>
            Drop transcripts, notes, or documents into{" "}
            <span className={styles.inboxFolder}>
              _inbox/
            </span>{" "}
            anytime. DailyOS processes them automatically.
          </p>

          <div className="flex justify-end">
            <Button onClick={() => onNext(selectedPath)}>
              Continue
              <ArrowRight className="ml-2 size-4" />
            </Button>
          </div>
        </>
      )}
    </div>
  );
}

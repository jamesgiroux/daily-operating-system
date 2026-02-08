import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { toast } from "sonner";
import { FolderOpen, Loader2, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { FolderTree } from "@/components/onboarding/FolderTree";

interface WorkspaceProps {
  entityMode: string;
  onNext: (path: string) => void;
}

export function Workspace({ entityMode, onNext }: WorkspaceProps) {
  const [homePath, setHomePath] = useState("");
  const [saving, setSaving] = useState(false);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  useEffect(() => {
    homeDir().then(setHomePath).catch(() => {});
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
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">
          Your files, on your machine
        </h2>
        <p className="text-sm text-muted-foreground">
          Everything DailyOS creates lives in a folder you control.
          Briefings, meeting prep, actions — plain files you can open, search, or move anywhere.
        </p>
      </div>

      {selectedPath ? (
        <FolderTree entityMode={entityMode} />
      ) : (
        <div className="space-y-3">
          <Button
            className="w-full justify-between"
            onClick={() => defaultWorkspacePath && handleWorkspacePath(defaultWorkspacePath)}
            disabled={saving || !defaultWorkspacePath}
          >
            <div className="flex items-center gap-2">
              <FolderOpen className="size-4" />
              <span>Use default location</span>
            </div>
            <code className="text-xs opacity-70">{defaultWorkspaceDisplay}</code>
          </Button>

          <div className="relative">
            <div className="absolute inset-0 flex items-center">
              <div className="w-full border-t" />
            </div>
            <div className="relative flex justify-center text-xs">
              <span className="bg-background px-2 text-muted-foreground">or</span>
            </div>
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
          <p className="text-xs text-muted-foreground text-center">
            Drop transcripts, notes, or documents into{" "}
            <code className="rounded bg-muted px-1">_inbox/</code> anytime. DailyOS processes them
            automatically.
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

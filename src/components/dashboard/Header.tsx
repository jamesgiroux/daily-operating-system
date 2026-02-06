import { Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { ModeToggle } from "@/components/mode-toggle";
import { StatusIndicator } from "./StatusIndicator";
import { RunNowIconButton } from "./RunNowButton";
import { useWorkflow } from "@/hooks/useWorkflow";

interface HeaderProps {
  onCommandMenuOpen: () => void;
}

export function Header({ onCommandMenuOpen }: HeaderProps) {
  const { status, nextRunTime, runNow, isRunning } = useWorkflow();

  return (
    <header
      data-tauri-drag-region
      className="flex h-14 shrink-0 items-center gap-2 border-b px-4"
    >
      <SidebarTrigger className="-ml-1" />
      <Separator orientation="vertical" className="mr-2 h-4" />

      <div className="flex flex-1 items-center justify-between">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-semibold">Overview</h1>
          <StatusIndicator status={status} nextRunTime={nextRunTime} />
        </div>

        <div className="flex items-center gap-2">
          <RunNowIconButton onClick={runNow} isRunning={isRunning} />
          <Button
            variant="outline"
            size="sm"
            className="h-8 w-48 justify-start text-muted-foreground"
            onClick={onCommandMenuOpen}
          >
            <Search className="mr-2 size-4" />
            <span className="flex-1 text-left">Search...</span>
            <kbd className="pointer-events-none ml-auto hidden h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium opacity-100 sm:flex">
              <span className="text-xs">⌘</span>K
            </kbd>
          </Button>
          <ModeToggle />
        </div>
      </div>
    </header>
  );
}

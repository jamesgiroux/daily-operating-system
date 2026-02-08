import { Copy, Check } from "lucide-react";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface CopyButtonProps {
  text: string;
  label?: string;
  className?: string;
}

export function CopyButton({ text, label, className }: CopyButtonProps) {
  const { copied, copy } = useCopyToClipboard();

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className={cn("size-7 text-muted-foreground hover:text-foreground", className)}
            onClick={(e) => {
              e.stopPropagation();
              copy(text);
            }}
          >
            {copied ? (
              <Check className="size-3.5 text-success" />
            ) : (
              <Copy className="size-3.5" />
            )}
            <span className="sr-only">Copy {label ?? "to clipboard"}</span>
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {copied ? "Copied!" : `Copy ${label ?? "to clipboard"}`}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Clock, Target } from "lucide-react";
import type { FocusBlock, TimeBlock } from "@/types";

interface FocusBlocksStepProps {
  blocks: TimeBlock[];
  onSubmit: (selected: FocusBlock[]) => void;
  onSkip: () => void;
}

export function FocusBlocksStep({
  blocks,
  onSubmit,
  onSkip,
}: FocusBlocksStepProps) {
  const [selected, setSelected] = useState<Set<number>>(
    new Set(blocks.map((_, i) => i))
  );

  function toggle(index: number) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  }

  function handleSubmit() {
    const selectedBlocks: FocusBlock[] = blocks
      .filter((_, i) => selected.has(i))
      .map((b) => ({
        day: b.day,
        start: b.start,
        end: b.end,
        durationMinutes: b.durationMinutes,
        suggestedActivity: b.suggestedUse ?? "Focus time",
        selected: true,
      }));
    onSubmit(selectedBlocks);
  }

  if (blocks.length === 0) {
    return (
      <div className="space-y-6">
        <div>
          <h2 className="text-xl font-semibold">Focus blocks</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            No focus block suggestions this week
          </p>
        </div>
        <Button onClick={() => onSubmit([])}>Finish</Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Suggested focus blocks</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Toggle the ones you want to protect on your calendar
        </p>
      </div>

      <div className="space-y-3">
        {blocks.map((block, index) => {
          const isSelected = selected.has(index);
          return (
            <button
              key={index}
              onClick={() => toggle(index)}
              className={cn(
                "flex w-full items-center gap-4 rounded-lg border p-4 text-left transition-all",
                "hover:border-primary/50",
                isSelected &&
                  "border-primary bg-primary/5 ring-1 ring-primary/30"
              )}
            >
              <div
                className={cn(
                  "flex size-8 items-center justify-center rounded-full",
                  isSelected
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted"
                )}
              >
                <Target className="size-4" />
              </div>
              <div className="flex-1">
                <p className="font-medium">
                  {block.suggestedUse ?? "Focus time"}
                </p>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Clock className="size-3" />
                  <span>
                    {block.day} {block.start}–{block.end}
                  </span>
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex items-center gap-3">
        <Button onClick={handleSubmit}>
          Create {selected.size} block{selected.size !== 1 ? "s" : ""}
        </Button>
        <Button variant="ghost" onClick={onSkip}>
          Skip
        </Button>
      </div>
    </div>
  );
}

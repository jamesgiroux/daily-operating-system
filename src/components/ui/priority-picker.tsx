import { cn } from "@/lib/utils";

const priorities = ["P1", "P2", "P3"] as const;

const priorityStyles: Record<string, string> = {
  P1: "bg-destructive/15 text-destructive border-destructive/30",
  P2: "bg-primary/15 text-primary border-primary/30",
  P3: "bg-muted text-muted-foreground border-muted-foreground/30",
};

interface PriorityPickerProps {
  value: string;
  onChange: (priority: string) => void;
  className?: string;
}

export function PriorityPicker({
  value,
  onChange,
  className,
}: PriorityPickerProps) {
  return (
    <div className={cn("flex gap-1", className)}>
      {priorities.map((p) => (
        <button
          key={p}
          type="button"
          onClick={() => onChange(p)}
          className={cn(
            "rounded-md border px-2 py-0.5 text-xs font-medium transition-colors",
            value === p
              ? priorityStyles[p]
              : "border-transparent text-muted-foreground hover:text-foreground"
          )}
        >
          {p}
        </button>
      ))}
    </div>
  );
}

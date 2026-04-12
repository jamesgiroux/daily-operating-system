import { cn } from "@/lib/utils";

const priorities = [
  { value: 1, label: "Urgent" },
  { value: 2, label: "High" },
  { value: 3, label: "Medium" },
  { value: 4, label: "Low" },
] as const;

const priorityStyles: Record<number, string> = {
  1: "bg-destructive/15 text-destructive border-destructive/30",
  2: "bg-primary/15 text-primary border-primary/30",
  3: "bg-muted text-muted-foreground border-muted-foreground/30",
  4: "bg-muted text-muted-foreground border-muted-foreground/30",
};

interface PriorityPickerProps {
  value: number;
  onChange: (priority: number) => void;
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
          key={p.value}
          type="button"
          onClick={() => onChange(p.value)}
          className={cn(
            "rounded-md border px-2 py-0.5 text-xs font-medium transition-colors",
            value === p.value
              ? priorityStyles[p.value]
              : "border-transparent text-muted-foreground hover:text-foreground"
          )}
        >
          {p.label}
        </button>
      ))}
    </div>
  );
}

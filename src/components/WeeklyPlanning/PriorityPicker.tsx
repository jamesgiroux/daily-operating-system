import { useState } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Building2, Code2, FileText, Sun } from "lucide-react";

interface PriorityPickerProps {
  onSubmit: (priorities: string[]) => void;
  onSkip: () => void;
}

const priorities = [
  {
    id: "customer-meetings",
    label: "Customer Meetings",
    description: "External calls, QBRs, partnerships",
    icon: Building2,
    color: "text-primary",
  },
  {
    id: "project-work",
    label: "Project Work",
    description: "Deep work, deliverables, initiatives",
    icon: Code2,
    color: "text-success",
  },
  {
    id: "admin-catchup",
    label: "Admin Catch-up",
    description: "Inbox zero, overdue actions, planning",
    icon: FileText,
    color: "text-peach",
  },
  {
    id: "light-week",
    label: "Light Week",
    description: "Recovery, learning, strategic thinking",
    icon: Sun,
    color: "text-muted-foreground",
  },
];

export function PriorityPicker({ onSubmit, onSkip }: PriorityPickerProps) {
  const [selected, setSelected] = useState<Set<string>>(new Set());

  function toggle(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">What's your focus this week?</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Select one or more priorities to guide your week
        </p>
      </div>

      <div className="grid grid-cols-2 gap-4">
        {priorities.map((priority) => {
          const isSelected = selected.has(priority.id);
          return (
            <button
              key={priority.id}
              onClick={() => toggle(priority.id)}
              className={cn(
                "flex items-start gap-3 rounded-lg border p-4 text-left transition-all",
                "hover:border-primary/50 hover:bg-primary/5",
                isSelected &&
                  "border-primary bg-primary/10 ring-1 ring-primary/30"
              )}
            >
              <priority.icon
                className={cn("mt-0.5 size-5", priority.color)}
              />
              <div>
                <p className="font-medium">{priority.label}</p>
                <p className="text-xs text-muted-foreground">
                  {priority.description}
                </p>
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex items-center gap-3">
        <Button
          onClick={() => onSubmit(Array.from(selected))}
          disabled={selected.size === 0}
        >
          Continue
        </Button>
        <Button variant="ghost" onClick={onSkip}>
          Skip
        </Button>
      </div>
    </div>
  );
}

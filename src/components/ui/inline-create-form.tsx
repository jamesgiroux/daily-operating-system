import { Button } from "@/components/ui/button";

interface InlineCreateFormProps {
  value: string;
  onChange: (value: string) => void;
  onCreate: () => void;
  onCancel: () => void;
  placeholder?: string;
}

export function InlineCreateForm({
  value,
  onChange,
  onCreate,
  onCancel,
  placeholder = "Name",
}: InlineCreateFormProps) {
  return (
    <div className="flex items-center gap-2">
      <input
        type="text"
        autoFocus
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") onCreate();
          if (e.key === "Escape") onCancel();
        }}
        placeholder={placeholder}
        className="rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
      />
      <Button size="sm" onClick={onCreate}>
        Create
      </Button>
      <Button size="sm" variant="ghost" onClick={onCancel}>
        Cancel
      </Button>
    </div>
  );
}

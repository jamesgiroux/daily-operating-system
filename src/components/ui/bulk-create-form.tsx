import { Button } from "@/components/ui/button";

export function parseBulkCreateInput(value: string): string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export function shouldSubmitBulkCreateKey(key: string, metaKey: boolean, ctrlKey: boolean): boolean {
  return (metaKey || ctrlKey) && key === "Enter";
}

interface BulkCreateFormProps {
  value: string;
  onChange: (value: string) => void;
  onCreate: () => void;
  onSingleMode: () => void;
  onCancel: () => void;
  placeholder: string;
}

export function BulkCreateForm({
  value,
  onChange,
  onCreate,
  onSingleMode,
  onCancel,
  placeholder,
}: BulkCreateFormProps) {
  const count = parseBulkCreateInput(value).length;

  return (
    <div className="flex flex-col gap-2">
      <textarea
        autoFocus
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            onCancel();
          }
          if (shouldSubmitBulkCreateKey(e.key, e.metaKey, e.ctrlKey)) {
            onCreate();
          }
        }}
        placeholder={placeholder}
        rows={5}
        className="w-64 rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
      />
      <div className="flex items-center gap-2">
        <Button size="sm" onClick={onCreate}>
          Create {count || ""}
        </Button>
        <Button size="sm" variant="ghost" onClick={onSingleMode}>
          Single
        </Button>
        <Button size="sm" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

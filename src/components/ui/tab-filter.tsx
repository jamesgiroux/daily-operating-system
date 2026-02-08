import { cn } from "@/lib/utils";

interface Tab<T extends string> {
  key: T;
  label: string;
}

interface TabFilterProps<T extends string> {
  tabs: Tab<T>[];
  active: T;
  onChange: (key: T) => void;
  counts?: Partial<Record<T, number>>;
  className?: string;
}

export function TabFilter<T extends string>({
  tabs,
  active,
  onChange,
  counts,
  className,
}: TabFilterProps<T>) {
  return (
    <div className={cn("flex gap-2", className)}>
      {tabs.map((t) => {
        const count = counts?.[t.key] ?? 0;
        return (
          <button
            key={t.key}
            onClick={() => onChange(t.key)}
            className={cn(
              "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
              active === t.key
                ? "bg-primary text-primary-foreground"
                : "bg-muted hover:bg-muted/80"
            )}
          >
            {t.label}
            {count > 0 && (
              <span
                className={cn(
                  "ml-1.5 inline-flex size-5 items-center justify-center rounded-full text-xs",
                  active === t.key
                    ? "bg-primary-foreground/20 text-primary-foreground"
                    : "bg-muted-foreground/15 text-muted-foreground"
                )}
              >
                {count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

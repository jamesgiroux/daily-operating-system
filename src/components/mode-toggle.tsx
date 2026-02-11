import { Moon, Sun, Monitor } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTheme } from "@/components/theme-provider";

export function ModeToggle() {
  const { theme, setTheme } = useTheme();

  return (
    <div className="flex items-center gap-1 rounded-lg border p-1">
      <Button
        variant={theme === "light" ? "secondary" : "ghost"}
        size="icon"
        className="size-7"
        onClick={() => setTheme("light")}
        title="Light"
      >
        <Sun className="size-3.5" />
      </Button>
      <Button
        variant={theme === "dark" ? "secondary" : "ghost"}
        size="icon"
        className="size-7"
        onClick={() => setTheme("dark")}
        title="Dark"
      >
        <Moon className="size-3.5" />
      </Button>
      <Button
        variant={theme === "system" ? "secondary" : "ghost"}
        size="icon"
        className="size-7"
        onClick={() => setTheme("system")}
        title="System"
      >
        <Monitor className="size-3.5" />
      </Button>
    </div>
  );
}

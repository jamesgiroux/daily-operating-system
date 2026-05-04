import { Search } from "lucide-react";
import {
  useEffect,
  useId,
  useRef,
  useState,
  type ChangeEvent,
  type ComponentPropsWithoutRef,
  type FormEvent,
} from "react";
import clsx from "clsx";
import {
  THREAD_MARK_SEED_EVENT,
  type ThreadMarkSeedDetail,
} from "@/components/ui/ThreadMark";
import styles from "./AskAnythingDock.module.css";

export interface AskAnythingSuggestionChip {
  label: string;
  value?: string;
}

export interface AskAnythingContext {
  source: "thread-mark" | "suggestion" | "manual";
  context?: string;
  threadId?: string;
}

export interface AskAnythingDockProps
  extends Omit<ComponentPropsWithoutRef<"form">, "onSubmit" | "onChange"> {
  placement?: "inline" | "fixed-bottom";
  placeholderRotation?: string[];
  suggestionChips?: AskAnythingSuggestionChip[];
  scopeSources?: string[];
  scopeSince?: string;
  writesBack?: boolean;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  onContextChange?: (context: AskAnythingContext | null) => void;
  onSubmit?: (query: string, context: AskAnythingContext | null) => void;
  enableKeyboardShortcut?: boolean;
  inputLabel?: string;
}

const DEFAULT_PLACEHOLDERS = [
  "What did Sara say about tier 3 last quarter?",
  "Remind me to follow up with James on Monday...",
  "What's slipping on Acme this week?",
  "Show me Northwind's last 5 emails",
];

const DEFAULT_CHIPS: AskAnythingSuggestionChip[] = [
  { label: "What's slipping on Acme this week?" },
  { label: "Show me Northwind's last 5 emails" },
  { label: "Remind me about James Monday" },
  { label: "Who's quiet that I should check on?" },
];

const DEFAULT_SCOPE_SOURCES = ["Mail", "Calendar", "Notes", "CRM", "Slack"];

function normalizeSeed(context: string): string {
  const trimmed = context.trim().replace(/\s+/g, " ");
  return trimmed ? `About: "${trimmed}" — ` : "";
}

export function AskAnythingDock({
  placement = "inline",
  placeholderRotation = DEFAULT_PLACEHOLDERS,
  suggestionChips = DEFAULT_CHIPS,
  scopeSources = DEFAULT_SCOPE_SOURCES,
  scopeSince = "Jan 1, 2024",
  writesBack = true,
  value,
  defaultValue = "",
  onChange,
  onContextChange,
  onSubmit,
  enableKeyboardShortcut = true,
  inputLabel = "Ask anything",
  className,
  id,
  ...rest
}: AskAnythingDockProps) {
  const generatedId = useId();
  const inputId = id ? `${id}-input` : `${generatedId}-ask-input`;
  const inputRef = useRef<HTMLInputElement>(null);
  const [draft, setDraft] = useState(defaultValue);
  const [placeholderIndex, setPlaceholderIndex] = useState(0);
  const [activeContext, setActiveContext] = useState<AskAnythingContext | null>(null);
  const controlled = value != null;
  const inputValue = controlled ? value : draft;
  const placeholders = placeholderRotation.length > 0 ? placeholderRotation : DEFAULT_PLACEHOLDERS;

  function setInputValue(nextValue: string) {
    if (!controlled) {
      setDraft(nextValue);
    }
    onChange?.(nextValue);
  }

  function setContext(nextContext: AskAnythingContext | null) {
    setActiveContext(nextContext);
    onContextChange?.(nextContext);
  }

  function seedInput(nextValue: string, nextContext: AskAnythingContext | null) {
    setInputValue(nextValue);
    setContext(nextContext);
    window.setTimeout(() => inputRef.current?.focus(), 0);
  }

  useEffect(() => {
    if (placeholders.length <= 1) return;

    const interval = window.setInterval(() => {
      setPlaceholderIndex((current) => (current + 1) % placeholders.length);
    }, 4500);

    return () => window.clearInterval(interval);
  }, [placeholders.length]);

  useEffect(() => {
    function handleThreadSeed(event: Event) {
      const detail = (event as CustomEvent<ThreadMarkSeedDetail>).detail;
      if (!detail) return;
      seedInput(normalizeSeed(detail.context), {
        source: "thread-mark",
        context: detail.context,
        threadId: detail.threadId,
      });
    }

    window.addEventListener(THREAD_MARK_SEED_EVENT, handleThreadSeed);
    return () => window.removeEventListener(THREAD_MARK_SEED_EVENT, handleThreadSeed);
  });

  useEffect(() => {
    if (!enableKeyboardShortcut) return;

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key.toLowerCase() !== "k" || (!event.metaKey && !event.ctrlKey)) return;
      event.preventDefault();
      inputRef.current?.focus();
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [enableKeyboardShortcut]);

  function handleInputChange(event: ChangeEvent<HTMLInputElement>) {
    setInputValue(event.target.value);
    if (!event.target.value.trim()) {
      setContext(null);
    }
  }

  function handleChipClick(chip: AskAnythingSuggestionChip) {
    const nextValue = chip.value ?? chip.label;
    seedInput(nextValue, { source: "suggestion", context: nextValue });
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const query = inputValue.trim();
    if (!query) return;
    onSubmit?.(query, activeContext ?? { source: "manual" });
  }

  return (
    <form
      className={clsx(
        styles.dock,
        placement === "fixed-bottom" ? styles.fixedBottom : styles.inline,
        className,
      )}
      id={id}
      onSubmit={handleSubmit}
      data-ds-name="AskAnythingDock"
      data-ds-spec="patterns/AskAnythingDock.md"
      data-placement={placement}
      {...rest}
    >
      <div className={styles.inputRow}>
        <Search className={styles.glyph} strokeWidth={1.5} aria-hidden="true" />
        <label htmlFor={inputId} className="sr-only">
          {inputLabel}
        </label>
        <input
          ref={inputRef}
          id={inputId}
          className={styles.input}
          type="text"
          value={inputValue}
          placeholder={placeholders[placeholderIndex]}
          onChange={handleInputChange}
          autoComplete="off"
        />
        <span className={styles.kbd} aria-hidden="true">
          ⌘ K
        </span>
      </div>

      {suggestionChips.length > 0 ? (
        <div className={styles.suggestions} aria-label="Suggested questions">
          {suggestionChips.map((chip) => (
            <button
              type="button"
              className={styles.chip}
              key={chip.label}
              onClick={() => handleChipClick(chip)}
            >
              {chip.label}
            </button>
          ))}
        </div>
      ) : null}

      <div className={styles.scope}>
        <div className={styles.scopeLeft}>
          <span className={styles.scopeSources}>
            <span className={styles.scopeDot} aria-hidden="true" />
            {scopeSources.join(" · ")}
          </span>
          {scopeSince ? (
            <>
              <span className={styles.scopeDivider} aria-hidden="true">
                ·
              </span>
              <span>Since {scopeSince}</span>
            </>
          ) : null}
        </div>
        {writesBack ? <span className={styles.scopeWrite}>Writes back to your briefing</span> : null}
      </div>
    </form>
  );
}

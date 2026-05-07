/**
 * InferredActionSelector.tsx — Daily Briefing Watch action picker
 *
 * Compact selector for a recommended watch-row action plus alternatives.
 *
 * Spec: .docs/design/patterns/InferredActionSelector.md
 * Contract: src/types/briefing.ts → InferredActionSelectorViewModel
 */

import {
  Fragment,
  useEffect,
  useId,
  useRef,
  useState,
  type ComponentPropsWithoutRef,
  type MouseEvent,
} from "react";
import clsx from "clsx";
import type {
  InferredActionOption,
  InferredActionSelectorViewModel,
} from "@/types/briefing";
import styles from "./InferredActionSelector.module.css";

export interface InferredActionSelectorProps
  extends Omit<ComponentPropsWithoutRef<"span">, "onSelect"> {
  selector: InferredActionSelectorViewModel;
  onSelect: (id: string) => void;
}

function findSelectedOption(
  selector: InferredActionSelectorViewModel,
): InferredActionOption | undefined {
  return selector.options.find((option) => option.id === selector.selectedOptionId);
}

export function InferredActionSelector({
  selector,
  onSelect,
  className,
  ...rest
}: InferredActionSelectorProps): JSX.Element {
  const menuId = useId();
  const rootRef = useRef<HTMLSpanElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const selectedOption = findSelectedOption(selector);
  const triggerLabel = selectedOption?.label ?? selector.triggerLabel;

  useEffect(() => {
    if (!isOpen) return;

    function handleDocumentMouseDown(event: globalThis.MouseEvent) {
      const root = rootRef.current;
      if (!root || !(event.target instanceof Node)) return;
      if (!root.contains(event.target)) {
        setIsOpen(false);
      }
    }

    function handleDocumentKeyDown(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    }

    document.addEventListener("mousedown", handleDocumentMouseDown);
    document.addEventListener("keydown", handleDocumentKeyDown);

    return () => {
      document.removeEventListener("mousedown", handleDocumentMouseDown);
      document.removeEventListener("keydown", handleDocumentKeyDown);
    };
  }, [isOpen]);

  function handleTriggerClick(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation();
    setIsOpen((current) => !current);
  }

  function handleOptionClick(
    event: MouseEvent<HTMLButtonElement>,
    optionId: string,
  ) {
    event.stopPropagation();
    onSelect(optionId);
    setIsOpen(false);
  }

  return (
    <span
      ref={rootRef}
      className={clsx(styles.root, className)}
      data-open={isOpen ? "true" : "false"}
      data-ds-tier="pattern"
      data-ds-name="InferredActionSelector"
      data-ds-spec="patterns/InferredActionSelector.md"
      {...rest}
    >
      <button
        type="button"
        className={styles.trigger}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-controls={isOpen ? menuId : undefined}
        onClick={handleTriggerClick}
      >
        <span className={styles.label}>{triggerLabel}</span>
        <span className={styles.chevron} aria-hidden="true" />
      </button>

      {isOpen ? (
        <span id={menuId} className={styles.menu} role="menu">
          {selector.options.map((option) => {
            const isSelected = option.id === selector.selectedOptionId;

            return (
              <Fragment key={option.id}>
                {option.divider ? (
                  <span
                    className={styles.divider}
                    role="separator"
                    aria-orientation="horizontal"
                  />
                ) : null}
                <button
                  type="button"
                  role="menuitem"
                  aria-label={
                    option.confidence
                      ? `${option.label} ${option.confidence.label}`
                      : undefined
                  }
                  className={clsx(
                    styles.option,
                    isSelected && styles.optionSelected,
                  )}
                  data-option-id={option.id}
                  data-selected={isSelected ? "true" : undefined}
                  onClick={(event) => handleOptionClick(event, option.id)}
                >
                  <span className={styles.optionLabel}>{option.label}</span>
                  {option.confidence ? (
                    <span
                      className={styles.optionMeta}
                      data-confidence-value={option.confidence.value}
                    >
                      {option.confidence.label}
                    </span>
                  ) : null}
                </button>
              </Fragment>
            );
          })}
        </span>
      ) : null}
    </span>
  );
}

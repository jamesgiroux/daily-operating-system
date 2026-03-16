/**
 * TeamRoleSelector — Clickable badge that opens a dropdown to change
 * an internal team member's role. Follows EngagementSelector pattern.
 *
 * Uses a portal to render the dropdown at document.body level,
 * avoiding z-index stacking context issues from parent grid layouts.
 *
 * These are internal team roles only. Champion/executive/technical are
 * stakeholder engagement levels handled by EngagementSelector.
 */
import { useState, useRef, useEffect, useCallback } from "react";
import { createPortal } from "react-dom";
import css from "./TeamRoleSelector.module.css";

interface TeamRoleSelectorProps {
  value: string;
  onChange: (value: string) => void;
}

interface TeamRoleOption {
  stored: string;
  label: string;
}

const TEAM_ROLE_OPTIONS: TeamRoleOption[] = [
  { stored: "ae", label: "AE" },
  { stored: "csm", label: "CSM" },
  { stored: "tam", label: "TAM" },
  { stored: "rm", label: "RM" },
  { stored: "ao", label: "AO" },
  { stored: "se", label: "SE" },
  { stored: "executive_sponsor", label: "Exec Sponsor" },
  { stored: "implementation", label: "Implementation" },
  { stored: "associated", label: "Associated" },
];

/** Map a stored team role to its display label. */
export function getTeamRoleDisplay(stored: string): string {
  const lower = stored.toLowerCase();
  const found = TEAM_ROLE_OPTIONS.find((o) => o.stored === lower);
  return found ? found.label : stored;
}

export function TeamRoleSelector({ value, onChange }: TeamRoleSelectorProps) {
  const [open, setOpen] = useState(false);
  const badgeRef = useRef<HTMLButtonElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0 });
  const label = getTeamRoleDisplay(value);

  const updatePosition = useCallback(() => {
    if (!badgeRef.current) return;
    const rect = badgeRef.current.getBoundingClientRect();
    setPos({
      top: rect.bottom + 4,
      left: rect.left,
    });
  }, []);

  useEffect(() => {
    if (!open) return;
    updatePosition();

    function handleClickOutside(e: MouseEvent) {
      const target = e.target as Node;
      if (
        badgeRef.current && !badgeRef.current.contains(target) &&
        dropdownRef.current && !dropdownRef.current.contains(target)
      ) {
        setOpen(false);
      }
    }

    function handleScroll() {
      updatePosition();
    }

    document.addEventListener("mousedown", handleClickOutside);
    window.addEventListener("scroll", handleScroll, true);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      window.removeEventListener("scroll", handleScroll, true);
    };
  }, [open, updatePosition]);

  return (
    <>
      <button
        ref={badgeRef}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setOpen(!open);
        }}
        className={css.badge}
      >
        {label}
      </button>

      {open && createPortal(
        <div
          ref={dropdownRef}
          className={css.dropdown}
          style={{ top: pos.top, left: pos.left }}
        >
          {TEAM_ROLE_OPTIONS.map((opt) => {
            const isActive = opt.stored === value.toLowerCase();
            return (
              <button
                key={opt.stored}
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  onChange(opt.stored);
                  setOpen(false);
                }}
                className={isActive ? css.optionActive : css.option}
              >
                <span className={css.optionLabel}>
                  {opt.label}
                </span>
              </button>
            );
          })}
        </div>,
        document.body,
      )}
    </>
  );
}

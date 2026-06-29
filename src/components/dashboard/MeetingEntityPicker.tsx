import { Fragment, useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { ChevronDown } from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import styles from "./MeetingEntityPicker.module.css";

interface EntityOption {
  id: string;
  name: string;
  type: "account" | "project";
  accountType?: "customer" | "internal" | "partner";
  parentName?: string;
}

export interface MeetingEntityPickerProps {
  meetingId: string;
  /** Current entity label shown in the eyebrow (the part before " · kind"). */
  label: string;
  /** Passed through to `add_meeting_entity`; the stored meeting type is reused
   *  verbatim so relinking never rewrites the meeting's classification. */
  meetingTitle: string;
  startTime: string;
  meetingType: string;
  /** Called after a successful relink so the briefing can refetch. */
  onChanged: () => void;
}

/**
 * Editorial in-spine selector for a meeting's primary entity. Reuses the same
 * candidate readers (`get_accounts_for_picker` / `get_projects_list`) and
 * relink command (`add_meeting_entity`) as the meeting-detail entity chips, but
 * presents in the magazine spine idiom rather than the shadcn app chrome.
 */
export function MeetingEntityPicker({
  meetingId,
  label,
  meetingTitle,
  startTime,
  meetingType,
  onChanged,
}: MeetingEntityPickerProps) {
  const [open, setOpen] = useState(false);
  const [entities, setEntities] = useState<EntityOption[]>([]);
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    let active = true;
    void (async () => {
      const items: EntityOption[] = [];
      try {
        const accounts = await invoke<
          {
            id: string;
            name: string;
            parentName?: string;
            accountType: "customer" | "internal" | "partner";
          }[]
        >("get_accounts_for_picker");
        items.push(
          ...accounts.map((a) => ({
            id: a.id,
            name: a.name,
            type: "account" as const,
            accountType: a.accountType,
            parentName: a.parentName ?? undefined,
          })),
        );
      } catch {
        // accounts unavailable — projects may still load
      }
      try {
        const projects = await invoke<{ id: string; name: string }[]>("get_projects_list");
        items.push(...projects.map((p) => ({ id: p.id, name: p.name, type: "project" as const })));
      } catch {
        // projects unavailable
      }
      if (active) setEntities(items);
    })();
    return () => {
      active = false;
    };
  }, [open]);

  const select = useCallback(
    async (option: EntityOption) => {
      setBusy(true);
      try {
        await invoke("add_meeting_entity", {
          meetingId,
          entityId: option.id,
          entityType: option.type,
          meetingTitle,
          startTime,
          meetingTypeStr: meetingType,
        });
        toast.success(`Linked to ${option.name}`);
        setOpen(false);
        setQuery("");
        onChanged();
      } catch (err) {
        console.error("relink meeting entity failed:", err);
        toast.error("Couldn't relink this meeting");
      } finally {
        setBusy(false);
      }
    },
    [meetingId, meetingTitle, startTime, meetingType, onChanged],
  );

  const needle = query.trim().toLowerCase();
  const matches = (entity: EntityOption) =>
    !needle || entity.name.toLowerCase().includes(needle);

  const internalAccounts = entities.filter(
    (e) => e.type === "account" && e.accountType === "internal" && matches(e),
  );
  const partnerAccounts = entities.filter(
    (e) => e.type === "account" && e.accountType === "partner" && matches(e),
  );
  const projects = entities.filter((e) => e.type === "project" && matches(e));

  // External accounts render parent → nested subsidiaries. A parent shows when
  // it matches the search directly or has a matching child.
  const externalGroups = entities
    .filter((e) => e.type === "account" && e.accountType === "customer" && !e.parentName)
    .map((parent) => ({
      parent,
      children: entities.filter(
        (e) =>
          e.type === "account" &&
          e.accountType === "customer" &&
          e.parentName === parent.name &&
          matches(e),
      ),
    }))
    .filter(({ parent, children }) => matches(parent) || children.length > 0);

  const isEmpty =
    internalAccounts.length === 0 &&
    partnerAccounts.length === 0 &&
    externalGroups.length === 0 &&
    projects.length === 0;

  const renderOption = (option: EntityOption, kindLabel: string, isChild = false) => (
    <li key={`${option.type}:${option.id}`}>
      <button
        type="button"
        className={isChild ? `${styles.option} ${styles.optionChild}` : styles.option}
        disabled={busy}
        onClick={() => void select(option)}
      >
        <span className={styles.optionName}>{option.name}</span>
        <span className={styles.optionKind}>{kindLabel}</span>
      </button>
    </li>
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        className={styles.trigger}
        aria-label={`Change the linked entity (currently ${label})`}
        onClick={(event) => event.stopPropagation()}
      >
        <span className={styles.label}>{label}</span>
        <ChevronDown className={styles.caret} aria-hidden="true" />
      </PopoverTrigger>
      <PopoverContent
        align="start"
        sideOffset={6}
        className="w-auto border-0 bg-transparent p-0 shadow-none"
        onOpenAutoFocus={(event) => event.preventDefault()}
      >
        <div className={styles.panel}>
          <input
            className={styles.search}
            placeholder="Link to account or project…"
            value={query}
            autoFocus
            onChange={(event) => setQuery(event.target.value)}
          />
          <ul className={styles.list}>
            {isEmpty ? (
              <li className={styles.empty}>No matches</li>
            ) : (
              <>
                {internalAccounts.length > 0 && (
                  <>
                    <li className={styles.groupHeading}>Internal Teams</li>
                    {internalAccounts.map((account) => renderOption(account, "Internal"))}
                  </>
                )}
                {externalGroups.length > 0 && (
                  <>
                    <li className={styles.groupHeading}>External Accounts</li>
                    {externalGroups.map(({ parent, children }) => (
                      <Fragment key={`group:${parent.id}`}>
                        {renderOption(parent, "Account")}
                        {children.map((child) => renderOption(child, "Subsidiary", true))}
                      </Fragment>
                    ))}
                  </>
                )}
                {partnerAccounts.length > 0 && (
                  <>
                    <li className={styles.groupHeading}>Partners</li>
                    {partnerAccounts.map((account) => renderOption(account, "Partner"))}
                  </>
                )}
                {projects.length > 0 && (
                  <>
                    <li className={styles.groupHeading}>Projects</li>
                    {projects.map((project) => renderOption(project, "Project"))}
                  </>
                )}
              </>
            )}
          </ul>
        </div>
      </PopoverContent>
    </Popover>
  );
}

/**
 * useTeamManagement — Team member search, add, remove, and inline creation.
 * Extracted from useAccountDetail to isolate team-management concerns.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Person } from "@/types";

function normalizeTeamRole(role: string): string {
  return role.trim() || "associated";
}

function syntheticUnknownEmail(name: string): string {
  const base = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ".")
    .replace(/^\.+|\.+$/g, "");
  const prefix = base.length > 0 ? base : "person";
  const uuid = crypto.randomUUID().slice(0, 8);
  return `${prefix}.${uuid}@unknown.local`;
}

export function useTeamManagement(
  accountId: string | undefined,
  reload: () => Promise<void>,
) {
  const [teamSearchQuery, setTeamSearchQuery] = useState("");
  const [teamSearchResults, setTeamSearchResults] = useState<Person[]>([]);
  const [selectedTeamPerson, setSelectedTeamPerson] = useState<Person | null>(null);
  const [teamRole, setTeamRole] = useState("CSM");
  const [teamWorking, setTeamWorking] = useState(false);
  const [teamInlineName, setTeamInlineName] = useState("");
  const [teamInlineEmail, setTeamInlineEmail] = useState("");
  const [teamInlineRole, setTeamInlineRole] = useState("Champion");
  const [resolvedImportNotes, setResolvedImportNotes] = useState<Set<number>>(new Set());
  const [teamError, setTeamError] = useState<string | null>(null);

  // Reset on account change
  useEffect(() => {
    setResolvedImportNotes(new Set());
    setTeamError(null);
  }, [accountId]);

  // Debounced team search
  useEffect(() => {
    if (!teamSearchQuery || teamSearchQuery.trim().length < 2) {
      setTeamSearchResults([]);
      return;
    }
    const timer = setTimeout(async () => {
      try {
        const results = await invoke<Person[]>("search_people", {
          query: teamSearchQuery.trim(),
        });
        setTeamSearchResults(results);
      } catch {
        setTeamSearchResults([]);
      }
    }, 180);
    return () => clearTimeout(timer);
  }, [teamSearchQuery]);

  const performTeamOperation = useCallback(
    async (operation: () => Promise<void>, onSuccess?: () => void) => {
      if (!accountId) return;
      try {
        setTeamWorking(true);
        setTeamError(null);
        await operation();
        onSuccess?.();
        await reload();
      } catch (e) {
        setTeamError(String(e));
      } finally {
        setTeamWorking(false);
      }
    },
    [accountId, reload],
  );

  const createAndAddTeamMember = useCallback(
    async (name: string, email: string, role: string) => {
      if (!accountId) return;
      const normalizedRole = normalizeTeamRole(role);
      const personName = name.trim();
      const personEmail = email.trim() || syntheticUnknownEmail(personName);
      const personId = await invoke<string>("create_person", {
        email: personEmail,
        name: personName,
        relationship: "unknown",
      });
      await invoke("add_account_team_member", {
        accountId,
        personId,
        role: normalizedRole,
      });
    },
    [accountId],
  );

  const handleAddExistingTeamMember = useCallback(async () => {
    if (!selectedTeamPerson) return;
    const normalizedRole = normalizeTeamRole(teamRole);
    await performTeamOperation(
      async () => {
        await invoke("add_account_team_member", {
          accountId,
          personId: selectedTeamPerson.id,
          role: normalizedRole,
        });
      },
      () => {
        setSelectedTeamPerson(null);
        setTeamSearchQuery("");
        setTeamSearchResults([]);
        setTeamRole("CSM");
      },
    );
  }, [accountId, selectedTeamPerson, teamRole, performTeamOperation]);

  const handleRemoveTeamMember = useCallback(
    async (personId: string, role: string) => {
      await performTeamOperation(async () => {
        await invoke("remove_account_team_member", {
          accountId,
          personId,
          role,
        });
      });
    },
    [accountId, performTeamOperation],
  );

  const handleCreateInlineTeamMember = useCallback(async () => {
    if (!teamInlineName.trim()) return;
    await performTeamOperation(
      async () => {
        await createAndAddTeamMember(teamInlineName, teamInlineEmail, teamInlineRole);
      },
      () => {
        setTeamInlineName("");
        setTeamInlineEmail("");
        setTeamInlineRole("Champion");
      },
    );
  }, [teamInlineName, teamInlineEmail, teamInlineRole, performTeamOperation, createAndAddTeamMember]);

  const handleImportNoteCreateAndAdd = useCallback(
    async (noteId: number, name: string, role: string) => {
      if (!name.trim()) return;
      await performTeamOperation(
        async () => {
          await createAndAddTeamMember(name.trim(), "", role);
        },
        () => {
          setResolvedImportNotes((prev) => new Set([...prev, noteId]));
        },
      );
    },
    [performTeamOperation, createAndAddTeamMember],
  );

  return {
    teamSearchQuery, setTeamSearchQuery,
    teamSearchResults,
    selectedTeamPerson, setSelectedTeamPerson,
    teamRole, setTeamRole,
    teamWorking,
    teamInlineName, setTeamInlineName,
    teamInlineEmail, setTeamInlineEmail,
    teamInlineRole, setTeamInlineRole,
    resolvedImportNotes,
    teamError,
    handleAddExistingTeamMember,
    handleRemoveTeamMember,
    handleCreateInlineTeamMember,
    handleImportNoteCreateAndAdd,
  };
}

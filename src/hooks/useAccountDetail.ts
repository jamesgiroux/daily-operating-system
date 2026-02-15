import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "@tanstack/react-router";
import type {
  AccountDetail,
  AccountEvent,
  ContentFile,
  Person,
  StrategicProgram,
} from "@/types";

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

export function useAccountDetail(accountId: string | undefined) {
  const navigate = useNavigate();
  const [detail, setDetail] = useState<AccountDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);

  // Editable structured fields
  const [editName, setEditName] = useState("");
  const [editHealth, setEditHealth] = useState("");
  const [editLifecycle, setEditLifecycle] = useState("");
  const [editArr, setEditArr] = useState("");
  const [editNps, setEditNps] = useState("");
  const [editRenewal, setEditRenewal] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [enriching, setEnriching] = useState(false);
  const [enrichSeconds, setEnrichSeconds] = useState(0);

  // Inline action creation
  const [addingAction, setAddingAction] = useState(false);
  const [newActionTitle, setNewActionTitle] = useState("");
  const [creatingAction, setCreatingAction] = useState(false);
  const [createChildOpen, setCreateChildOpen] = useState(false);
  const [childName, setChildName] = useState("");
  const [childDescription, setChildDescription] = useState("");
  const [childOwnerId, setChildOwnerId] = useState("");
  const [creatingChild, setCreatingChild] = useState(false);

  // Content index state
  const [files, setFiles] = useState<ContentFile[]>([]);
  const [indexing, setIndexing] = useState(false);
  const [newFileCount, setNewFileCount] = useState(0);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const [indexFeedback, setIndexFeedback] = useState<string | null>(null);

  // Strategic programs inline editing
  const [programs, setPrograms] = useState<StrategicProgram[]>([]);
  const programsSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Lifecycle events
  const [events, setEvents] = useState<AccountEvent[]>([]);
  const [showEventForm, setShowEventForm] = useState(false);
  const [newEventType, setNewEventType] = useState("renewal");
  const [newEventDate, setNewEventDate] = useState("");
  const [newArrImpact, setNewArrImpact] = useState("");
  const [newEventNotes, setNewEventNotes] = useState("");

  // Team management
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

  // Evidence section collapse state
  const [recentMeetingsExpanded, setRecentMeetingsExpanded] = useState(false);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (programsSaveTimer.current) clearTimeout(programsSaveTimer.current);
    };
  }, []);

  const intelligence = detail?.intelligence ?? null;

  const load = useCallback(async () => {
    if (!accountId) return;
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AccountDetail>("get_account_detail", {
        accountId,
      });
      setDetail(result);
      setEditName(result.name);
      setEditHealth(result.health ?? "");
      setEditLifecycle(result.lifecycle ?? "");
      setEditArr(result.arr?.toString() ?? "");
      setEditNps(result.nps?.toString() ?? "");
      setEditRenewal(result.renewalDate ?? "");
      setEditNotes(result.notes ?? "");
      setPrograms(result.strategicPrograms);
      setDirty(false);
      // Load content files
      try {
        const contentFiles = await invoke<ContentFile[]>("get_entity_files", {
          entityId: accountId,
        });
        setFiles(contentFiles);
      } catch {
        // Non-critical
      }
      // Load lifecycle events
      try {
        const accountEvents = await invoke<AccountEvent[]>("get_account_events", {
          accountId,
        });
        setEvents(accountEvents);
      } catch {
        // Non-critical
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [accountId]);

  useEffect(() => {
    load();
  }, [load]);

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

  // Listen for intelligence-updated events
  useEffect(() => {
    const unlisten = listen<{ entityId: string }>(
      "intelligence-updated",
      (event) => {
        if (accountId && event.payload.entityId === accountId) {
          load();
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [accountId, load]);

  // Listen for content-changed events from watcher
  useEffect(() => {
    const unlisten = listen<{ entityIds: string[]; count: number }>(
      "content-changed",
      (event) => {
        if (accountId && event.payload.entityIds.includes(accountId)) {
          setNewFileCount(event.payload.count);
          setBannerDismissed(false);
        }
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [accountId]);

  // Timer for enrichment progress
  useEffect(() => {
    if (!enriching) {
      setEnrichSeconds(0);
      return;
    }
    const interval = setInterval(() => {
      setEnrichSeconds((s) => s + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [enriching]);

  // ─── Handlers ───────────────────────────────────────────────────────

  async function handleSave() {
    if (!detail) return;
    setSaving(true);
    try {
      const fieldUpdates: [string, string][] = [];
      if (editName !== detail.name) fieldUpdates.push(["name", editName]);
      if (editHealth !== (detail.health ?? "")) fieldUpdates.push(["health", editHealth]);
      if (editLifecycle !== (detail.lifecycle ?? "")) fieldUpdates.push(["lifecycle", editLifecycle]);
      if (editArr !== (detail.arr?.toString() ?? "")) fieldUpdates.push(["arr", editArr]);
      if (editNps !== (detail.nps?.toString() ?? "")) fieldUpdates.push(["nps", editNps]);
      if (editRenewal !== (detail.renewalDate ?? "")) fieldUpdates.push(["contract_end", editRenewal]);

      for (const [field, value] of fieldUpdates) {
        await invoke("update_account_field", { accountId: detail.id, field, value });
      }
      if (editNotes !== (detail.notes ?? "")) {
        await invoke("update_account_notes", { accountId: detail.id, notes: editNotes });
      }
      setDirty(false);
      setEditing(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function handleCancelEdit() {
    if (!detail) return;
    setEditName(detail.name);
    setEditHealth(detail.health ?? "");
    setEditLifecycle(detail.lifecycle ?? "");
    setEditArr(detail.arr?.toString() ?? "");
    setEditNps(detail.nps?.toString() ?? "");
    setEditRenewal(detail.renewalDate ?? "");
    setDirty(false);
    setEditing(false);
  }

  async function handleIndexFiles() {
    if (!detail) return;
    setIndexing(true);
    setIndexFeedback(null);
    try {
      const updated = await invoke<ContentFile[]>("index_entity_files", {
        entityId: detail.id,
      });
      const diff = updated.length - files.length;
      setFiles(updated);
      setNewFileCount(0);
      setBannerDismissed(true);
      if (diff > 0) {
        setIndexFeedback(`${diff} new file${diff !== 1 ? "s" : ""} found`);
      } else {
        setIndexFeedback("Up to date");
      }
      setTimeout(() => setIndexFeedback(null), 3000);
    } catch (e) {
      setError(String(e));
    } finally {
      setIndexing(false);
    }
  }

  async function handleEnrich() {
    if (!detail) return;
    setEnriching(true);
    try {
      await invoke("enrich_account", { accountId: detail.id });
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setEnriching(false);
    }
  }

  async function handleCreateChild() {
    if (!detail || !childName.trim()) return;
    setCreatingChild(true);
    try {
      const result = await invoke<{ id: string }>("create_child_account", {
        parentId: detail.id,
        name: childName.trim(),
        description: childDescription.trim() || null,
        ownerPersonId: childOwnerId || null,
      });
      setCreateChildOpen(false);
      setChildName("");
      setChildDescription("");
      setChildOwnerId("");
      await load();
      navigate({ to: "/accounts/$accountId", params: { accountId: result.id } });
    } catch (e) {
      setError(String(e));
    } finally {
      setCreatingChild(false);
    }
  }

  // Debounced save for strategic programs
  const savePrograms = useCallback(
    async (updated: StrategicProgram[]) => {
      if (!detail) return;
      if (programsSaveTimer.current) clearTimeout(programsSaveTimer.current);
      programsSaveTimer.current = setTimeout(async () => {
        try {
          await invoke("update_account_programs", {
            accountId: detail.id,
            programsJson: JSON.stringify(updated),
          });
        } catch (e) {
          console.error("Failed to save programs:", e);
        }
      }, 400);
    },
    [detail]
  );

  function handleProgramUpdate(index: number, updated: StrategicProgram) {
    const next = [...programs];
    next[index] = updated;
    setPrograms(next);
    savePrograms(next);
  }

  function handleProgramDelete(index: number) {
    const next = programs.filter((_, i) => i !== index);
    setPrograms(next);
    savePrograms(next);
  }

  function handleAddProgram() {
    const next = [...programs, { name: "", status: "Active", notes: "" }];
    setPrograms(next);
  }

  async function handleRecordEvent() {
    if (!detail || !newEventDate) return;
    try {
      await invoke("record_account_event", {
        accountId: detail.id,
        eventType: newEventType,
        eventDate: newEventDate,
        arrImpact: newArrImpact ? parseFloat(newArrImpact) : null,
        notes: newEventNotes || null,
      });
      const updated = await invoke<AccountEvent[]>("get_account_events", {
        accountId: detail.id,
      });
      setEvents(updated);
      setShowEventForm(false);
      setNewEventType("renewal");
      setNewEventDate("");
      setNewArrImpact("");
      setNewEventNotes("");
    } catch (err) {
      console.error("Failed to record event:", err);
    }
  }

  async function handleArchive() {
    if (!detail) return;
    try {
      await invoke("archive_account", { id: detail.id, archived: true });
      navigate({ to: "/accounts" });
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnarchive() {
    if (!detail) return;
    try {
      await invoke("archive_account", { id: detail.id, archived: false });
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  async function performTeamOperation(
    operation: () => Promise<void>,
    onSuccess?: () => void
  ) {
    if (!detail) return;
    try {
      setTeamWorking(true);
      setTeamError(null);
      await operation();
      onSuccess?.();
      await load();
    } catch (e) {
      setTeamError(String(e));
    } finally {
      setTeamWorking(false);
    }
  }

  async function createAndAddTeamMember(name: string, email: string, role: string) {
    if (!detail) return;
    const normalizedRole = normalizeTeamRole(role);
    const personName = name.trim();
    const personEmail = email.trim() || syntheticUnknownEmail(personName);
    const personId = await invoke<string>("create_person", {
      email: personEmail,
      name: personName,
      relationship: "unknown",
    });
    await invoke("add_account_team_member", {
      accountId: detail.id,
      personId,
      role: normalizedRole,
    });
  }

  async function handleAddExistingTeamMember() {
    if (!selectedTeamPerson) return;
    const normalizedRole = normalizeTeamRole(teamRole);
    await performTeamOperation(
      async () => {
        await invoke("add_account_team_member", {
          accountId: detail!.id,
          personId: selectedTeamPerson.id,
          role: normalizedRole,
        });
      },
      () => {
        setSelectedTeamPerson(null);
        setTeamSearchQuery("");
        setTeamSearchResults([]);
        setTeamRole("CSM");
      }
    );
  }

  async function handleRemoveTeamMember(personId: string, role: string) {
    await performTeamOperation(async () => {
      await invoke("remove_account_team_member", {
        accountId: detail!.id,
        personId,
        role,
      });
    });
  }

  async function handleCreateInlineTeamMember() {
    if (!teamInlineName.trim()) return;
    await performTeamOperation(
      async () => {
        await createAndAddTeamMember(teamInlineName, teamInlineEmail, teamInlineRole);
      },
      () => {
        setTeamInlineName("");
        setTeamInlineEmail("");
        setTeamInlineRole("Champion");
      }
    );
  }

  async function handleImportNoteCreateAndAdd(noteId: number, name: string, role: string) {
    if (!name.trim()) return;
    await performTeamOperation(
      async () => {
        await createAndAddTeamMember(name.trim(), "", role);
      },
      () => {
        setResolvedImportNotes((prev) => new Set([...prev, noteId]));
      }
    );
  }

  async function handleCreateAction() {
    if (!detail || !newActionTitle.trim()) return;
    setCreatingAction(true);
    try {
      await invoke("create_action", {
        request: { title: newActionTitle.trim(), accountId: detail.id },
      });
      setNewActionTitle("");
      setAddingAction(false);
      load();
    } finally {
      setCreatingAction(false);
    }
  }

  return {
    // Core data
    detail,
    loading,
    error,
    intelligence,
    files,
    events,
    programs,

    // Edit fields
    editing, setEditing,
    editName, setEditName,
    editHealth, setEditHealth,
    editLifecycle, setEditLifecycle,
    editArr, setEditArr,
    editNps, setEditNps,
    editRenewal, setEditRenewal,
    editNotes, setEditNotes,
    dirty, setDirty,
    saving,
    enriching,
    enrichSeconds,

    // Action creation
    addingAction, setAddingAction,
    newActionTitle, setNewActionTitle,
    creatingAction,

    // Child account creation
    createChildOpen, setCreateChildOpen,
    childName, setChildName,
    childDescription, setChildDescription,
    childOwnerId, setChildOwnerId,
    creatingChild,

    // File indexing
    indexing,
    newFileCount,
    bannerDismissed, setBannerDismissed,
    indexFeedback,

    // Lifecycle events
    showEventForm, setShowEventForm,
    newEventType, setNewEventType,
    newEventDate, setNewEventDate,
    newArrImpact, setNewArrImpact,
    newEventNotes, setNewEventNotes,

    // Team management
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

    // Evidence collapse
    recentMeetingsExpanded, setRecentMeetingsExpanded,

    // Handlers
    load,
    handleSave,
    handleCancelEdit,
    handleIndexFiles,
    handleEnrich,
    handleCreateChild,
    handleProgramUpdate,
    handleProgramDelete,
    handleAddProgram,
    handleRecordEvent,
    handleArchive,
    handleUnarchive,
    handleAddExistingTeamMember,
    handleRemoveTeamMember,
    handleCreateInlineTeamMember,
    handleImportNoteCreateAndAdd,
    handleCreateAction,
  };
}

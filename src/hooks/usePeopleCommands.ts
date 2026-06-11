import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DuplicateCandidate, PersonListItem } from "@/types";

export function usePeopleCommands() {
  const getDuplicatePeople = useCallback(() => {
    return invoke<DuplicateCandidate[]>("get_duplicate_people");
  }, []);

  const createPerson = useCallback((email: string, name: string) => {
    return invoke<string>("create_person", { email, name });
  }, []);

  const getPeople = useCallback(() => {
    return invoke<PersonListItem[]>("get_people", { relationship: null });
  }, []);

  const getArchivedPeople = useCallback(() => {
    return invoke<PersonListItem[]>("get_archived_people");
  }, []);

  const mergePeople = useCallback((keepId: string, removeId: string) => {
    return invoke("merge_people", { keepId, removeId });
  }, []);

  return useMemo(() => ({
    createPerson,
    getArchivedPeople,
    getDuplicatePeople,
    getPeople,
    mergePeople,
  }), [createPerson, getArchivedPeople, getDuplicatePeople, getPeople, mergePeople]);
}

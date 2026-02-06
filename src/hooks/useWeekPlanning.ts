import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { WeekPlanningState, FocusBlock, WeekOverview } from "@/types";

export function useWeekPlanning() {
  const [planningState, setPlanningState] =
    useState<WeekPlanningState>("notready");
  const [wizardVisible, setWizardVisible] = useState(false);
  const [step, setStep] = useState(0);
  const [weekData, setWeekData] = useState<WeekOverview | null>(null);

  // Check planning state on mount
  useEffect(() => {
    invoke<WeekPlanningState>("get_week_planning_state")
      .then((state) => {
        setPlanningState(state);
        if (state === "dataready") {
          loadWeekData();
          const now = new Date();
          if (now.getDay() === 1) {
            setWizardVisible(true);
          }
        }
      })
      .catch(() => {});
  }, []);

  // Listen for week-data-ready event
  useEffect(() => {
    const unlisten = listen("week-data-ready", () => {
      setPlanningState("dataready");
      const now = new Date();
      if (now.getDay() === 1) {
        setWizardVisible(true);
      }
      // Load week data
      loadWeekData();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const loadWeekData = useCallback(async () => {
    try {
      const result = await invoke<{ status: string; data?: WeekOverview }>(
        "get_week_prep_data"
      );
      if (result.status === "success" && result.data) {
        setWeekData(result.data);
      }
    } catch {
      // Week data not available
    }
  }, []);

  const submitPriorities = useCallback(async (priorities: string[]) => {
    try {
      await invoke("submit_week_priorities", { priorities });
      setStep(1);
    } catch (err) {
      console.error("Failed to submit priorities:", err);
    }
  }, []);

  const submitFocusBlocks = useCallback(async (blocks: FocusBlock[]) => {
    try {
      await invoke("submit_focus_blocks", { blocks });
      setPlanningState("completed");
      setWizardVisible(false);
    } catch (err) {
      console.error("Failed to submit focus blocks:", err);
    }
  }, []);

  const skipAll = useCallback(async () => {
    try {
      await invoke("skip_week_planning");
      setPlanningState("defaultsapplied");
      setWizardVisible(false);
    } catch (err) {
      console.error("Failed to skip planning:", err);
    }
  }, []);

  const doLater = useCallback(() => {
    setWizardVisible(false);
  }, []);

  return {
    planningState,
    wizardVisible,
    step,
    setStep,
    weekData,
    loadWeekData,
    submitPriorities,
    submitFocusBlocks,
    skipAll,
    doLater,
  };
}

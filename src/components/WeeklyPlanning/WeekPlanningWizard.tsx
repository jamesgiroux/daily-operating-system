import { useWeekPlanning } from "@/hooks/useWeekPlanning";
import { PriorityPicker } from "./PriorityPicker";
import { WeekOverviewStep } from "./WeekOverviewStep";
import { FocusBlocksStep } from "./FocusBlocksStep";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { X } from "lucide-react";

export function WeekPlanningWizard() {
  const {
    wizardVisible,
    step,
    setStep,
    weekData,
    submitPriorities,
    submitFocusBlocks,
    skipAll,
    doLater,
  } = useWeekPlanning();

  if (!wizardVisible || !weekData) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/95 backdrop-blur-sm">
      <div className="w-full max-w-2xl px-6">
        {/* Header */}
        <div className="mb-8 flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">
              Plan your week
            </h1>
            <p className="text-sm text-muted-foreground">
              Week {weekData.weekNumber} &middot; {weekData.dateRange}
            </p>
          </div>
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="sm" onClick={doLater}>
              Do this later
            </Button>
            <Button variant="ghost" size="icon" onClick={skipAll}>
              <X className="size-4" />
            </Button>
          </div>
        </div>

        {/* Step indicators */}
        <div className="mb-8 flex items-center gap-2">
          {[0, 1, 2].map((s) => (
            <div
              key={s}
              className={cn(
                "h-1 flex-1 rounded-full transition-colors",
                s <= step ? "bg-primary" : "bg-muted"
              )}
            />
          ))}
        </div>

        {/* Step content */}
        {step === 0 && (
          <PriorityPicker
            onSubmit={(priorities) => submitPriorities(priorities)}
            onSkip={() => setStep(1)}
          />
        )}
        {step === 1 && (
          <WeekOverviewStep
            weekData={weekData}
            onContinue={() => setStep(2)}
            onSkip={() => setStep(2)}
          />
        )}
        {step === 2 && (
          <FocusBlocksStep
            blocks={weekData.availableTimeBlocks ?? []}
            onSubmit={(blocks) => submitFocusBlocks(blocks)}
            onSkip={skipAll}
          />
        )}
      </div>
    </div>
  );
}

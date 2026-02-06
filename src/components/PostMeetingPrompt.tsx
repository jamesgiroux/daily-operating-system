import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  Trophy,
  AlertTriangle,
  ListTodo,
  X,
  ChevronRight,
  Check,
  MessageSquare,
} from "lucide-react";
import { usePostMeetingCapture } from "@/hooks/usePostMeetingCapture";
import type { CapturedOutcome, CapturedAction } from "@/types";

type CaptureType = "win" | "risk" | "action";

interface CaptureItem {
  type: CaptureType;
  content: string;
  owner?: string;
  dueDate?: string;
}

export function PostMeetingPrompt() {
  const { visible, meeting, isFallback, capture, skip, dismiss } =
    usePostMeetingCapture();
  const [phase, setPhase] = useState<
    "prompt" | "input" | "confirm"
  >("prompt");
  const [activeType, setActiveType] = useState<CaptureType>("win");
  const [inputValue, setInputValue] = useState("");
  const [items, setItems] = useState<CaptureItem[]>([]);
  const [autoDismissProgress, setAutoDismissProgress] = useState(100);
  const [interacted, setInteracted] = useState(false);

  // Auto-dismiss after 60 seconds if no interaction
  useEffect(() => {
    if (!visible || phase !== "prompt" || interacted) return;

    const start = Date.now();
    const duration = 60_000;
    const interval = setInterval(() => {
      const elapsed = Date.now() - start;
      const remaining = Math.max(0, 100 - (elapsed / duration) * 100);
      setAutoDismissProgress(remaining);
      if (remaining <= 0) {
        dismiss();
      }
    }, 200);

    return () => clearInterval(interval);
  }, [visible, phase, interacted, dismiss]);

  // Reset state when a new meeting prompt appears
  useEffect(() => {
    if (visible) {
      setPhase("prompt");
      setItems([]);
      setInputValue("");
      setAutoDismissProgress(100);
      setInteracted(false);
    }
  }, [visible, meeting?.id]);

  const handleTypeSelect = useCallback((type: CaptureType) => {
    setActiveType(type);
    setPhase("input");
    setInputValue("");
    setInteracted(true);
  }, []);

  const handleAddItem = useCallback(() => {
    if (!inputValue.trim()) return;
    setItems((prev) => [
      ...prev,
      { type: activeType, content: inputValue.trim() },
    ]);
    setInputValue("");
    setPhase("confirm");
  }, [inputValue, activeType]);

  const handleFallbackSave = useCallback(async () => {
    if (!meeting || !inputValue.trim()) return;

    const outcome: CapturedOutcome = {
      meetingId: meeting.id,
      meetingTitle: meeting.title,
      account: meeting.account,
      capturedAt: new Date().toISOString(),
      wins: [inputValue.trim()],
      risks: [],
      actions: [],
    };

    await capture(outcome);
  }, [meeting, inputValue, capture]);

  const handleDone = useCallback(async () => {
    if (!meeting) return;

    const outcome: CapturedOutcome = {
      meetingId: meeting.id,
      meetingTitle: meeting.title,
      account: meeting.account,
      capturedAt: new Date().toISOString(),
      wins: items.filter((i) => i.type === "win").map((i) => i.content),
      risks: items.filter((i) => i.type === "risk").map((i) => i.content),
      actions: items
        .filter((i) => i.type === "action")
        .map(
          (i): CapturedAction => ({
            title: i.content,
            owner: i.owner,
            dueDate: i.dueDate,
          })
        ),
    };

    await capture(outcome);
  }, [meeting, items, capture]);

  if (!visible || !meeting) return null;

  // Fallback variant: simplified text-only prompt
  if (isFallback) {
    return (
      <div className="fixed bottom-4 right-4 z-50 w-80 animate-slide-in-right">
        <Card className="border-muted-foreground/20 shadow-lg">
          <CardContent className="p-4">
            <div className="mb-3 flex items-center justify-between">
              <p className="text-xs font-medium text-muted-foreground">
                Meeting ended
              </p>
              <Button
                variant="ghost"
                size="icon"
                className="size-6"
                onClick={skip}
              >
                <X className="size-3" />
              </Button>
            </div>

            <p className="mb-1 text-sm font-medium leading-tight">
              {meeting.title}
            </p>
            <p className="mb-3 text-xs text-muted-foreground">
              Quick note? Or skip — we'll process the transcript if one arrives.
            </p>

            <input
              type="text"
              autoFocus
              className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary"
              placeholder="Quick note..."
              value={inputValue}
              onChange={(e) => {
                setInputValue(e.target.value);
                if (!interacted) setInteracted(true);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter" && inputValue.trim()) handleFallbackSave();
                if (e.key === "Escape") skip();
              }}
            />

            <div className="mt-2 flex gap-2">
              <Button
                size="sm"
                className="flex-1 text-xs"
                onClick={handleFallbackSave}
                disabled={!inputValue.trim()}
              >
                <MessageSquare className="mr-1 size-3" />
                Save
              </Button>
              <Button
                size="sm"
                variant="ghost"
                className="text-xs text-muted-foreground"
                onClick={skip}
              >
                Skip
              </Button>
            </div>

            {/* Auto-dismiss progress bar */}
            <div className="mt-2 h-0.5 w-full overflow-hidden rounded-full bg-muted">
              <div
                className="h-full bg-muted-foreground/30 transition-all duration-200"
                style={{ width: `${autoDismissProgress}%` }}
              />
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Full variant: Win / Risk / Action buttons
  return (
    <div className="fixed bottom-4 right-4 z-50 w-80 animate-slide-in-right">
      <Card className="border-primary/20 shadow-lg">
        <CardContent className="p-4">
          {/* Header */}
          <div className="mb-3 flex items-center justify-between">
            <p className="text-xs font-medium text-muted-foreground">
              Meeting ended
            </p>
            <Button
              variant="ghost"
              size="icon"
              className="size-6"
              onClick={skip}
            >
              <X className="size-3" />
            </Button>
          </div>

          <p className="mb-3 text-sm font-medium leading-tight">
            {meeting.title}
          </p>

          {/* Prompt phase */}
          {phase === "prompt" && (
            <>
              <p className="mb-3 text-xs text-muted-foreground">
                Any quick outcomes?
              </p>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  className="flex-1 text-xs"
                  onClick={() => handleTypeSelect("win")}
                >
                  <Trophy className="mr-1 size-3 text-success" />
                  Win
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  className="flex-1 text-xs"
                  onClick={() => handleTypeSelect("risk")}
                >
                  <AlertTriangle className="mr-1 size-3 text-peach" />
                  Risk
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  className="flex-1 text-xs"
                  onClick={() => handleTypeSelect("action")}
                >
                  <ListTodo className="mr-1 size-3 text-primary" />
                  Action
                </Button>
              </div>
              <Button
                variant="ghost"
                size="sm"
                className="mt-2 w-full text-xs text-muted-foreground"
                onClick={skip}
              >
                Skip
              </Button>
              {/* Auto-dismiss progress bar */}
              <div className="mt-2 h-0.5 w-full overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-muted-foreground/30 transition-all duration-200"
                  style={{ width: `${autoDismissProgress}%` }}
                />
              </div>
            </>
          )}

          {/* Input phase */}
          {phase === "input" && (
            <>
              <div className="mb-2 flex items-center gap-1">
                {activeType === "win" && (
                  <Trophy className="size-3 text-success" />
                )}
                {activeType === "risk" && (
                  <AlertTriangle className="size-3 text-peach" />
                )}
                {activeType === "action" && (
                  <ListTodo className="size-3 text-primary" />
                )}
                <span className="text-xs font-medium capitalize">
                  {activeType}
                </span>
              </div>
              <input
                type="text"
                autoFocus
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-primary"
                placeholder={`Quick ${activeType} note...`}
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleAddItem();
                  if (e.key === "Escape") setPhase("prompt");
                }}
              />
              <div className="mt-2 flex gap-2">
                <Button
                  size="sm"
                  className="flex-1 text-xs"
                  onClick={handleAddItem}
                  disabled={!inputValue.trim()}
                >
                  Add
                  <ChevronRight className="ml-1 size-3" />
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="text-xs"
                  onClick={() => setPhase("prompt")}
                >
                  Back
                </Button>
              </div>
            </>
          )}

          {/* Confirm phase */}
          {phase === "confirm" && (
            <>
              <div className="mb-3 space-y-1">
                {items.map((item, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 rounded bg-muted/50 px-2 py-1 text-xs"
                  >
                    {item.type === "win" && (
                      <Trophy className="size-3 text-success" />
                    )}
                    {item.type === "risk" && (
                      <AlertTriangle className="size-3 text-peach" />
                    )}
                    {item.type === "action" && (
                      <ListTodo className="size-3 text-primary" />
                    )}
                    <span className="flex-1 truncate">{item.content}</span>
                  </div>
                ))}
              </div>
              <p className="mb-2 text-xs text-muted-foreground">
                Captured. Add another?
              </p>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  className="flex-1 text-xs"
                  onClick={() => setPhase("prompt")}
                >
                  Add more
                </Button>
                <Button
                  size="sm"
                  className="flex-1 text-xs"
                  onClick={handleDone}
                >
                  <Check className="mr-1 size-3" />
                  Done
                </Button>
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

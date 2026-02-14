import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";
import type { EntityMode } from "@/types";

import { Welcome } from "./chapters/Welcome";
import { EntityMode as EntityModeChapter } from "./chapters/EntityMode";
import { Workspace } from "./chapters/Workspace";
import { GoogleConnect } from "./chapters/GoogleConnect";
import { ClaudeCode } from "./chapters/ClaudeCode";
import { AboutYou } from "./chapters/AboutYou";
import { PopulateWorkspace } from "./chapters/PopulateWorkspace";
import { InboxTraining } from "./chapters/InboxTraining";
import { DashboardTour } from "./chapters/DashboardTour";
import { MeetingDeepDive } from "./chapters/MeetingDeepDive";
import { InternalTeamSetup } from "./chapters/InternalTeamSetup";
import { PrimeBriefing } from "./chapters/PrimeBriefing";

interface OnboardingFlowProps {
  mode?: "full" | "internal";
  onComplete: () => void;
}

const FULL_CHAPTERS = [
  "welcome",
  "entity-mode",
  "workspace",
  "google",
  "claude-code",
  "about-you",
  "internal-team-setup",
  "populate",
  "inbox-training",
  "dashboard-tour",
  "meeting-deep-dive",
  "prime-briefing",
] as const;

const INTERNAL_ONLY_CHAPTERS = [
  "internal-team-setup",
  "prime-briefing",
] as const;

type Chapter = (typeof FULL_CHAPTERS)[number];

const CHAPTER_LABELS: Record<Chapter, string> = {
  "welcome": "Welcome",
  "entity-mode": "Work Style",
  "workspace": "Workspace",
  "google": "Google",
  "claude-code": "Claude",
  "about-you": "About You",
  "internal-team-setup": "Internal Team",
  "populate": "Your Work",
  "inbox-training": "Inbox",
  "dashboard-tour": "Dashboard",
  "meeting-deep-dive": "Meeting Prep",
  "prime-briefing": "Prime",
};

export function OnboardingFlow({ mode = "full", onComplete }: OnboardingFlowProps) {
  const chapterOrder = (mode === "internal" ? INTERNAL_ONLY_CHAPTERS : FULL_CHAPTERS) as readonly Chapter[];
  const [chapter, setChapter] = useState<Chapter>(chapterOrder[0]);
  const [entityMode, setEntityMode] = useState<EntityMode>("account");
  const [quickSetup, setQuickSetup] = useState(false);

  const chapterIndex = chapterOrder.indexOf(chapter);

  useEffect(() => {
    setChapter(chapterOrder[0]);
  }, [mode]); // eslint-disable-line react-hooks/exhaustive-deps

  function goToChapter(c: Chapter) {
    setChapter(c);
  }

  function handleSkipToQuickSetup() {
    setQuickSetup(true);
    setChapter("entity-mode");
  }

  // Determine width class based on chapter
  const widthClass = chapter === "dashboard-tour"
    ? "max-w-5xl"
    : chapter === "meeting-deep-dive"
      ? "max-w-2xl"
      : "max-w-lg";

  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-4">
      <div className={cn("w-full space-y-6 transition-all duration-300", widthClass)}>
        {/* Progress dots */}
        <div className="flex items-center justify-center gap-2">
          {chapterOrder.map((c, i) => (
            <div
              key={c}
              className={cn(
                "size-2 rounded-full transition-colors",
                i < chapterIndex
                  ? "bg-primary/40"
                  : i === chapterIndex
                    ? "bg-primary"
                    : "bg-muted",
              )}
              title={CHAPTER_LABELS[c]}
            />
          ))}
        </div>

        {/* Chapter content */}
        {chapter === "welcome" && (
          <Welcome onNext={() => goToChapter("entity-mode")} />
        )}

        {chapter === "entity-mode" && (
          <EntityModeChapter
            onNext={(mode) => {
              setEntityMode(mode);
              goToChapter("workspace");
            }}
          />
        )}

        {chapter === "workspace" && (
          <Workspace
            entityMode={entityMode}
            onNext={(_path) => goToChapter("google")}
          />
        )}

        {chapter === "google" && (
          <GoogleConnect
            onNext={() => goToChapter("claude-code")}
          />
        )}

        {chapter === "claude-code" && (
          <ClaudeCode
            onNext={(_installed) => goToChapter("about-you")}
          />
        )}

        {chapter === "about-you" && (
          <AboutYou
            onNext={() => goToChapter("internal-team-setup")}
          />
        )}

        {chapter === "internal-team-setup" && (
          <InternalTeamSetup
            onNext={() => {
              if (mode === "internal") {
                goToChapter("prime-briefing");
              } else {
                goToChapter("populate");
              }
            }}
          />
        )}

        {chapter === "populate" && (
          <PopulateWorkspace
            entityMode={entityMode}
            onNext={() => goToChapter("inbox-training")}
          />
        )}

        {chapter === "inbox-training" && (
          <InboxTraining
            onNext={(_state) => {
              if (quickSetup) {
                goToChapter("prime-briefing");
              } else {
                goToChapter("dashboard-tour");
              }
            }}
          />
        )}

        {chapter === "dashboard-tour" && (
          <DashboardTour
            onNext={() => goToChapter("meeting-deep-dive")}
            onSkipTour={() => goToChapter("prime-briefing")}
          />
        )}

        {chapter === "meeting-deep-dive" && (
          <MeetingDeepDive onNext={() => goToChapter("prime-briefing")} />
        )}

        {chapter === "prime-briefing" && (
          <PrimeBriefing onComplete={onComplete} />
        )}

        {/* Skip to Quick Setup — visible on chapters 1-4 when not already in quick setup */}
        {mode === "full" && !quickSetup && chapterIndex <= 3 && (
          <div className="text-center pt-2">
            <button
              className="text-xs text-muted-foreground hover:text-foreground transition-colors"
              onClick={handleSkipToQuickSetup}
            >
              Skip to Quick Setup
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

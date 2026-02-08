import { useState } from "react";
import { cn } from "@/lib/utils";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import type { EntityMode } from "@/types";

import { Welcome } from "./chapters/Welcome";
import { EntityMode as EntityModeChapter } from "./chapters/EntityMode";
import { Workspace } from "./chapters/Workspace";
import { GoogleConnect } from "./chapters/GoogleConnect";
import { ClaudeCode } from "./chapters/ClaudeCode";
import { AboutYou } from "./chapters/AboutYou";
import { PopulateWorkspace } from "./chapters/PopulateWorkspace";
import { InboxTraining, type InboxProcessingState } from "./chapters/InboxTraining";
import { DashboardTour } from "./chapters/DashboardTour";
import { MeetingDeepDive } from "./chapters/MeetingDeepDive";
import { Ready } from "./chapters/Ready";

interface OnboardingFlowProps {
  onComplete: () => void;
}

const CHAPTERS = [
  "welcome",
  "entity-mode",
  "workspace",
  "google",
  "claude-code",
  "about-you",
  "populate",
  "inbox-training",
  "dashboard-tour",
  "meeting-deep-dive",
  "ready",
] as const;

type Chapter = (typeof CHAPTERS)[number];

const CHAPTER_LABELS = [
  "Welcome",
  "Work Style",
  "Workspace",
  "Google",
  "Claude",
  "About You",
  "Your Work",
  "Inbox",
  "Dashboard",
  "Meeting Prep",
  "Ready",
];

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [chapter, setChapter] = useState<Chapter>("welcome");
  const [entityMode, setEntityMode] = useState<EntityMode>("account");
  const [workspacePath, setWorkspacePath] = useState("");
  const [quickSetup, setQuickSetup] = useState(false);
  const [claudeCodeInstalled, setClaudeCodeInstalled] = useState(false);
  const [inboxProcessing, setInboxProcessing] = useState<InboxProcessingState | undefined>();

  const { status: googleStatus } = useGoogleAuth();

  const chapterIndex = CHAPTERS.indexOf(chapter);

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
          {CHAPTERS.map((c, i) => (
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
              title={CHAPTER_LABELS[i]}
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
            onNext={(path) => {
              setWorkspacePath(path);
              goToChapter("google");
            }}
          />
        )}

        {chapter === "google" && (
          <GoogleConnect
            onNext={() => goToChapter("claude-code")}
          />
        )}

        {chapter === "claude-code" && (
          <ClaudeCode
            onNext={(installed) => {
              setClaudeCodeInstalled(installed);
              goToChapter("about-you");
            }}
          />
        )}

        {chapter === "about-you" && (
          <AboutYou
            onNext={() => goToChapter("populate")}
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
            onNext={(state) => {
              setInboxProcessing(state);
              if (quickSetup) {
                goToChapter("ready");
              } else {
                goToChapter("dashboard-tour");
              }
            }}
          />
        )}

        {chapter === "dashboard-tour" && (
          <DashboardTour
            onNext={() => goToChapter("meeting-deep-dive")}
            onSkipTour={() => goToChapter("ready")}
          />
        )}

        {chapter === "meeting-deep-dive" && (
          <MeetingDeepDive onNext={() => goToChapter("ready")} />
        )}

        {chapter === "ready" && (
          <Ready
            entityMode={entityMode}
            workspacePath={workspacePath}
            googleStatus={googleStatus}
            claudeCodeInstalled={claudeCodeInstalled}
            inboxProcessing={inboxProcessing}
            onComplete={onComplete}
          />
        )}

        {/* Skip to Quick Setup — visible on chapters 1-4 when not already in quick setup */}
        {!quickSetup && chapterIndex <= 3 && (
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

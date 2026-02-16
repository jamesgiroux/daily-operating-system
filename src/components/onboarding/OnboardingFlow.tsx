import { useState } from "react";
import type { EntityMode } from "@/types";

import { AtmosphereLayer } from "@/components/layout/AtmosphereLayer";
import { FolioBar } from "@/components/layout/FolioBar";
import { FloatingNavIsland, type ChapterItem } from "@/components/layout/FloatingNavIsland";
import {
  Sparkles,
  Layers,
  FolderOpen,
  Mail,
  Terminal,
  User,
  Package,
  Inbox,
  LayoutDashboard,
  CalendarCheck,
  Rocket,
} from "lucide-react";

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
import { PrimeBriefing } from "./chapters/PrimeBriefing";

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
  "prime-briefing",
] as const;

type Chapter = (typeof CHAPTERS)[number];

const CHAPTER_ICONS: Record<Chapter, React.ReactNode> = {
  "welcome": <Sparkles size={16} strokeWidth={1.8} />,
  "entity-mode": <Layers size={16} strokeWidth={1.8} />,
  "workspace": <FolderOpen size={16} strokeWidth={1.8} />,
  "google": <Mail size={16} strokeWidth={1.8} />,
  "claude-code": <Terminal size={16} strokeWidth={1.8} />,
  "about-you": <User size={16} strokeWidth={1.8} />,
  "populate": <Package size={16} strokeWidth={1.8} />,
  "inbox-training": <Inbox size={16} strokeWidth={1.8} />,
  "dashboard-tour": <LayoutDashboard size={16} strokeWidth={1.8} />,
  "meeting-deep-dive": <CalendarCheck size={16} strokeWidth={1.8} />,
  "prime-briefing": <Rocket size={16} strokeWidth={1.8} />,
};

const CHAPTER_LABELS: Record<Chapter, string> = {
  "welcome": "Welcome",
  "entity-mode": "Work Style",
  "workspace": "Workspace",
  "google": "Google",
  "claude-code": "Claude",
  "about-you": "About You",
  "populate": "Your Work",
  "inbox-training": "Inbox",
  "dashboard-tour": "Dashboard",
  "meeting-deep-dive": "Meeting Prep",
  "prime-briefing": "Prime",
};

export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [chapter, setChapter] = useState<Chapter>(CHAPTERS[0]);
  const [entityMode, setEntityMode] = useState<EntityMode>("account");
  const [workspacePath, setWorkspacePath] = useState("~/Documents/DailyOS");
  const [quickSetup, setQuickSetup] = useState(false);
  const [visitedChapters, setVisitedChapters] = useState<Set<Chapter>>(new Set([CHAPTERS[0]]));

  const chapterIndex = CHAPTERS.indexOf(chapter);

  function goToChapter(c: Chapter) {
    setChapter(c);
    setVisitedChapters((prev) => new Set([...prev, c]));
  }

  function handleSkipToQuickSetup() {
    setQuickSetup(true);
    setChapter("entity-mode");
  }

  // Build chapter items for FloatingNavIsland
  const navChapters: ChapterItem[] = CHAPTERS.map((c) => ({
    id: c,
    label: CHAPTER_LABELS[c],
    icon: CHAPTER_ICONS[c],
  }));

  // Determine max width based on chapter
  const maxWidth = chapter === "dashboard-tour" ? 1200 : 720;

  return (
    <div
      style={{
        minHeight: "100vh",
        background: "var(--color-paper-cream)",
        position: "relative",
      }}
    >
      <AtmosphereLayer color="turmeric" />
      <FolioBar publicationLabel="Setup" />

      {/* FloatingNavIsland in chapter mode */}
      <FloatingNavIsland
        mode="chapters"
        chapters={navChapters}
        activeChapterId={chapter}
        activeColor="turmeric"
        onChapterClick={(id) => {
          // Only allow navigating to previously visited chapters
          if (visitedChapters.has(id as Chapter)) {
            goToChapter(id as Chapter);
          }
        }}
      />

      {/* Content column */}
      <div
        style={{
          maxWidth,
          margin: "0 auto",
          paddingTop: 80,
          paddingBottom: 120,
          paddingLeft: "var(--page-padding-horizontal)",
          paddingRight: "var(--page-padding-horizontal)",
          position: "relative",
          zIndex: "var(--z-page-content)",
          transition: "max-width 0.3s ease",
        }}
      >
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
              setWorkspacePath(path.replace(/^\/Users\/[^/]+/, "~"));
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
            workspacePath={workspacePath}
            onNext={(_installed) => goToChapter("about-you")}
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
        {!quickSetup && chapterIndex <= 3 && (
          <div style={{ textAlign: "center", paddingTop: 24 }}>
            <button
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                letterSpacing: "0.04em",
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
                transition: "color 0.15s ease",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "var(--color-text-primary)")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--color-text-tertiary)")}
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

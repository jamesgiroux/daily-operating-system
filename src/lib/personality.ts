import type { Personality } from "@/hooks/usePersonality";

/**
 * Personality-driven copy for UI chrome.
 *
 * Each key maps to an empty-state or loading context.
 * Each tone provides a title + optional message variant.
 *
 * Usage:
 *   const copy = getPersonalityCopy("actions-empty", personality);
 *   <SectionEmpty title={copy.title} message={copy.message} />
 */

interface Copy {
  title: string;
  message?: string;
  /** What this surface does when populated — for EmptyState explanations */
  explanation?: string;
  /** What the user gains — italic benefit statement */
  benefit?: string;
}

type CopyKey =
  | "actions-empty"
  | "actions-completed-empty"
  | "actions-waiting-empty"
  | "dashboard-empty"
  | "week-empty"
  | "emails-empty"
  | "emails-clear"
  | "inbox-empty"
  | "people-empty"
  | "people-archived-empty"
  | "people-no-matches"
  | "projects-empty"
  | "projects-archived-empty"
  | "projects-no-matches"
  | "accounts-empty"
  | "history-empty";

const COPY: Record<CopyKey, Record<Personality, Copy>> = {
  "actions-empty": {
    professional: {
      title: "No actions to show",
      message: "You're all caught up.",
      explanation: "Actions surface from meetings, emails, and manual capture. They group by urgency so you always know what's next.",
      benefit: "Never lose track of a commitment again.",
    },
    friendly: {
      title: "All clear!",
      message: "Nothing on your plate right now. Enjoy the calm.",
      explanation: "Actions show up here from your meetings and emails. They'll be organized by when they're due.",
      benefit: "Your to-do list, maintained for you.",
    },
    playful: {
      title: "Clean slate",
      message: "Zero actions. This is what winning looks like.",
      explanation: "Meetings and emails generate actions automatically. They'll land here grouped by urgency.",
      benefit: "A to-do list that writes itself.",
    },
  },
  "actions-completed-empty": {
    professional: { title: "No actions to show", message: "No completed actions yet." },
    friendly: { title: "Nothing completed yet", message: "Your first checkmark is waiting." },
    playful: { title: "Fresh start", message: "The done pile is empty. Time to make it grow." },
  },
  "actions-waiting-empty": {
    professional: { title: "No actions to show", message: "Nothing waiting on others." },
    friendly: { title: "Nobody's blocking you", message: "All clear on the waiting front." },
    playful: { title: "No one to blame", message: "The 'waiting on others' column is gloriously empty." },
  },
  "dashboard-empty": {
    professional: {
      title: "Your briefing will appear here",
      explanation: "Each morning, DailyOS prepares a briefing with your schedule, priorities, and what needs attention.",
      benefit: "Open the app, read your day, get to work.",
    },
    friendly: {
      title: "Your morning briefing starts here",
      explanation: "DailyOS prepares your day while you sleep — schedule, priorities, and what needs your attention.",
      benefit: "Everything you need, ready when you are.",
    },
    playful: {
      title: "The briefing room awaits",
      explanation: "Your daily briefing generates overnight — schedule, priorities, actions, and meeting prep all in one place.",
      benefit: "Fifteen minutes to feel prepared for anything.",
    },
  },
  "week-empty": {
    professional: {
      title: "Your week at a glance",
      explanation: "The week view shows your upcoming schedule, meeting density, and focus blocks. Connect your calendar to populate it.",
      benefit: "See the shape of your week before it starts.",
    },
    friendly: {
      title: "Your week is a blank canvas",
      explanation: "Once your calendar is connected, this page shows your week — meetings, focus time, and how busy each day looks.",
      benefit: "Plan your energy, not just your time.",
    },
    playful: {
      title: "A week of possibilities",
      explanation: "Connect your calendar and this page will map out your week — meetings, breathing room, and all.",
      benefit: "Know what's coming before it arrives.",
    },
  },
  "emails-empty": {
    professional: {
      title: "No email data yet",
      message: "Emails are triaged as part of your morning briefing.",
      explanation: "Your morning briefing analyzes Gmail messages and surfaces what matters. Connect Gmail to get started.",
      benefit: "The important emails, without the noise.",
    },
    friendly: {
      title: "No emails yet",
      message: "Once your morning briefing runs, triaged emails will appear here.",
      explanation: "DailyOS reads your Gmail and highlights what needs attention — sorted by importance, not arrival time.",
      benefit: "Email triage, done for you.",
    },
    playful: {
      title: "Inbox zen",
      message: "No email intel yet. The morning briefing will sort that out.",
      explanation: "Connect Gmail and your morning briefing will triage everything — flagging what matters and filing the rest.",
      benefit: "Never manually sort email again.",
    },
  },
  "emails-clear": {
    professional: { title: "All clear", message: "Nothing needs your attention right now." },
    friendly: { title: "You're good!", message: "All emails handled. Nothing needs your attention." },
    playful: { title: "Inbox zero hero", message: "Everything's handled. You may now take a bow." },
  },
  "inbox-empty": {
    professional: { title: "Inbox is clear", message: "Drop meeting notes, transcripts, or documents here — DailyOS classifies, extracts actions, and routes them automatically." },
    friendly: { title: "Nothing waiting", message: "Drop files here and DailyOS will analyze them, match them to your accounts, and file them for you." },
    playful: { title: "Blissfully empty", message: "Drag something in — we'll figure out what it is, who it's about, and where it goes." },
  },
  "people-empty": {
    professional: {
      title: "No people discovered yet",
      message: "People are discovered automatically from your calendar. Connect Google in Settings to get started.",
      explanation: "People are discovered from your calendar and enriched over time. Each person builds a relationship profile that informs meeting briefings.",
      benefit: "Know who you're meeting before you walk in.",
    },
    friendly: {
      title: "No contacts yet",
      message: "People show up automatically from your calendar. Head to Settings to connect Google.",
      explanation: "Your calendar contacts appear here automatically. Over time, each person builds a profile that helps prepare your meetings.",
      benefit: "Relationship context at your fingertips.",
    },
    playful: {
      title: "It's quiet in here",
      message: "Connect your calendar and watch the people roll in.",
      explanation: "Connect Google and people from your calendar will populate here — each building a profile that makes your meeting prep smarter.",
      benefit: "Your personal CRM, built from your calendar.",
    },
  },
  "people-archived-empty": {
    professional: { title: "No archived people", message: "Archived people will appear here." },
    friendly: { title: "Archive is empty", message: "No one's been archived yet." },
    playful: { title: "The archive is lonely", message: "No one's been sent to the archives. Yet." },
  },
  "people-no-matches": {
    professional: { title: "No matches", message: "Try a different search or filter." },
    friendly: { title: "No matches found", message: "Try adjusting your search." },
    playful: { title: "Nope", message: "Nobody matches that. Try something different?" },
  },
  "projects-empty": {
    professional: {
      title: "No projects yet",
      message: "Create your first project to get started.",
      explanation: "Projects track deliverables, milestones, and the people involved. Meeting briefings reference project context automatically.",
      benefit: "Better meeting prep through project awareness.",
    },
    friendly: {
      title: "No projects yet",
      message: "Ready to organize? Create your first project.",
      explanation: "Add projects to track milestones and deliverables. DailyOS will weave project context into your meeting briefings.",
      benefit: "Your projects, connected to your calendar.",
    },
    playful: {
      title: "Blank canvas",
      message: "No projects yet. The world is your oyster.",
      explanation: "Create a project and DailyOS will link it to your meetings, people, and briefings automatically.",
      benefit: "Projects that brief themselves.",
    },
  },
  "projects-archived-empty": {
    professional: { title: "No archived projects", message: "Archived projects will appear here." },
    friendly: { title: "Archive is empty", message: "No projects have been archived yet." },
    playful: { title: "Nothing in the vault", message: "The project archive awaits its first resident." },
  },
  "projects-no-matches": {
    professional: { title: "No matches", message: "Try a different search or filter." },
    friendly: { title: "No matches found", message: "Try adjusting your search." },
    playful: { title: "No dice", message: "Nothing matches. Try a different spell?" },
  },
  "accounts-empty": {
    professional: {
      title: "No accounts yet",
      message: "Create your first account to get started.",
      explanation: "Accounts track customer relationships, health, and revenue. Meeting briefings pull account context automatically.",
      benefit: "Walk into every meeting knowing the account story.",
    },
    friendly: {
      title: "No accounts yet",
      message: "Ready to start tracking? Create your first account.",
      explanation: "Add accounts to track health, renewals, and key contacts. DailyOS will connect accounts to your meetings and briefings.",
      benefit: "Your book of business, always up to date.",
    },
    playful: {
      title: "Fresh territory",
      message: "No accounts yet. Time to populate your empire.",
      explanation: "Create an account and watch DailyOS link it to your meetings, people, and daily briefings.",
      benefit: "An account manager that never sleeps.",
    },
  },
  "history-empty": {
    professional: {
      title: "No processing history yet",
      message: "Files processed from the inbox will appear here.",
      explanation: "When you drop files into the inbox, DailyOS classifies, extracts, and routes them. This page shows what was processed and where it went.",
      benefit: "Full visibility into what was captured.",
    },
    friendly: {
      title: "No history yet",
      message: "Once you process inbox files, they'll show up here.",
      explanation: "Drop meeting notes, transcripts, or documents into the inbox. This page tracks everything that was processed.",
      benefit: "A paper trail for your knowledge capture.",
    },
    playful: {
      title: "History hasn't started",
      message: "Process some inbox files and you'll have a history to be proud of.",
      explanation: "The inbox turns raw files into structured intelligence. This page is the receipt.",
      benefit: "Proof that the system is working for you.",
    },
  },
};

export function getPersonalityCopy(
  key: CopyKey,
  personality: Personality = "professional",
): Copy {
  return COPY[key]?.[personality] ?? COPY[key]?.professional ?? { title: key };
}

export type { CopyKey };

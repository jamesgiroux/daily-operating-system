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
}

type CopyKey =
  | "actions-empty"
  | "actions-completed-empty"
  | "actions-waiting-empty"
  | "emails-empty"
  | "emails-clear"
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
    professional: { title: "No actions to show", message: "You're all caught up." },
    friendly: { title: "All clear!", message: "Nothing on your plate right now. Enjoy the calm." },
    playful: { title: "Clean slate", message: "Zero actions. This is what winning looks like." },
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
  "emails-empty": {
    professional: { title: "No email data yet", message: "Emails are triaged as part of your morning briefing." },
    friendly: { title: "No emails yet", message: "Once your morning briefing runs, triaged emails will appear here." },
    playful: { title: "Inbox zen", message: "No email intel yet. The morning briefing will sort that out." },
  },
  "emails-clear": {
    professional: { title: "All clear", message: "Nothing needs your attention right now." },
    friendly: { title: "You're good!", message: "All emails handled. Nothing needs your attention." },
    playful: { title: "Inbox zero hero", message: "Everything's handled. You may now take a bow." },
  },
  "people-empty": {
    professional: { title: "No people discovered yet", message: "People are discovered automatically from your calendar. Connect Google in Settings to get started." },
    friendly: { title: "No contacts yet", message: "People show up automatically from your calendar. Head to Settings to connect Google." },
    playful: { title: "It's quiet in here", message: "Connect your calendar and watch the people roll in." },
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
    professional: { title: "No projects yet", message: "Create your first project to get started." },
    friendly: { title: "No projects yet", message: "Ready to organize? Create your first project." },
    playful: { title: "Blank canvas", message: "No projects yet. The world is your oyster." },
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
    professional: { title: "No accounts yet", message: "Create your first account to get started." },
    friendly: { title: "No accounts yet", message: "Ready to start tracking? Create your first account." },
    playful: { title: "Fresh territory", message: "No accounts yet. Time to populate your empire." },
  },
  "history-empty": {
    professional: { title: "No processing history yet", message: "Files processed from the inbox will appear here." },
    friendly: { title: "No history yet", message: "Once you process inbox files, they'll show up here." },
    playful: { title: "History hasn't started", message: "Process some inbox files and you'll have a history to be proud of." },
  },
};

export function getPersonalityCopy(
  key: CopyKey,
  personality: Personality = "professional",
): Copy {
  return COPY[key]?.[personality] ?? COPY[key]?.professional ?? { title: key };
}

export type { CopyKey };

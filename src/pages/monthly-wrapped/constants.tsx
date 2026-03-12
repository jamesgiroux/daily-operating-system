/**
 * Constants and utility functions for the Monthly Wrapped page.
 */
import {
  BookOpen,
  BarChart2,
  Star,
  Layers,
  Eye,
  Sparkles,
  Target,
  Trophy,
  ArrowRight,
  Moon,
} from "lucide-react";
import type { WrappedMoment, MonthlyWrappedContent } from "./types";

// =============================================================================
// Slide registry
// =============================================================================

export const SLIDES = [
  { id: "splash",          label: "Cover",              icon: <BookOpen size={18} strokeWidth={1.8} /> },
  { id: "volume",          label: "You Showed Up",      icon: <BarChart2 size={18} strokeWidth={1.8} /> },
  { id: "top-account",      label: "Top Account",        icon: <Star size={18} strokeWidth={1.8} /> },
  { id: "moments",         label: "The Moments",        icon: <Layers size={18} strokeWidth={1.8} /> },
  { id: "hidden-pattern",  label: "You Missed This",    icon: <Eye size={18} strokeWidth={1.8} /> },
  { id: "personality",     label: "Your Type",          icon: <Sparkles size={18} strokeWidth={1.8} /> },
  { id: "priority",        label: "Priority Check",     icon: <Target size={18} strokeWidth={1.8} /> },
  { id: "top-win",         label: "Your Win",           icon: <Trophy size={18} strokeWidth={1.8} /> },
  { id: "carry-forward",   label: "Carry Forward",      icon: <ArrowRight size={18} strokeWidth={1.8} /> },
  { id: "close",           label: "See You Next Month", icon: <Moon size={18} strokeWidth={1.8} /> },
];

export const ANALYSIS_PHASES = [
  {
    key: "counting",
    label: "Counting up your month",
    detail: "Tallying conversations, updates, and relationship touches",
  },
  {
    key: "moments",
    label: "Finding your moments",
    detail: "Surfacing firsts, peaks, and memorable interactions",
  },
  {
    key: "pattern",
    label: "Reading the pattern",
    detail: "Looking for what you might have missed",
  },
  {
    key: "type",
    label: "Assigning your type",
    detail: "This one is personal",
  },
  {
    key: "wrapping",
    label: "Wrapping it up",
    detail: "Almost done",
  },
];

export const EDITORIAL_QUOTES = [
  "Every conversation leaves a trace.",
  "The months you show up are the ones that compound.",
  "Your relationships are your record.",
  "Consistency is invisible until it isn't.",
];

// =============================================================================
// Normalization — guards against schema drift in cached JSON
// =============================================================================

function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

export function normalizeMonthlyWrapped(raw: Record<string, unknown>): MonthlyWrappedContent {
  const p = (raw.personality ?? {}) as Record<string, unknown>;
  return {
    monthLabel: (raw.monthLabel as string) ?? "",
    totalConversations: (raw.totalConversations as number) ?? 0,
    totalEntitiesTouched: (raw.totalEntitiesTouched as number) ?? 0,
    totalPeopleMet: (raw.totalPeopleMet as number) ?? 0,
    signalsCaptured: (raw.signalsCaptured as number) ?? 0,
    topEntityName: (raw.topEntityName as string) ?? "",
    topEntityTouches: (raw.topEntityTouches as number) ?? 0,
    moments: toArr<WrappedMoment>(raw.moments),
    hiddenPattern: (raw.hiddenPattern as string) ?? "",
    personality: {
      typeName: (p.typeName as string) ?? "",
      description: (p.description as string) ?? "",
      keySignal: (p.keySignal as string) ?? "",
      rarityLabel: (p.rarityLabel as string) ?? "",
    },
    priorityAlignmentPct: (raw.priorityAlignmentPct as number) ?? null,
    priorityAlignmentLabel: (raw.priorityAlignmentLabel as string) ?? null,
    topWin: (raw.topWin as string) ?? "",
    carryForward: (raw.carryForward as string) ?? "",
    wordOne: (raw.wordOne as string) ?? "",
    wordTwo: (raw.wordTwo as string) ?? "",
    wordThree: (raw.wordThree as string) ?? "",
  };
}

// =============================================================================
// Helper — next month label
// =============================================================================

export function nextMonthName(monthLabel: string): string {
  const months = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
  ];
  const found = months.find((m) => monthLabel.toLowerCase().startsWith(m.toLowerCase()));
  if (!found) return "next month";
  const idx = months.indexOf(found);
  return months[(idx + 1) % 12];
}

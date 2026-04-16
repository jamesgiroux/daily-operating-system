/**
 * Utility functions for the account detail page.
 * Extracted from AccountDetailEditorial to keep the page component thin.
 */
import React from "react";
import { formatArr, formatShortDate } from "@/lib/utils";
import type { VitalDisplay } from "@/lib/entity-types";
import {
  AlignLeft,
  BarChart3,
  Briefcase,
  Clock,
  Users,
  Eye,
  Activity,
  CheckSquare2,
  FileText,
  Award,
  Compass,
  Telescope,
} from "lucide-react";

/* ── Vitals assembly ── */

function formatRenewalCountdown(dateStr: string): string {
  try {
    const renewal = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.round(
      (renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24),
    );
    if (diffDays < 0) return `${Math.abs(diffDays)}d overdue`;
    return `Renewal in ${diffDays}d`;
  } catch {
    return dateStr;
  }
}

const healthColorMap: Record<string, "saffron" | undefined> = {
  yellow: "saffron",
};

export function buildAccountVitals(detail: {
  arr?: number | null;
  health?: string;
  lifecycle?: string;
  renewalDate?: string;
  renewalStage?: string | null;
  commercialStage?: string | null;
  nps?: number | null;
  signals?: { meetingFrequency30d?: number };
  contractStart?: string;
}): VitalDisplay[] {
  const vitals: VitalDisplay[] = [];
  if (detail.arr != null) {
    vitals.push({ text: `$${formatArr(detail.arr)} ARR`, highlight: "turmeric" });
  }
  if (detail.health) {
    vitals.push({
      text: `${detail.health.charAt(0).toUpperCase() + detail.health.slice(1)} Health`,
      highlight: healthColorMap[detail.health],
    });
  }
  if (detail.lifecycle) vitals.push({ text: detail.lifecycle });
  if (detail.renewalStage) {
    vitals.push({
      text: `Stage: ${detail.renewalStage.replace(/_/g, " ")}`,
      highlight: "olive",
    });
  }
  if (detail.commercialStage) {
    vitals.push({
      text: `Opp: ${detail.commercialStage.replace(/_/g, " ")}`,
      highlight: "larkspur",
    });
  }
  if (detail.renewalDate) {
    const renewal = new Date(detail.renewalDate);
    const now = new Date();
    const diffDays = Math.round((renewal.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
    vitals.push({
      text: formatRenewalCountdown(detail.renewalDate),
      highlight: diffDays <= 60 ? "saffron" : undefined,
    });
  }
  if (detail.nps != null) vitals.push({ text: `NPS ${detail.nps}` });
  if (detail.signals?.meetingFrequency30d != null) {
    vitals.push({ text: `${detail.signals.meetingFrequency30d} meetings / 30d` });
  }
  if (detail.contractStart) {
    vitals.push({ text: `Contract: ${formatShortDate(detail.contractStart)}` });
  }
  return vitals;
}

/* ── Chapter definitions ── */

const BASE_CHAPTERS: { id: string; label: string; icon: React.ReactNode }[] = [
  { id: "headline", label: "The Headline", icon: React.createElement(AlignLeft, { size: 18, strokeWidth: 1.5 }) },
  { id: "outlook", label: "Outlook", icon: React.createElement(Telescope, { size: 18, strokeWidth: 1.5 }) },
  { id: "state-of-play", label: "State of Play", icon: React.createElement(Clock, { size: 18, strokeWidth: 1.5 }) },
  { id: "the-room", label: "The Room", icon: React.createElement(Users, { size: 18, strokeWidth: 1.5 }) },
  { id: "watch-list", label: "Watch List", icon: React.createElement(Eye, { size: 18, strokeWidth: 1.5 }) },
  { id: "value-commitments", label: "Value & Commitments", icon: React.createElement(Award, { size: 18, strokeWidth: 1.5 }) },
  { id: "strategic-landscape", label: "Competitive & Strategic", icon: React.createElement(Compass, { size: 18, strokeWidth: 1.5 }) },
  { id: "the-record", label: "The Record", icon: React.createElement(Activity, { size: 18, strokeWidth: 1.5 }) },
  { id: "the-work", label: "The Work", icon: React.createElement(CheckSquare2, { size: 18, strokeWidth: 1.5 }) },
  { id: "reports", label: "Reports", icon: React.createElement(FileText, { size: 18, strokeWidth: 1.5 }) },
];

const PORTFOLIO_CHAPTER = {
  id: "portfolio",
  label: "Portfolio",
  icon: React.createElement(Briefcase, { size: 18, strokeWidth: 1.5 }),
};

const HEALTH_CHAPTER = {
  id: "relationship-health",
  label: "Health",
  icon: React.createElement(BarChart3, { size: 18, strokeWidth: 1.5 }),
};

export function buildChapters(isParent: boolean, hasHealth: boolean) {
  let chapters = [...BASE_CHAPTERS];
  if (isParent) {
    chapters.splice(2, 0, PORTFOLIO_CHAPTER);
  }
  const sopIndex = chapters.findIndex((c) => c.id === "state-of-play");
  if (hasHealth && sopIndex >= 0) {
    chapters.splice(sopIndex + 1, 0, HEALTH_CHAPTER);
  }
  return chapters;
}

/* ── Per-view chapter builders (DOS-112) ── */

export function buildHealthChapters(isParent: boolean, hasHealth: boolean) {
  const chapters: { id: string; label: string; icon: React.ReactNode }[] = [];
  if (hasHealth) {
    chapters.push(HEALTH_CHAPTER);
  }
  chapters.push(
    { id: "outlook", label: "Outlook", icon: React.createElement(Telescope, { size: 18, strokeWidth: 1.5 }) },
  );
  if (isParent) {
    chapters.push(PORTFOLIO_CHAPTER);
  }
  chapters.push(
    { id: "products", label: "Products", icon: React.createElement(Activity, { size: 18, strokeWidth: 1.5 }) },
  );
  return chapters;
}

export function buildContextChapters() {
  return [
    { id: "state-of-play", label: "State of Play", icon: React.createElement(Clock, { size: 18, strokeWidth: 1.5 }) },
    { id: "the-room", label: "The Room", icon: React.createElement(Users, { size: 18, strokeWidth: 1.5 }) },
    { id: "strategic-landscape", label: "Competitive & Strategic", icon: React.createElement(Compass, { size: 18, strokeWidth: 1.5 }) },
    { id: "value-commitments", label: "Value & Commitments", icon: React.createElement(Award, { size: 18, strokeWidth: 1.5 }) },
    { id: "the-record", label: "The Record", icon: React.createElement(Activity, { size: 18, strokeWidth: 1.5 }) },
    { id: "files", label: "Files", icon: React.createElement(FileText, { size: 18, strokeWidth: 1.5 }) },
  ];
}

export function buildWorkChapters() {
  return [
    { id: "the-work", label: "The Work", icon: React.createElement(CheckSquare2, { size: 18, strokeWidth: 1.5 }) },
    { id: "watch-list", label: "Watch List", icon: React.createElement(Eye, { size: 18, strokeWidth: 1.5 }) },
    { id: "reports", label: "Reports", icon: React.createElement(FileText, { size: 18, strokeWidth: 1.5 }) },
  ];
}

/* ── Field formatting helpers ── */

export function formatTrackedFieldLabel(field: string): string {
  const labels: Record<string, string> = {
    arr: "ARR",
    lifecycle: "Lifecycle",
    contract_end: "Renewal Date",
    nps: "NPS",
  };
  return labels[field] ?? field.replace(/_/g, " ");
}

export function formatLifecycleDisplay(value?: string | null): string {
  if (!value) return "Unknown";
  return value
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

export function formatSuggestedValue(field: string, value: string): string {
  if (field === "arr") {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? `$${formatArr(parsed)} ARR` : value;
  }
  if (field === "contract_end") {
    return `Renews ${formatShortDate(value)}`;
  }
  if (field === "lifecycle") {
    return formatLifecycleDisplay(value);
  }
  if (field === "nps") {
    return `NPS ${value}`;
  }
  return value;
}

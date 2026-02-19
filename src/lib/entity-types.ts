/**
 * entity-types.ts — Narrow interfaces for generalized entity editorial components.
 * Uses structural typing so AccountDetail, ProjectDetail, and PersonDetail
 * all satisfy these without adapters.
 */
import type { Action, EmailSignal } from "@/types";

/** A single vital metric for VitalsStrip. */
export interface VitalDisplay {
  text: string;
  highlight?: "turmeric" | "saffron" | "olive" | "larkspur";
}

/** Data source for UnifiedTimeline (The Record). */
export interface TimelineSource {
  recentMeetings: { id: string; title: string; startTime: string; meetingType: string }[];
  recentEmailSignals?: EmailSignal[];
  recentCaptures?: { id: string; captureType: string; content: string; meetingTitle: string; meetingId?: string }[];
  accountEvents?: { id: number; eventType: string; eventDate: string; arrImpact?: number; notes?: string }[];
}

/** Data source for TheWork (commitments + upcoming meetings). */
export interface WorkSource {
  openActions: Action[];
  upcomingMeetings?: { id: string; title: string; startTime: string; meetingType: string }[];
}

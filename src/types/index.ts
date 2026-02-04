/**
 * Dashboard Types
 * Core type definitions for the DailyOS dashboard
 */

export type MeetingType = "customer" | "internal" | "personal";

export type Priority = "P1" | "P2" | "P3";

export type ActionStatus = "pending" | "completed";

export interface Meeting {
  id: string;
  time: string;
  endTime?: string;
  title: string;
  type: MeetingType;
  account?: string;
  prep?: MeetingPrep;
  isCurrent?: boolean;
}

export interface MeetingPrep {
  metrics?: string[];
  risks?: string[];
  wins?: string[];
  actions?: string[];
  context?: string;
}

export interface Action {
  id: string;
  title: string;
  account?: string;
  dueDate?: string;
  priority: Priority;
  status: ActionStatus;
  isOverdue?: boolean;
}

export interface DayStats {
  totalMeetings: number;
  customerMeetings: number;
  actionsDue: number;
  inboxCount: number;
}

export interface DashboardData {
  overview: {
    greeting: string;
    date: string;
    summary: string;
    focus?: string;
  };
  stats: DayStats;
  meetings: Meeting[];
  actions: Action[];
}

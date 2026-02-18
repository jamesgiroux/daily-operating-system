import type { DashboardData } from "@/types";

/**
 * Mock data for the dashboard
 * Represents a realistic workday with meetings and actions
 */

export const mockDashboardData: DashboardData = {
  overview: {
    greeting: "Good morning",
    date: "Tuesday, February 4",
    summary:
      "You have 4 meetings today with 2 customer calls. Your QBR with Acme Corp is this afternoon — prep materials are ready.",
    focus: "QBR Prep for Acme Corp",
  },
  stats: {
    totalMeetings: 4,
    customerMeetings: 2,
    actionsDue: 3,
    inboxCount: 7,
  },
  meetings: [
    {
      id: "1",
      time: "9:00 AM",
      endTime: "9:30 AM",
      title: "Team Standup",
      type: "internal",
      hasPrep: false,
    },
    {
      id: "2",
      time: "10:30 AM",
      endTime: "11:00 AM",
      title: "Discovery Call",
      type: "customer",
      linkedEntities: [{ id: "e1", name: "TechStart Inc", entityType: "account" }],
      isCurrent: true,
      hasPrep: true,
      prepFile: "02-1030-customer-techstart-prep.md",
      prep: {
        context:
          "First meeting with TechStart. They reached out after the webinar on AI productivity.",
        metrics: [
          "Series A, 45 employees",
          "$2.3M ARR, growing 15% MoM",
          "Currently using Notion + scattered tools",
        ],
        risks: [
          "Small team may have budget constraints",
          "Founder-led sales process, may move slowly",
        ],
        wins: [
          "Strong product-market fit signal from inbound",
          "Technical founder will appreciate our architecture",
        ],
        actions: [
          "Confirm their current workflow pain points",
          "Demo the AI briefing feature",
          "Schedule technical deep-dive if interested",
        ],
      },
    },
    {
      id: "3",
      time: "2:00 PM",
      endTime: "3:00 PM",
      title: "Quarterly Business Review",
      type: "customer",
      linkedEntities: [{ id: "e2", name: "Acme Corp", entityType: "account" }],
      hasPrep: true,
      prepFile: "03-1400-customer-acme-prep.md",
      prep: {
        context:
          "Q4 QBR with our largest enterprise customer. Renewal coming up in March.",
        metrics: [
          "Usage up 34% this quarter",
          "NPS score: 72 (up from 65)",
          "12 active users, up from 8",
        ],
        risks: [
          "Champion Sarah mentioned budget pressure from CFO",
          "Competitor demo scheduled for next week",
        ],
        wins: [
          "Saved 6 hours/week per user on average",
          "Zero critical incidents this quarter",
          "Expansion into marketing team approved",
        ],
        actions: [
          "Present ROI summary for CFO meeting",
          "Discuss multi-year discount options",
          "Get commitment on marketing pilot timeline",
        ],
      },
    },
    {
      id: "4",
      time: "5:30 PM",
      endTime: "6:00 PM",
      title: "Gym - Personal Training",
      type: "personal",
      hasPrep: false,
    },
  ],
  actions: [
    {
      id: "a1",
      title: "Send follow-up proposal",
      account: "TechStart Inc",
      dueDate: "Today",
      priority: "P1",
      status: "pending",
    },
    {
      id: "a2",
      title: "Review QBR slides",
      account: "Acme Corp",
      dueDate: "Today",
      priority: "P1",
      status: "pending",
    },
    {
      id: "a3",
      title: "Prepare ROI analysis",
      account: "Acme Corp",
      dueDate: "Yesterday",
      priority: "P1",
      status: "pending",
      isOverdue: true,
    },
    {
      id: "a4",
      title: "Schedule intro call with VP of Sales",
      account: "BigCo Industries",
      dueDate: "This week",
      priority: "P2",
      status: "pending",
    },
    {
      id: "a5",
      title: "Update CRM notes from last call",
      account: "TechStart Inc",
      dueDate: "This week",
      priority: "P3",
      status: "completed",
    },
  ],
};

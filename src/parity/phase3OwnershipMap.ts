export interface SurfaceOwnership {
  id: string;
  label: string;
  routes: string[];
  owners: string[];
  notes: string;
}

export const PHASE3_SURFACE_OWNERSHIP: SurfaceOwnership[] = [
  {
    id: "dashboard_briefing",
    label: "Dashboard / Briefing",
    routes: ["/"],
    owners: ["hooks/useDashboardData.ts", "components/dashboard/DailyBriefing.tsx"],
    notes: "Dashboard data is normalized in the hook and rendered through the daily briefing shell.",
  },
  {
    id: "actions",
    label: "Actions",
    routes: ["/actions"],
    owners: ["hooks/useActions.ts", "pages/ActionsPage.tsx"],
    notes: "ActionsPage owns composition; useActions owns DB-backed pending and proposed action loading.",
  },
  {
    id: "account_detail",
    label: "Account Detail",
    routes: ["/accounts/$accountId"],
    owners: ["hooks/useAccountDetail.ts", "pages/AccountDetailEditorial.tsx"],
    notes: "Account detail state is centralized in the hook and rendered through the editorial page.",
  },
  {
    id: "project_detail",
    label: "Project Detail",
    routes: ["/projects/$projectId"],
    owners: ["hooks/useProjectDetail.ts", "pages/ProjectDetailEditorial.tsx"],
    notes: "Project detail fetch and normalization stay in the hook; page owns chapter composition.",
  },
  {
    id: "person_detail",
    label: "Person Detail",
    routes: ["/people/$personId"],
    owners: ["hooks/usePersonDetail.ts", "pages/PersonDetailEditorial.tsx"],
    notes: "Person detail route uses a single hook-plus-page ownership path.",
  },
  {
    id: "meeting_detail",
    label: "Meeting Detail",
    routes: ["/meeting/$meetingId"],
    owners: ["pages/MeetingDetailPage.tsx"],
    notes: "Meeting detail currently owns its data loading in-page and is the single routed surface for meeting intelligence.",
  },
  {
    id: "inbox_emails",
    label: "Inbox / Emails",
    routes: ["/inbox", "/emails"],
    owners: ["hooks/useInbox.ts", "pages/InboxPage.tsx", "pages/EmailsPage.tsx"],
    notes: "Inbox and Emails remain separate routes but share one parity surface because both depend on inbox/email command shapes.",
  },
  {
    id: "settings_data",
    label: "Settings / Data",
    routes: ["/settings"],
    owners: ["pages/SettingsPage.tsx", "components/settings/DatabaseRecoveryCard.tsx"],
    notes: "SettingsPage owns section composition; the recovery card owns database/recovery data rendering.",
  },
  {
    id: "reports",
    label: "Reports",
    routes: [
      "/accounts/$accountId/reports/risk_briefing",
      "/accounts/$accountId/reports/swot",
      "/accounts/$accountId/reports/$reportType",
      "/accounts/$accountId/reports/account_health",
      "/accounts/$accountId/reports/ebr_qbr",
      "/me/reports/$reportType",
      "/me/reports/weekly_impact",
      "/me/reports/monthly_wrapped",
      "/me/reports/book_of_business",
    ],
    owners: [
      "pages/RiskBriefingPage.tsx",
      "pages/SwotPage.tsx",
      "pages/ReportPage.tsx",
      "pages/AccountHealthPage.tsx",
      "pages/EbrQbrPage.tsx",
      "pages/WeeklyImpactPage.tsx",
      "pages/monthly-wrapped/MonthlyWrappedPage.tsx",
      "pages/BookOfBusinessPage.tsx",
    ],
    notes: "Report routes are split by report type, but all Phase 3 report command contracts flow through these routed pages.",
  },
];

/**
 * Preset-aware report configuration.
 * Determines which reports appear in the account Reports dropdown
 * and what they're called for each role preset.
 */

export interface AccountReportItem {
  label: string;
  reportType: 'account_health' | 'ebr_qbr' | 'swot' | null;
  /** null reportType = risk briefing (legacy route) */
}

/** Reports shown for each preset, in display order. */
const PRESET_REPORTS: Record<string, AccountReportItem[]> = {
  'customer-success': [
    { label: 'Account Health',  reportType: 'account_health' },
    { label: 'EBR / QBR',       reportType: 'ebr_qbr' },
    { label: 'SWOT',            reportType: 'swot' },
    { label: 'Risk Briefing',   reportType: null },
  ],
  'sales': [
    { label: 'Account Overview', reportType: 'account_health' },
    { label: 'Business Review',  reportType: 'ebr_qbr' },
    { label: 'SWOT',             reportType: 'swot' },
    { label: 'Risk Briefing',    reportType: null },
  ],
  'agency': [
    { label: 'Client Health',   reportType: 'account_health' },
    { label: 'Client Review',   reportType: 'ebr_qbr' },
    { label: 'SWOT',            reportType: 'swot' },
    { label: 'Risk Briefing',   reportType: null },
  ],
  'consulting': [
    { label: 'Engagement Health', reportType: 'account_health' },
    { label: 'Executive Review',  reportType: 'ebr_qbr' },
    { label: 'SWOT',              reportType: 'swot' },
    { label: 'Risk Briefing',     reportType: null },
  ],
  'partnerships': [
    { label: 'Partner Health',  reportType: 'account_health' },
    { label: 'Partner Review',  reportType: 'ebr_qbr' },
    { label: 'SWOT',            reportType: 'swot' },
    { label: 'Risk Briefing',   reportType: null },
  ],
  'leadership': [
    { label: 'Account Overview',   reportType: 'account_health' },
    { label: 'Executive Briefing', reportType: 'ebr_qbr' },
    { label: 'SWOT',               reportType: 'swot' },
    { label: 'Risk Briefing',      reportType: null },
  ],
  'marketing': [
    // Account health and EBR/QBR don't fit a marketing workflow
    { label: 'SWOT',          reportType: 'swot' },
    { label: 'Risk Briefing', reportType: null },
  ],
  'product': [
    // EBR/QBR and account health don't fit a product workflow
    { label: 'SWOT',          reportType: 'swot' },
    { label: 'Risk Briefing', reportType: null },
  ],
  'the-desk': [
    { label: 'SWOT',          reportType: 'swot' },
    { label: 'Risk Briefing', reportType: null },
  ],
};

/** Fallback to customer-success if preset unknown. */
export function getAccountReports(presetId: string | null | undefined): AccountReportItem[] {
  return PRESET_REPORTS[presetId ?? 'customer-success'] ?? PRESET_REPORTS['customer-success'];
}

/** Preset-aware label for the cross-account portfolio report. */
export function getPortfolioReportLabel(presetId: string | null | undefined): string {
  switch (presetId) {
    case 'customer-success':
    case 'sales':
      return 'Book of Business';
    case 'agency':
      return 'Client Portfolio Review';
    case 'consulting':
      return 'Engagement Portfolio';
    case 'partnerships':
      return 'Partner Portfolio';
    case 'leadership':
      return 'Portfolio Review';
    default:
      return 'Book of Business';
  }
}

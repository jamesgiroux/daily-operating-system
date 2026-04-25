/**
 * Preset-aware report configuration.
 * Determines which reports appear in the account Reports dropdown
 * and what they're called for each role preset.
 */

export interface AccountReportItem {
  label: string;
  reportType: 'account_health' | 'ebr_qbr' | 'swot' | 'risk_briefing';
}

/** Reports shown for each preset, in display order. */
const PRESET_REPORTS: Record<string, AccountReportItem[]> = {
  'core': [
    { label: 'SWOT',          reportType: 'swot' },
    { label: 'Risk Briefing', reportType: 'risk_briefing' },
  ],
  'customer-success': [
    { label: 'Account Health',  reportType: 'account_health' },
    { label: 'EBR / QBR',       reportType: 'ebr_qbr' },
    { label: 'SWOT',            reportType: 'swot' },
    { label: 'Risk Briefing',   reportType: 'risk_briefing' },
  ],
  'sales': [
    { label: 'Account Overview', reportType: 'account_health' },
    { label: 'Business Review',  reportType: 'ebr_qbr' },
    { label: 'SWOT',             reportType: 'swot' },
    { label: 'Risk Briefing',    reportType: 'risk_briefing' },
  ],
  'agency': [
    { label: 'Client Health',   reportType: 'account_health' },
    { label: 'Client Review',   reportType: 'ebr_qbr' },
    { label: 'SWOT',            reportType: 'swot' },
    { label: 'Risk Briefing',   reportType: 'risk_briefing' },
  ],
  'consulting': [
    { label: 'Engagement Health', reportType: 'account_health' },
    { label: 'Executive Review',  reportType: 'ebr_qbr' },
    { label: 'SWOT',              reportType: 'swot' },
    { label: 'Risk Briefing',     reportType: 'risk_briefing' },
  ],
  'affiliates-partnerships': [
    { label: 'Partner Health',  reportType: 'account_health' },
    { label: 'Partner Review',  reportType: 'ebr_qbr' },
    { label: 'SWOT',            reportType: 'swot' },
    { label: 'Risk Briefing',   reportType: 'risk_briefing' },
  ],
  'leadership': [
    { label: 'Account Overview',   reportType: 'account_health' },
    { label: 'Executive Briefing', reportType: 'ebr_qbr' },
    { label: 'SWOT',               reportType: 'swot' },
    { label: 'Risk Briefing',      reportType: 'risk_briefing' },
  ],
  'product-marketing': [
    // EBR/QBR and account health don't fit a product workflow
    { label: 'SWOT',          reportType: 'swot' },
    { label: 'Risk Briefing', reportType: 'risk_briefing' },
  ],
};

function canonicalPresetId(presetId: string | null | undefined): string {
  switch (presetId) {
    case 'the-desk':
      return 'core';
    case 'affiliates':
    case 'partnerships':
      return 'affiliates-partnerships';
    case 'product':
    case 'marketing':
      return 'product-marketing';
    default:
      return presetId ?? 'core';
  }
}

/** Fallback to core if preset unknown. */
export function getAccountReports(presetId: string | null | undefined): AccountReportItem[] {
  return PRESET_REPORTS[canonicalPresetId(presetId)] ?? PRESET_REPORTS['core'];
}

/** Preset-aware label for the cross-account portfolio report. */
export function getPortfolioReportLabel(presetId: string | null | undefined): string {
  switch (presetId) {
    case 'customer-success':
      return 'Book of Business';
    case 'affiliates-partnerships':
    case 'affiliates':
    case 'partnerships':
      return 'Partner Portfolio';
    case 'product-marketing':
    case 'product':
    case 'marketing':
      return 'Initiative Portfolio';
    default:
      return 'Project Portfolio';
  }
}

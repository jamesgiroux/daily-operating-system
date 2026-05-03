/**
 * Per-preset playbook configurations for the /me page.
 * Each preset defines which playbook fields to render,
 * their labels, and placeholder text.
 */

export interface PresetPlaybook {
  key: string;
  label: string;
  placeholder: string;
}

export interface PresetMeConfig {
  playbooks: PresetPlaybook[];
  placeholders: Record<string, string>;
  prominence: Record<string, "featured" | "shown" | "collapsed">;
}

const PRESET_ME_CONFIGS: Record<string, PresetMeConfig> = {
  "customer-success": {
    playbooks: [
      { key: "at_risk_accounts", label: "At-Risk Accounts", placeholder: "How do you approach accounts showing risk signals?" },
      { key: "renewal_approach", label: "Renewal Approach", placeholder: "What's your renewal strategy and timeline?" },
      { key: "ebr_qbr_prep", label: "EBR/QBR Preparation", placeholder: "How do you prepare for executive business reviews?" },
    ],
    placeholders: {
      value_proposition: "What does your platform do for customers? Write it as a one-sentence outcome, not a feature list.",
      success_definition: "What does a healthy, successful customer look like 12 months in?",
      product_context: "Platform, features, and integrations your customers use day-to-day.",
      pricing_model: "How your product is priced — per seat, usage-based, tiered...",
      competitive_context: "Key competitors your customers evaluate. What makes you win or lose?",
      priorities: "What accounts or outcomes matter most right now?",
    },
    prominence: {
      "what-i-deliver": "featured",
      "my-priorities": "featured",
      "my-playbooks": "featured",
      "context-entries": "shown",
    },
  },
  "sales": {
    playbooks: [
      { key: "deal_review", label: "Deal Review", placeholder: "How do you evaluate and progress deals?" },
      { key: "territory_planning", label: "Territory Planning", placeholder: "How do you prioritize your territory?" },
      { key: "competitive_response", label: "Competitive Response", placeholder: "How do you handle competitive situations?" },
    ],
    placeholders: {
      value_proposition: "What do you sell and what makes it win against the competition?",
      success_definition: "What does a closed-won deal look like? Average deal size, cycle, and shape.",
      product_context: "Product lines, packaging, and what resonates in demos.",
      pricing_model: "Pricing structure, discount authority, and deal desk process.",
      competitive_context: "Head-to-head competitors, win/loss themes, and positioning traps.",
      priorities: "What deals or targets matter most this quarter?",
    },
    prominence: {
      "what-i-deliver": "featured",
      "my-priorities": "featured",
      "my-playbooks": "featured",
      "context-entries": "shown",
    },
  },
  "marketing": {
    playbooks: [
      { key: "campaign_retrospective", label: "Campaign Retrospective", placeholder: "How do you evaluate campaign performance?" },
      { key: "launch_playbook", label: "Launch Playbook", placeholder: "What's your standard launch process?" },
      { key: "channel_strategy", label: "Channel Strategy", placeholder: "How do you approach channel mix and optimization?" },
    ],
    placeholders: {
      value_proposition: "What's the core message you take to market? One sentence.",
      success_definition: "What metrics define a successful quarter for your team?",
      product_context: "Products and features you market. Key differentiators in messaging.",
      pricing_model: "How pricing shows up in your campaigns and positioning.",
      competitive_context: "Competitive landscape from a messaging and positioning perspective.",
      priorities: "What campaigns or programs are your focus right now?",
    },
    prominence: {
      "what-i-deliver": "shown",
      "my-priorities": "featured",
      "my-playbooks": "featured",
      "context-entries": "featured",
    },
  },
  "affiliates-partnerships": {
    playbooks: [
      { key: "partner_review", label: "Partner Review", placeholder: "How do you review partner, creator, or affiliate performance?" },
      { key: "activation_motion", label: "Activation Motion", placeholder: "How do you onboard and activate new partners?" },
      { key: "commercial_motion", label: "Commercial Motion", placeholder: "How do you structure referrals, commissions, co-sell, or payouts?" },
    ],
    placeholders: {
      value_proposition: "What value does your partner, affiliate, or creator program deliver?",
      success_definition: "What does a thriving partner relationship look like? Revenue, attribution, co-sell, integrations?",
      product_context: "Products, integrations, offers, and assets partners need to represent you well.",
      pricing_model: "Referral fees, commission, revenue share, reseller terms, or payout structure.",
      competitive_context: "Competing partner ecosystems, affiliate programs, or creator offers.",
      priorities: "Which partners, creators, or campaigns need attention right now?",
    },
    prominence: {
      "what-i-deliver": "shown",
      "my-priorities": "featured",
      "my-playbooks": "featured",
      "context-entries": "shown",
    },
  },
  "agency": {
    playbooks: [
      { key: "scope_change", label: "Scope Change", placeholder: "How do you handle scope changes with clients?" },
      { key: "client_escalation", label: "Client Escalation", placeholder: "What's your escalation process?" },
      { key: "retainer_review", label: "Retainer Review", placeholder: "How do you review and adjust retainers?" },
    ],
    placeholders: {
      value_proposition: "What does your agency do better than anyone? One sentence.",
      success_definition: "What does a successful client engagement look like at your shop?",
      product_context: "Services, capabilities, and tools you deliver with.",
      pricing_model: "How you price — retainer, project-based, T&M, or blended.",
      competitive_context: "Competing agencies and what makes clients pick you.",
      priorities: "Which clients or projects need attention right now?",
    },
    prominence: {
      "what-i-deliver": "featured",
      "my-priorities": "featured",
      "my-playbooks": "featured",
      "context-entries": "shown",
    },
  },
  "consulting": {
    playbooks: [
      { key: "engagement_kickoff", label: "Engagement Kickoff", placeholder: "How do you start new engagements?" },
      { key: "stakeholder_alignment", label: "Stakeholder Alignment", placeholder: "How do you align stakeholders?" },
      { key: "findings_presentation", label: "Findings Presentation", placeholder: "How do you present findings and recommendations?" },
    ],
    placeholders: {
      value_proposition: "What transformation do you help clients achieve?",
      success_definition: "What does a successful engagement outcome look like?",
      product_context: "Frameworks, methodologies, and deliverables you bring.",
      pricing_model: "Engagement pricing — fixed fee, daily rate, outcome-based.",
      competitive_context: "Competing firms and your differentiation.",
      priorities: "Which engagements or deliverables are top priority?",
    },
    prominence: {
      "what-i-deliver": "featured",
      "my-priorities": "shown",
      "my-playbooks": "featured",
      "context-entries": "featured",
    },
  },
  "product-marketing": {
    playbooks: [
      { key: "discovery_sprint", label: "Discovery Sprint", placeholder: "How do you run discovery or campaign planning?" },
      { key: "launch_checklist", label: "Launch Checklist", placeholder: "What's your launch readiness process?" },
      { key: "retrospective", label: "Retrospective", placeholder: "How do you evaluate shipped features or completed campaigns?" },
    ],
    placeholders: {
      value_proposition: "What problem does your product or campaign solve? Who for?",
      success_definition: "What adoption, launch, pipeline, or usage metrics define success?",
      product_context: "Your product area, campaign surface, channels, and key integrations.",
      pricing_model: "How packaging, pricing, or campaign economics affect the work.",
      competitive_context: "Direct and indirect competitors. Where you win and where you're vulnerable.",
      priorities: "What launches, campaigns, or initiatives are your focus?",
    },
    prominence: {
      "what-i-deliver": "featured",
      "my-priorities": "featured",
      "my-playbooks": "shown",
      "context-entries": "shown",
    },
  },
  "leadership": {
    playbooks: [
      { key: "team_operating_cadence", label: "Team Operating Cadence", placeholder: "What's your team's operating rhythm?" },
      { key: "board_prep", label: "Board Prep", placeholder: "How do you prepare for board meetings?" },
      { key: "strategic_review", label: "Strategic Review", placeholder: "How do you conduct strategic reviews?" },
    ],
    placeholders: {
      value_proposition: "What does your organization deliver? The elevator pitch for the board.",
      success_definition: "What outcomes is your team measured on this year?",
      product_context: "Products and business lines under your purview.",
      pricing_model: "Business model and pricing strategy at a portfolio level.",
      competitive_context: "Market landscape, strategic threats, and positioning.",
      priorities: "What strategic priorities matter most right now?",
    },
    prominence: {
      "what-i-deliver": "shown",
      "my-priorities": "featured",
      "my-playbooks": "featured",
      "context-entries": "shown",
    },
  },
  "core": {
    playbooks: [
      { key: "weekly_review", label: "Weekly Review", placeholder: "How do you run your weekly review?" },
      { key: "project_retrospective", label: "Project Retrospective", placeholder: "How do you reflect on completed projects?" },
      { key: "deep_work_planning", label: "Deep Work Planning", placeholder: "How do you plan and protect deep work time?" },
    ],
    placeholders: {
      value_proposition: "What do you do? Describe your work in one sentence.",
      success_definition: "What does a good week look like for you?",
      product_context: "Tools, systems, and platforms you work with.",
      pricing_model: "How your work is valued or compensated.",
      competitive_context: "Professional landscape and where you stand out.",
      priorities: "What projects or goals are your focus right now?",
    },
    prominence: {
      "what-i-deliver": "shown",
      "my-priorities": "featured",
      "my-playbooks": "shown",
      "context-entries": "shown",
    },
  },
};

export function getPresetMeConfig(presetId: string): PresetMeConfig {
  const canonical = (() => {
    switch (presetId) {
      case "the-desk":
        return "core";
      case "affiliates":
      case "partnerships":
        return "affiliates-partnerships";
      case "product":
      case "marketing":
        return "product-marketing";
      default:
        return presetId;
    }
  })();
  return PRESET_ME_CONFIGS[canonical] ?? PRESET_ME_CONFIGS["core"];
}

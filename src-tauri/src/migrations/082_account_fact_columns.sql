-- Account fact columns for Outlook chapter commercial intelligence.

-- ARR range (low/high bounds from Salesforce or Glean)
ALTER TABLE accounts ADD COLUMN arr_range_low REAL;
ALTER TABLE accounts ADD COLUMN arr_range_high REAL;

-- Renewal likelihood (0.0-1.0 confidence score)
ALTER TABLE accounts ADD COLUMN renewal_likelihood REAL;
ALTER TABLE accounts ADD COLUMN renewal_likelihood_source TEXT;
ALTER TABLE accounts ADD COLUMN renewal_likelihood_updated_at TEXT;

-- Renewal model and pricing method (e.g., "annual", "multi-year"; "flat", "usage-based")
ALTER TABLE accounts ADD COLUMN renewal_model TEXT;
ALTER TABLE accounts ADD COLUMN renewal_pricing_method TEXT;

-- Support tier on the account itself (mirrors technical_footprint but account-level source-tracked)
ALTER TABLE accounts ADD COLUMN support_tier TEXT;
ALTER TABLE accounts ADD COLUMN support_tier_source TEXT;
ALTER TABLE accounts ADD COLUMN support_tier_updated_at TEXT;

-- Active subscription count (integer, from Salesforce or Glean)
ALTER TABLE accounts ADD COLUMN active_subscription_count INTEGER;

-- Growth potential score (0.0-1.0, AI/Glean-derived)
ALTER TABLE accounts ADD COLUMN growth_potential_score REAL;
ALTER TABLE accounts ADD COLUMN growth_potential_score_source TEXT;

-- ICP fit score (0.0-1.0, AI/Glean-derived)
ALTER TABLE accounts ADD COLUMN icp_fit_score REAL;
ALTER TABLE accounts ADD COLUMN icp_fit_score_source TEXT;

-- Primary product (e.g., "CMS", "Analytics")
ALTER TABLE accounts ADD COLUMN primary_product TEXT;

-- Customer status (e.g., "active", "at_risk", "churned", "new")
ALTER TABLE accounts ADD COLUMN customer_status TEXT;
ALTER TABLE accounts ADD COLUMN customer_status_source TEXT;
ALTER TABLE accounts ADD COLUMN customer_status_updated_at TEXT;

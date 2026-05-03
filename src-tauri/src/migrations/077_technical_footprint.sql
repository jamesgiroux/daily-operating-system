-- Technical footprint, adoption, and service-delivery intelligence

CREATE TABLE IF NOT EXISTS account_technical_footprint (
  account_id TEXT PRIMARY KEY,
  integrations_json TEXT,        -- JSON array of integration names/types
  usage_tier TEXT,               -- 'enterprise', 'professional', 'starter', etc.
  adoption_score REAL,           -- 0.0-1.0
  active_users INTEGER,
  support_tier TEXT,             -- 'premium', 'standard', 'basic'
  csat_score REAL,
  open_tickets INTEGER DEFAULT 0,
  services_stage TEXT,           -- 'onboarding', 'implementation', 'optimization', 'steady-state'
  source TEXT NOT NULL DEFAULT 'glean',
  sourced_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

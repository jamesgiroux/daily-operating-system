-- DOS-110: User health sentiment columns for manual account assessment.
-- Both nullable. NULL = unset (renders as "Set your assessment" in UI).
ALTER TABLE accounts ADD COLUMN user_health_sentiment TEXT
  CHECK(user_health_sentiment IN ('strong','on_track','concerning','at_risk','critical'));

ALTER TABLE accounts ADD COLUMN sentiment_set_at TEXT;

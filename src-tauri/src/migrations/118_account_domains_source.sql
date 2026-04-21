-- DOS-258 follow-up: add source provenance to account_domains.
--
-- Before this migration, account_domains had no provenance — we couldn't
-- distinguish "user explicitly typed this domain" from "the old resolver
-- inferred this from a meeting attendee email and it was wrong."
--
-- raw_rebuild_account_domains (DOS-258 repository.rs) uses this column to
-- purge inferred domains before the new linking engine goes live.
--
-- Sources:
--   'user'        — explicitly entered by the user on the account page
--   'enrichment'  — from Clay/Glean/Google enrichment (trusted providers)
--   'inferred'    — guessed from meeting attendee emails (unreliable, default)

ALTER TABLE account_domains ADD COLUMN source TEXT NOT NULL DEFAULT 'inferred';

CREATE INDEX IF NOT EXISTS idx_account_domains_source
    ON account_domains (account_id, source);

-- Sprint 20: Internal teams foundation (ADR-0070)

ALTER TABLE accounts ADD COLUMN is_internal INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_accounts_internal ON accounts(is_internal);

CREATE TABLE IF NOT EXISTS account_domains (
    account_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    PRIMARY KEY (account_id, domain)
);
CREATE INDEX IF NOT EXISTS idx_account_domains_domain ON account_domains(domain);


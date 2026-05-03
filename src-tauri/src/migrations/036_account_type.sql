-- Replace boolean is_internal with account_type enum column.
-- Valid values: 'customer', 'internal', 'partner'.
ALTER TABLE accounts ADD COLUMN account_type TEXT NOT NULL DEFAULT 'customer';

-- Back-fill from existing is_internal flag.
UPDATE accounts SET account_type = 'internal' WHERE is_internal = 1;

-- Index for queries that filter by account type.
CREATE INDEX IF NOT EXISTS idx_accounts_account_type ON accounts(account_type);

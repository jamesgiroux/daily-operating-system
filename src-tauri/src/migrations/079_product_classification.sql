-- I651: Product classification enrichment from Glean.
-- Extends account_products with Salesforce product classification fields.

ALTER TABLE account_products ADD COLUMN product_type TEXT;
ALTER TABLE account_products ADD COLUMN tier TEXT;
ALTER TABLE account_products ADD COLUMN billing_terms TEXT;
ALTER TABLE account_products ADD COLUMN arr REAL;
ALTER TABLE account_products ADD COLUMN last_verified_at TEXT;
ALTER TABLE account_products ADD COLUMN data_source TEXT;

-- Create unique constraint for product classification
-- One row per account/product_type/data_source tuple to prevent duplicates
CREATE UNIQUE INDEX IF NOT EXISTS idx_account_products_upsert_key
    ON account_products(account_id, product_type, data_source)
    WHERE product_type IS NOT NULL AND data_source IS NOT NULL;

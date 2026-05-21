-- v1.4.5 W1-A / DOS-463 (cycle 8 Option B-prime) — entity-relative sub-category
-- column for AI navigation (Accounts/Acme/presentations/q1.pdf etc.).
--
-- String values are canonical WorkspaceCategory::as_slug() output (lowercase
-- ASCII slugs: "presentations", "transcripts", "meetings", "notes",
-- "contracts", "attachments", or registry-validated Other slugs). NULL means
-- the file landed at entity root with no sub-directory (auto-detection found
-- no rule match and the caller did not provide a category hint).
--
-- No CHECK constraint here — per-entity registry validation is W1-B's
-- WorkspaceCategoryRegistry::validate boundary; auto-detection
-- (W2-A pipeline) and caller-provided slug validation (W4-C DOS-474) both run
-- before any INSERT/UPDATE hits this column.

ALTER TABLE workspace_file_lifecycle ADD COLUMN category TEXT;

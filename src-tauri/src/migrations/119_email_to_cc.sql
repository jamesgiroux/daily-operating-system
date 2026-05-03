-- store email To/Cc recipients for multi-participant
-- domain evidence in the entity linking engine (P4b/P4c rules).
--
-- Previously, email_adapter::build_context only had access to the sender
-- (From) because DbEmail had no recipient columns. Rules P4b/P4c (domain
-- evidence) and P4a (1:1 internal×external) could not evaluate recipient
-- domains — a 1:1 email where the user is From and a customer is To would
-- produce no external domain evidence.
--
-- Stored as comma-separated bare email addresses (lowercase):
--   "alice@acme.com,bob@acme.com"
-- Populated from Gmail To/Cc headers at sync time.

ALTER TABLE emails ADD COLUMN to_recipients TEXT;
ALTER TABLE emails ADD COLUMN cc_recipients TEXT;

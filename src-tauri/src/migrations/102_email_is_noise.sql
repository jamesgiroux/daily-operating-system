-- Email noise filter — hard drop.
-- Add is_noise column to emails table so bulk/automated/marketing email
-- can be suppressed entirely (not merely demoted to priority='low').
--
-- Suppressed emails are excluded from inbox listings, account Records,
-- entity contexts, and signal emission. The classify pipeline writes
-- this column on upsert; the rescue path is `unsuppress_email(email_id)`.

ALTER TABLE emails ADD COLUMN is_noise INTEGER NOT NULL DEFAULT 0;

-- Index for fast filtering on the dominant query pattern
-- (resolved_at IS NULL AND is_noise = 0).
CREATE INDEX IF NOT EXISTS idx_emails_is_noise ON emails(is_noise);

-- Backfill: any existing email whose sender domain ends with one of the
-- known bulk-marketing senders (kept in sync with prepare/constants.rs)
-- gets is_noise = 1. We use a conservative LIKE-based heuristic because
-- we don't have access to the original List-Unsubscribe header on
-- historical rows. Customer-domain emails are never marked: an account
-- domain match would have populated entity_id during enrichment.
UPDATE emails
SET is_noise = 1
WHERE entity_id IS NULL
  AND (
       sender_email LIKE '%@linkedin.com'
    OR sender_email LIKE '%@slack.com'
    OR sender_email LIKE '%@github.com'
    OR sender_email LIKE '%@notifications.github.com'
    OR sender_email LIKE '%@notion.so'
    OR sender_email LIKE '%@stripe.com'
    OR sender_email LIKE '%@amazonaws.com'
    OR sender_email LIKE '%@datadoghq.com'
    OR sender_email LIKE '%@atlassian.com'
    OR sender_email LIKE '%@calendly.com'
    OR sender_email LIKE '%@zoom.us'
    OR sender_email LIKE '%@loom.com'
    OR sender_email LIKE '%@docusign.net'
    OR sender_email LIKE '%@dropbox.com'
    OR sender_email LIKE '%@figma.com'
    OR sender_email LIKE '%@mailchimp.com'
    OR sender_email LIKE '%@sendgrid.net'
    OR sender_email LIKE '%@mandrillapp.com'
    OR sender_email LIKE '%@hubspot.com'
    OR sender_email LIKE '%@marketo.com'
    OR sender_email LIKE '%@pardot.com'
    OR sender_email LIKE '%@intercom.io'
    OR sender_email LIKE '%@customer.io'
    OR sender_email LIKE '%@mailgun.org'
    OR sender_email LIKE '%@postmarkapp.com'
    OR sender_email LIKE '%@amazonses.com'
  );

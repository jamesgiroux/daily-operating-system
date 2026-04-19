-- DOS-247: Recovery migration — un-suppress emails that DOS-242's overly
-- aggressive Rule 3 (List-Unsubscribe alone) flagged as noise.
--
-- Migration 102 + the original `should_suppress_email` rule treated any
-- email with a List-Unsubscribe header from an untracked domain as noise.
-- That over-fires: legitimate 1:1 customer email sent via Salesforce,
-- HubSpot, Outreach, Google Groups, etc. all carry that header.
--
-- This migration restores `is_noise = 0` for any row whose sender domain
-- is NOT in the bulk allow-list. The tightened rule (DOS-247) will
-- correctly re-classify those rows on the next email refresh; rows
-- legitimately suppressed by Rules 1 (bulk domain) or 2 (subject
-- pattern) stay suppressed because the sender domain check below is
-- the same domain list as `BULK_SENDER_DOMAINS` in
-- prepare/constants.rs (kept in sync manually).

UPDATE emails
SET is_noise = 0
WHERE is_noise = 1
  AND NOT (
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

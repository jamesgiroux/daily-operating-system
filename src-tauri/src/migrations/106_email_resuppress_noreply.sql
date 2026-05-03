-- Re-suppress emails that escaped the new noise rules.
--
-- The email noise recovery migration un-suppressed everything outside the
-- bulk allow-list to fix a 75+ false-positive case. That recovery was
-- correct but coarse — it also restored noreply / no-reply / donotreply
-- senders and emails with the new bracket-prefix subject patterns
-- ([New post], [WPVIP], etc.) that the tightened code rules suppress.
--
-- Re-mark these as is_noise = 1 so the user's existing data lines up
-- with the fixed rules without requiring a fresh sync.

UPDATE emails
SET is_noise = 1
WHERE is_noise = 0
  AND resolved_at IS NULL
  AND (
       sender_email LIKE 'noreply@%'
    OR sender_email LIKE 'no-reply@%'
    OR sender_email LIKE 'donotreply@%'
    OR sender_email LIKE 'do-not-reply@%'
    OR sender_email LIKE 'mailer-daemon@%'
    OR LOWER(COALESCE(subject, '')) LIKE '%[new post]%'
    OR LOWER(COALESCE(subject, '')) LIKE '%[new mention]%'
    OR LOWER(COALESCE(subject, '')) LIKE '%[new comment]%'
    OR LOWER(COALESCE(subject, '')) LIKE '%[wpvip]%'
    OR LOWER(COALESCE(subject, '')) LIKE '%registration confirmed%'
    OR LOWER(COALESCE(subject, '')) LIKE '%registration confirmation%'
    OR LOWER(COALESCE(subject, '')) LIKE '%thursday updates%'
    OR LOWER(COALESCE(subject, '')) LIKE '%spring cleaning%'
    OR LOWER(COALESCE(subject, '')) LIKE '%weekly digest%'
    OR LOWER(COALESCE(subject, '')) LIKE '%weekly summary%'
    OR LOWER(COALESCE(subject, '')) LIKE '%monthly digest%'
    OR LOWER(COALESCE(subject, '')) LIKE '%daily digest%'
    OR LOWER(COALESCE(subject, '')) LIKE '%added activity%'
    OR LOWER(COALESCE(subject, '')) LIKE '%your invitation%'
  );

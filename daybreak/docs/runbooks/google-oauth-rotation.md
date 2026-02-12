# Runbook: Google OAuth credential rotation (post-I158)

## Purpose

Rotate the Google OAuth Desktop client credentials after PKCE + Keychain
hardening is validated in production-like builds.

## Preconditions

1. I158 code is merged and released to internal/staging users.
2. Auth smoke checks pass:
   - existing users stay authenticated after upgrade;
   - fresh OAuth flow succeeds;
   - disconnect/reconnect succeeds.
3. Monitoring is available for auth failure rate (`start_google_auth`,
   calendar/email fetch auth errors).

## Procedure

1. **Create replacement OAuth desktop client**
   - In Google Cloud Console, create new OAuth Desktop App credentials.
   - Keep old client active during rollout.

2. **Prepare release update**
   - Update embedded OAuth `client_id` to the new client in code.
   - Keep PKCE flow unchanged.
   - Build internal candidate.

3. **Internal canary rollout**
   - Roll out to internal users first.
   - Verify:
     - new auth flows complete;
     - existing refresh-token users continue operating;
     - no spike in `invalid_client` or `AuthExpired` errors.

4. **General rollout**
   - Publish release to all users.
   - Observe auth metrics for at least one business day.

5. **Decommission old client**
   - Revoke old OAuth client only after rollout is stable.

## Rollback

If auth failure rate spikes after cutover:

1. Re-enable old OAuth client in Google Cloud (if already revoked).
2. Re-issue app build using previous `client_id`.
3. Notify users to retry auth if needed.
4. Collect failing auth payload patterns before attempting next rotation.

## Verification Checklist

- [ ] Existing authenticated users do not get forced re-auth.
- [ ] New users complete consent flow successfully.
- [ ] Calendar poller resumes successfully after auth.
- [ ] Gmail fetch/auth errors stay within baseline.
- [ ] No persistent `invalid_client` failures after full rollout.

# ADR-0068: OAuth PKCE + Keychain token hardening

**Date:** 2026-02-12  
**Status:** Accepted

## Context

Google OAuth in DailyOS used Authorization Code flow semantics that depended on
`client_secret` in source/runtime token payloads and persisted user tokens in
plaintext at `~/.dailyos/google/token.json`.

For prelaunch hardening (I158), this was not acceptable for a desktop client:

1. OAuth should use PKCE (`S256`) and state verification.
2. macOS token storage should use the system credential store.
3. Existing users must not be forced to re-authenticate.
4. Non-macOS builds must remain operational.

## Decision

DailyOS moves Google OAuth/token handling to the following model:

1. **PKCE by default**
   - Authorization requests include `code_challenge` + `code_challenge_method=S256`.
   - Callback validates `state`.
   - Token exchange sends `code_verifier`.

2. **Storage abstraction with macOS Keychain canonical backend**
   - New token-store boundary owns `load_token`, `save_token`, `delete_token`,
     and `peek_account_email`.
   - macOS canonical storage is Keychain (`service=com.dailyos.desktop.google-auth`,
     `account=oauth-token-v1`).
   - Non-macOS keeps file backend (`~/.dailyos/google/token.json`).

3. **One-time migration**
   - On macOS load: read Keychain first.
   - If missing, read legacy file, persist to Keychain, then remove file.

4. **Secretless default token operations**
   - Runtime token exchange/refresh paths do not send `client_secret` by default.
   - Legacy compatibility retry is allowed only when server returns
     `invalid_client` and a legacy secret is present.
   - Refreshed/persisted token payloads drop `client_secret`.

5. **Credential rotation is operational**
   - Client credential rotation is run as a separate post-validation operation,
     not coupled to this code deployment.

## Consequences

### Positive

- Eliminates plaintext token as canonical storage on macOS.
- Aligns desktop OAuth behavior with PKCE expectations.
- Preserves continuity for already-authenticated users through migration.
- Keeps cross-platform runtime behavior stable during rollout.

### Trade-offs

- macOS token operations now depend on OS credential APIs and migration handling.
- Legacy token compatibility code remains temporarily for safe transition.
- Credential rotation still requires operational follow-through after code ship.

## Related

- [I158](../BACKLOG.md) OAuth PKCE + Keychain storage
- [ADR-0049](./0049-eliminate-python-runtime.md) Rust-native Google API client

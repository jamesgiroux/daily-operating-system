# Claim Receipt Privacy Snapshot Fixtures

Golden JSON fixtures for the DOS-341 privacy matrix (W1 §5.9).

Each fixture documents the expected `ClaimReceipt` shape for a
`(audience × sensitivity × claim_type)` triple. The in-module test
`assert_keys_subset` (in `src/services/claim_receipt/privacy.rs`)
exercises the JSON-key allowlist contract per AC-341.10.

## File naming

```
<audience>__<sensitivity>__<claim_type>.json
```

- `audience` ∈ { `user_tauri`, `agent_mcp`, `activity_log`, `lint` }
  (no fixture for `operational_audit_storage` — non-disclosure tag)
- `sensitivity` ∈ { `public`, `internal`, `confidential`, `user_only` }
- `claim_type` reflects the claim's `claim_type` column

## Special fixtures

- `account_health_story_derived_from_confidential.json` —
  composed-claim drop scenario (correctness F5 / AC-341.10):
  a Public claim whose `derived_from` chain includes a Confidential parent.
  The MCP audience MUST drop the entire receipt.

## Field allowlists (source of truth: `privacy.rs` consts)

| Audience | Allowlist constant |
|---|---|
| UserTauri | `USER_TAURI_ALLOWED_FIELDS` |
| AgentMcp | `AGENT_MCP_ALLOWED_FIELDS` |
| ActivityLog | `ACTIVITY_LOG_ALLOWED_FIELDS` |
| Lint | `LINT_ALLOWED_FIELDS` |
| OperationalAuditStorage | `&[]` (non-disclosure) |

## CI lint cross-reference

`scripts/check_audit_disclosure_allowlist.sh` extends the DOS-340 §5.8
boundary lint to forbid any direct read of `maintenance_audit` from
`services::claim_receipt::*`. The privacy module is allowed to *write*
assertions through this audience but never to *surface* raw rows
(AC-341.11).

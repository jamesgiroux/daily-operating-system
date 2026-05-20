# DOS-340 — Receipt boundary fixture snapshots

One serialized `ClaimReceipt`-shaped projection per
(`SurfaceContext` × `ClaimSensitivity`) cell. The integration test
`claim_receipt_boundary_snapshots.rs` loads every JSON file in this
directory and asserts that every key is present in
`services::claim_receipt::boundary::RECEIPT_ALLOWED_FIELDS`.

This is AC-340.1 + AC-340.3 + AC-340.5 in fixture form. Customer-facing
data is generic per project rule "no customer-specific data in source
code": labels are `subsidiary.com`, `parent.com`, etc.

| Surface           | Public | Internal | Confidential | UserOnly |
|-------------------|--------|----------|--------------|----------|
| actions_work      | Y      | Y        | Y            | Y        |
| entity_detail     | Y      | Y        | Y            | Y        |
| daily_briefing    | Y      | Y        | Y            | Y        |
| meeting_detail    | Y      | Y        | Y            | Y        |
| mcp               | Y      | Y        | Y            | Y        |

BLOCK

**Actionable Findings**
- The plan puts persistence behind a `Read` ability. Packet lines 127-132 say `score_salience` writes `salience_factors`; lines 180-188 register it as `category = Read`; line 247 explicitly accepts “Scoring writes in a Read ability.” Current runtime treats read handles as read-only, and the ability macro blocks Read/Transform abilities from mutating services. Split this into:
  - a pure `score_salience` Read ability that computes/returns salience without DB writes, or reads latest stored salience;
  - a separate Live-only maintenance/service recompute path that writes `salience_factors`.
  W2-B can call the maintenance path for trigger-driven persistence.

- The proposed `SalienceReadHandle` is the wrong boundary for a writer. The packet models it after `claim_receipt`, `workspace_graph`, and `list_open_loops`, but those are read handles; existing write-capable seams are named/typed as commit or maintenance handles. Use an explicit `SalienceMaintenanceHandle`/write handle if persistence remains ability-routed.

- The ability descriptor is incomplete for the current macro. The packet omits `allowed_modes` and `version`; the macro requires `allowed_modes`. Amend the plan with a full `#[ability(...)]` shape. If the ability remains pure read, use `allowed_modes = [Live, Evaluate]` or `[Live, Simulate, Evaluate]`; if it writes, it should not be a Read ability exposed through MCP as currently described.

- Migration v270 must seed weights, not just create tables. The packet says live computation loads `salience_factors_weights`, but the shown SQL only creates the table. Add deterministic seed rows for all ten factor kinds from the wave table, an idempotent seed strategy, and a migration test asserting count=10 and sum=`1.0 +/- 0.0001`.

- Novelty/UserFit are under-specified against the available substrate. Current repo has durable canonicalization evidence/decisions, but no public read-only “recent claim corpus vector similarity” API that W1-B can use without touching `services::claims` internals. Amend factor rules to use only existing durable rows (`canonicalization_decisions`, `claim_semantic_evidence`, same-subject claim metadata) and return `None` when unavailable. Do not add a new vector primitive or modify claims core for W1-B.
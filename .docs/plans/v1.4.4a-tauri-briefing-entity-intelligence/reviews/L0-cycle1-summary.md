# v1.4.4a Tauri Briefing Entity Intelligence - L0 Cycle 1 Summary

Date: 2026-05-23
Plan: `.docs/plans/v1.4.4a-tauri-briefing-entity-intelligence-waves.html`
Final status: APPROVED after cycle-1 folds

## Panel

| Lane | Initial verdict | Post-fold verdict |
|---|---:|---:|
| Adversarial document review | APPROVE-WITH-CONDITIONS | APPROVE |
| Engineering feasibility | APPROVE-WITH-CONDITIONS | APPROVE |
| Security lens | APPROVE-WITH-CONDITIONS | APPROVE |
| Coherence / consult | BLOCK | APPROVE |

## K-in

Reviewed prior substrate and security decisions before scoring:

- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`
- ADR-0093 prompt-injection hardening
- ADR-0106 prompt fingerprinting
- ADR-0108 provenance rendering and privacy
- ADR-0111 surface-independent ability invocation
- ADR-0130 surface-independent composition contract
- v1.4.4 master wave packet and W1 substrate context
- Current source anchors for `get_entity_intelligence`, `get_daily_briefing`, `prepare_meeting`, Tauri `invoke_ability`, and dashboard/meeting render paths

## Required Folds Landed

1. Tauri-freeze exception: v1.4.4a is a scoped exception for existing Tauri surfaces, limited to data sourcing, actor-filtered trust/provenance rendering, and prompt-input plumbing. No new Tauri routes, visual patterns, UI features, or reskinning.
2. Main v1.4.4 packet amendment: the Tauri-freeze exception is mirrored into `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` and summarized in `.docs/plans/v1.4.4-waves.md`.
3. Cache authority: `prep_context_json` and `prep_frozen_json` are cached display snapshots only. Current envelopes are authoritative for new prompt input and new claim-backed rendering.
4. Prompt authority: only the Rust prompt-safe projection built from `EntityIntelligenceEnvelope` after the centralized `services::claims` prompt-channel sensitivity gate may enter canonical prompt JSON.
5. Actor/exposure matrix: W0 freezes `allowed_actors`, `required_scopes`, `mcp_exposure`, and `client_side_executable` for touched abilities. `get_daily_briefing` target authority is User-only unless main W3/sub-L0 separately approves SurfaceClient execution.
6. Existing non-Tauri exposure: `get_entity_intelligence` output changes require User/MCP/SurfaceClient golden fixtures for data shape, rendered text, provenance redaction, diagnostics omission, and source-id masking.
7. Audit redaction: Suite S now covers app logs, MCP audit rows, SurfaceClient/local-loopback audit, PTY errors, diagnostics, and structured telemetry. Allowed audit payloads are only invocation ids, hashes, counts, byte sizes, enum states, source classes, and redaction flags.
8. `prepare_meeting` composition: W3 must update descriptor, provenance, and prompt fixtures so `get_entity_intelligence` or a named projection ability is the claim-backed composed source.
9. Rendered provenance boundary: Tauri UI must render user-visible trust/provenance from `AbilityResponseJson.rendered_provenance`; DTO-local provenance is refs/counts/bindings unless passed through actor/surface rendering.
10. ADR-0130 boundary: this is a temporary envelope-rendered Tauri consumer exception. New substrate-authored composed output must use `Composition`.
11. Live Daily Briefing gate: W1 must prove a populated live Tauri `invoke_ability("get_daily_briefing", renderSurface="tauri_briefing_prep")` fixture with the daily-readiness reader registered and complete outer provenance attribution.
12. Suite P caps: W0/W1 must freeze route latency budgets, DB lock hold threshold, maximum meetings/entities processed before pagination, maximum frontend ability calls per route refresh, and representative fixture size.
13. Meeting Detail seam: W3 must choose either a `get_meeting_intelligence` envelope adjunct or a separate `useMeetingEntityIntelligence(meetingId)` hook, with merge precedence and failure behavior named.
14. Workspace-memory adjacency: v1.4.5 file-ingestion substrate is adjacent producer work, not owned by this lane.

## Final Verdict

L0 is approved for W0 issue authoring and implementation planning. W1 cannot start until W0 records the Tauri-freeze exception in Linear, resolves `get_daily_briefing` actor descriptor drift, freezes no-bypass/prompt/actor/performance matrices, and captures the required route baselines.

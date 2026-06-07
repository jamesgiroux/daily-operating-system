# DOS-511 / W6 L0 Packet - Transcript Claims, Quote Source Typing, and Provenance Readiness

- **Version:** v1.4.9 - W6 transcript substrate
- **Primary issue:** [DOS-511](https://linear.app/a8c/issue/DOS-511)
- **Related issues:** DOS-343, DOS-327, DOS-338, DOS-628, DOS-832
- **Author date:** 2026-06-03
- **Tier:** Tier 3 markdown-only
- **Trust topology:** Local-to-local single-user for Tauri/file surfaces. Transcript content is still hostile input for prompts and private content for logs. MCP egress, if a transcript-backed claim becomes MCP-readable through W5 tools, remains subject to ADR-0125 sensitivity gating.
- **Scope tier:** Wave-scope claim/provenance producer, source typing, trust recomputation, prompt-boundary, and read-surface proof. L0 requires `/codex challenge` or project-approved equivalent, `ce-feasibility-reviewer`, `ce-security-lens-reviewer`, and mandatory K-in. Cycle 1 routed `ce-data-migrations-reviewer` for source/corroboration storage risk and `ce-performance-reviewer` for reprocessing scope. Any later bulk/historical mode must return to L0 with performance and data/migration review.
- **Status:** L0 approved on 2026-06-03 CDT after adversarial, feasibility, security-lens, routed specialist, and K-in cycles reached unanimous pass on the current packet text.

---

## Section 0 - Origination, Release-Valve Role, and Non-Negotiables

**Origination class:** Extension. W6 turns an existing transcript extraction pipeline into a first-class claim producer with provenance, trust, correction, and source typing semantics. The current pipeline already extracts useful outcomes, but much of the output lands in capture/read-model tables or side tables instead of the claim substrate.

**2026-06-07 steering override:** James moved W6 implementation into the v1.4.9 completion path. The original release-valve contract below remains historical L0 context only; the current v1.4.9 tag gate requires DOS-511 AC2-AC23 implementation, AC-bounded L2, PR, and integrated L3 proof.

**Original release-valve contract, superseded for v1.4.9 tag gating:** W6 was not a v1.4.9 tag gate. The original wave plan said DOS-511 closed when this L0 packet passed review, independent of whether DOS-343/DOS-327 code landed in v1.4.9 or slipped to v1.5.0. W6 claims fed DOS-338 additively when they landed; DOS-338 still had to prove the correction loop on existing claim types without waiting for transcript claims.

**Headline contract:** A processed meeting transcript can produce claim-substrate rows whose assertion, source, source-as-of, sensitivity, trust band, provenance, correction path, and surface behavior are indistinguishable from other first-class DailyOS claims. Quote Wall readiness is source/provenance readiness over those claims, not a second authority layer.

**Non-negotiables:**

- Transcript-derived claim writes go through `services::claims::commit_claim` or a service wrapper that calls it. New transcript claim producers must not write `intelligence_claims`, `claim_feedback`, `claim_corroborations`, or `claim_contradictions` directly.
- Transcript claims are point-in-time evidence from a meeting. Do not treat a transcript statement as a current state claim unless a separate derived-state producer explicitly creates a state claim with provenance back to the point-in-time transcript evidence.
- One transcript file/source is one source for trust and corroboration. Multiple quotes, chunks, speakers, or extraction phases from the same transcript must not inflate independent-source confidence.
- Quote Wall consumes quote-bearing claims and rendered provenance. It must not create a raw quote table, display unprovenanced snippets, or infer trust from quote presence.
- Raw transcript text, prompt text, and model response text must not be emitted to logs, audit sidecars, diagnostics, screenshots, fixtures, or CI output.
- Existing capture tables, meeting transcript metadata, role-change rows, dynamics rows, and captured commitments remain downstream artifacts/read models until explicitly migrated. They are not sufficient proof that W6 shipped the claim substrate.
- W6 L1 does not perform historical or bulk transcript backfill. It is limited to bounded single-transcript fresh intake and explicit single-meeting reprocess. Bulk/provider-wide backfill requires a new L0 review with performance and migration/data reviewers.
- W6 L1 uses conservative transcript trust behavior: transcript-backed claims do not write transcript-source corroboration rows and do not gain positive trust reinforcement from transcript volume. A storage/trust migration for source-ref-aware corroboration is a later L0 decision.

### Section 0.1 - Branch and Authority Notes

This packet was authored on `codex/v1.4.9-w6-dos511-l0` from `public/dev`. Current code has:

- `processor/transcript.rs`, which runs three local AI extraction phases and writes meeting summaries, actions, captures, champion health, role changes, commitments, and dynamics.
- `services/mutations.rs`, which wraps some transcript writes (`persist_transcript_outcomes`, `persist_transcript_metadata`, `persist_key_advocate_health`, `replace_transcript_outcome_captures`) but still writes capture/read-model data, not claims.
- Legacy `DIRECT_DB_ALLOWED` transcript writes for dynamics, champion health, role changes, commitments, and capture dual-writes.
- `services::claims::commit_claim`, a closed-registry claim writer that validates claim type, subject kind, actor class, temporal scope, sensitivity, dedup identity, supersession, and tombstone pre-gates.
- `abilities_runtime::abilities::claims::ClaimType`, where reusable claim types already include `meeting_event_note`, `meeting_change_marker`, `risk`, `win`, `entity_risk`, `entity_win`, `value_delivered`, `open_loop`, `commitment`, `stakeholder_engagement`, and `stakeholder_assessment`.
- `WorkspaceFileKind::{GranolaTranscript, QuillTranscript}` and workspace claim ingestion patterns using `source_ref = workspace_file:<opaque-id>` and `data_source = workspace_file:<kind>`.
- `claim_corroborations` currently coalesces by `(claim_id, data_source)`, not concrete transcript `source_ref`. That is a known W6 blocker, not an implementation detail.

L1 must rebase onto the latest W1-W5 authority before implementation and reconcile any changed claim registry, feedback, MCP, or source lifecycle contracts.

### Section 0.2 - L0 Cycle 1 Decisions

Cycle 1 failed the draft packet on concrete blockers that are now W6 contracts:

1. **One-transcript-one-source trust chooses the conservative path.** W6 does not modify `claim_corroborations` and does not let transcript sources positively reinforce trust. Transcript claims can render cautiously from their own provenance and can later be corroborated by independent non-transcript sources.
2. **No bulk or historical backfill in W6.** The claim producer runs only for bounded single-transcript fresh intake and explicit single-meeting reprocess. Provider-wide sync/backfill, historical migrations, or batch reprocessing return to L0 with a rollout/performance plan.
3. **Provider-to-workspace identity consumes current substrate.** The canonical `source_ref` is the workspace lifecycle `file_id` for the persisted transcript destination. Provider ids/content hashes are duplicate-detection inputs and provenance metadata, not primary source refs. Temp paths and raw provider ids never become source identity.
4. **Model output is hostile when reused.** Phase-to-phase model output is treated as untrusted derived content under ADR-0093; later prompts must wrap/sanitize earlier phase output the same way they wrap transcript text.
5. **Error/result messages are observability surfaces.** Internal result records, processing logs, and audit sidecars must use stable codes/counts/lengths only, never raw model output or transcript snippets. User-visible toasts, sync states, and rendered errors must use ADR-0083 product vocabulary, with internal codes kept out of visible text.
6. **Runtime proof must be end-to-end.** W6 proof starts from synthetic transcript input processed through the transcript pipeline into the claim service and then into runtime surfaces. A manually inserted synthetic claim is not sufficient.

---

## Section 1 - Existing Substrate W6 Must Consume

### Section 1.1 - Transcript Processor and Read Models

The existing transcript processor is useful but not enough:

- Phase 1 extracts summary, discussion, analysis, actions, and decisions.
- Phase 2 extracts wins, risks, decisions, sentiment, and champion health.
- Phase 3 extracts interaction dynamics, commitments, and role changes.
- Reviewed outcomes replace win/risk/decision captures through `replace_transcript_outcome_captures`.
- `persist_enriched_transcript_data` still writes several artifacts directly through `ActionDb` or `conn_ref`.
- Current phase audit/log calls write raw phase output or a raw preview. ADR-0120 forbids raw user content, prompt text, and response text in logs. W6 must remove or sanitize these touched paths before promoting transcript output into claim substrate proof.

Treat these tables as existing read models:

- `meeting_transcripts`: transcript path, processed-at, summary, and intelligence state.
- `captures`: win/risk/decision/commitment rows with optional `evidence_quote`, `speaker`, urgency, impact, and subtype metadata.
- `meeting_interaction_dynamics`, `meeting_champion_health`, `meeting_role_changes`, and `captured_commitments`: structured transcript artifacts.

W6 may continue to populate those artifacts for backward compatibility, but the W6 proof must be claim-backed.

### Section 1.2 - Claim Registry and Commit Writer

`commit_claim` already enforces the core W6 rules:

- The claim type string must be in the closed `ClaimType` registry.
- The subject kind must be allowed for the claim type.
- The actor class must be permitted by claim metadata.
- Defaults for temporal scope and sensitivity come from registry metadata unless a proposal explicitly overrides them.
- Dedup identity is based on content hash, canonical subject, claim type, and field path.
- Tombstone pre-gates, supersession rules, contradiction/fork behavior, and immutable-column checks live in the service.

W6 therefore should not invent a transcript claim table. It should construct `ClaimProposal` values and call the service.

Current registry implications:

- `meeting_event_note` and `meeting_change_marker` already default to `TemporalScope::PointInTime` and attach to Meeting.
- `risk`, `win`, `entity_risk`, `entity_win`, `value_delivered`, `open_loop`, and `commitment` can represent transcript-extracted business outcomes when the subject kind matches the registry.
- `stakeholder_assessment` is Confidential by default and person-scoped; it may be derived from transcript evidence but must not cross MCP.
- There is no standalone `quote` claim type. Quote Wall should render quote-bearing evidence attached to real claim types unless L1 returns to L0 with a registry addition.

### Section 1.3 - Source and Provenance Substrate

The source/provenance substrate already has enough shape for transcripts:

- `ClaimProposal` carries `data_source`, `source_ref`, `source_asof`, `observed_at`, `provenance_json`, `metadata_json`, temporal scope, and sensitivity.
- `SourceAttribution` carries `DataSource`, identifiers, `observed_at`, optional `source_asof`, evidence weight, scoring class, and synthesis marker.
- Workspace ingestion already converts workspace proposals into claims with `source_ref = workspace_file:<opaque-id>`, `data_source = workspace_file:<kind>`, serialized source attribution, and metadata carrying workspace file id/kind.
- Existing transcript kinds cover Quill and Granola. Generic meeting-scoped transcript files need an approved mapping before W6 code ships: either a new generic transcript workspace kind with tests, or a documented fallback that still preserves a single opaque transcript source identity.

W6 must make the transcript source identity stable and opaque. A quote offset, chunk id, extraction phase, or speaker must not become the primary source id because that would turn one transcript into many artificial sources.

Canonical transcript identity for W6:

1. If the transcript is workspace-backed, `source_ref` is `workspace_file:<file_id>`. The file id comes from the workspace lifecycle/source ledger for the persisted transcript destination, not from a temp import path or caller-supplied source path.
2. The current workspace lifecycle substrate is path-backed. W6 consumes it rather than inventing a provider source table: L1 must create or look up the lifecycle row for the canonical persisted transcript destination before committing claims, then use that lifecycle `file_id` as the source identity.
3. If a provider transcript enters before a workspace lifecycle row exists, L1 first materializes or resolves the canonical persisted transcript destination. Provider ids, meeting ids, and content hashes may be stored only as normalized/hash metadata for duplicate detection and provenance; they do not replace the canonical `source_ref`.
4. Generic transcript imports must get an approved workspace file kind before claim commit. Do not store raw source, temp, or destination paths in `source_ref`, provenance, logs, proof artifacts, or result messages.
5. Same workspace file id resolves to the same transcript source. Same provider + same provider transcript id/content hash should resolve to the same canonical workspace file before commit; if W6 cannot prove that mapping for a provider path, the producer must skip claim commit with a structured warning rather than creating a second source.

### Section 1.4 - Surface Consumers

The W6 proof must use existing intelligence-loop consumers:

- `get_entity_intelligence` and the account/project overview projections already read claim-backed entity context and render trust/provenance data.
- `get_daily_briefing` composes daily readiness, meeting prep status, and entity intelligence. W5 decides MCP exposure; W6 only needs app/runtime parity proof that transcript claims reach briefing inputs.
- `build_intelligence_context()` and `gather_account_context()` are runtime consumers for account/customer intelligence context. W6 L1 must include them in the runtime-wide trust/provenance audit or prove they do not consume the affected transcript-backed claim classes on the implementation base.
- Quote Wall is source/provenance readiness. If no current Tauri Quote Wall surface exists on the implementation base, W6 must prove the projection contract with a renderer/component fixture or a typed block payload, not by reviving an archived WordPress implementation.
- Existing runtime readers must parse transcript claim `data_source` strings into the structured provenance source type. Falling back to `DataSource::Other` for `workspace_file:quill_transcript`, `workspace_file:granola_transcript`, or the approved generic transcript kind is not W6-compliant.

---

## Section 2 - Chosen Architecture

### Section 2.1 - Service-Owned Transcript Claim Producer

Add a service-owned producer, for example `services::transcript_claims`, with a narrow public entry point:

```text
commit_transcript_claims(ctx, db, TranscriptClaimBatch) -> TranscriptClaimCommitReport
```

The exact Rust type names are L1 decisions, but the boundary is fixed:

- Input is parsed, structured transcript extraction output plus meeting/source metadata. The service does not receive full raw transcript text or raw model output. It may receive bounded, verifier-produced quote snippets and source locators after exactness checks pass.
- The service resolves canonical subject refs for account/project/person/meeting through existing services or typed inputs. Unsupported or ambiguous subjects are dropped with structured warnings.
- The service maps each extracted item to a registered claim type or returns an explicit unsupported-type warning. Unknown free-form claim types are not passed into `commit_claim`.
- The service constructs provenance with source attribution, transcript source identity, extraction phase id, prompt template id/version or parser version, and field-level quote/evidence pointers where available.
- The service preflights every accepted proposal against active claims by dedup identity, canonical/semantic match, `source_ref`, and lifecycle state before calling `commit_claim`. It calls `commit_claim` only for proposals that do not match an existing active claim under the duplicate/corroboration path.
- If a transcript proposal matches an existing claim, the W6 conservative path returns a structured duplicate/no-op or review warning and does **not** call `commit_claim`, because current `commit_claim` would route the match into `corroborate_in_tx` and inflate transcript-source trust. `CommittedClaim::Reinforced` is not an allowed W6 transcript producer outcome.
- If L1 wants transcript evidence to attach to an existing claim as positive corroboration, it must return to L0 with a writer option or source-ref-aware corroboration migration.
- The service returns counts, claim ids only where allowed for the caller, warnings, and recompute/invalidation status.
- The service emits or enqueues the same signal/invalidation family used by other claim producers. No hidden refresh side effects.

The transcript processor calls this service after phase parsing/review, not inside prompt construction. Existing capture writes can continue as read-model compatibility, but they are not the source of truth for W6 claims.

W6 L1 scope is bounded:

- fresh single-transcript intake may call the producer after successful phase parsing/review;
- explicit single-meeting reprocess may call the producer after building the reprocessing manifest in §2.4;
- provider-wide sync, historical backfill, migration-time transcript replay, and background batch reprocessing do not call the W6 producer in this packet.

If L1 needs any bulk/historical mode, it returns to L0 with batch size, concurrency, dry-run counts, checkpoint/resume, maximum transcripts/claims/quotes per run, quote-verifier cost bounds, recompute/invalidation coalescing, stop conditions, and `ce-performance-reviewer` approval.

### Section 2.2 - Claim Type Mapping

W6 starts with the existing registry:

- Meeting-scoped notes, decisions, and direct observations: `ClaimType::MeetingEventNote` on the Meeting subject, `TemporalScope::PointInTime`.
- Role/status movement observed in the meeting: `ClaimType::MeetingChangeMarker` on the Meeting subject, `TemporalScope::PointInTime`. A separate state producer may later emit `stakeholder_role` on Person if approved.
- Account/project wins and risks: `win`, `risk`, `entity_win`, or `entity_risk` on the relevant account/project/person subject when the registry permits it, explicitly `TemporalScope::PointInTime` for direct transcript evidence.
- Value statements: `value_delivered` when the subject and wording match the registry. Otherwise use `meeting_event_note`.
- Open loops or commitments: `open_loop` for entity-or-meeting work items; `commitment` only when the subject is an Account because the registry currently pins it to Account.
- Stakeholder assessments: only if the evidence is specific, person-resolved, and sensitivity is at least the registry default. These do not satisfy MCP parity and must be covered by sensitivity tests.

Do not add `transcript_quote`, `quote_wall_quote`, or another quote-only claim type in L1 unless reviewers approve a registry change that names allowed subjects, temporal scope, sensitivity, freshness class, commit policy, actor class, surface placement, and feedback behavior.

### Section 2.3 - Subject Resolution Matrix

Transcript claim producers must not use the current "first linked entity" fallback as claim authority. L1 must implement and test this matrix:

| Extracted item | Allowed subject | Resolution rule | Ambiguous/unknown behavior |
| --- | --- | --- | --- |
| Meeting summary, discussion note, decision observed in the meeting, role-change event | Meeting | Use the meeting id from the transcript intake context | Commit as `meeting_event_note` or `meeting_change_marker`; no entity fallback needed |
| Account/project win, risk, value, open loop | Account or Project | Use exactly one resolved entity from meeting context, explicit entity mention, or entity-linking result above the approved confidence threshold | Downgrade to Meeting-scoped `meeting_event_note` with structured warning, or drop; never choose first linked entity |
| Person-scoped engagement or assessment | Person | Speaker/attendee name maps to exactly one person id in the meeting/account context | Drop person-scoped claim and record warning; optionally keep a Meeting-scoped note without person attribution |
| Commitment | Account only, per current registry | Commit only when exactly one Account subject is resolved | Use `open_loop` on Meeting or drop with warning; do not force an Account |
| Multi-entity parent meeting segment | Meeting or explicitly resolved child entity | Segment text must identify the child entity and resolver must confirm it | Meeting-scoped note only; no child entity claim |

Subject resolution warnings are structured producer output and test evidence. They must not include raw transcript snippets or names from real data in logs/CI.

### Section 2.4 - Temporal and Source Semantics

For every transcript claim:

- `temporal_scope = PointInTime`.
- `source_asof` is the meeting occurrence time, preferably meeting end time; if no end time exists, use meeting start time and include a structured `source_asof_fallback = meeting_start` warning in metadata/provenance.
- `observed_at` is the processing/commit time.
- `metadata_json` carries `occurred_at`, `meeting_id`, `transcript_source_kind`, `transcript_source_ref`, extraction phase, parser/schema version, and quote metadata when present.
- `provenance_json` carries source attribution and field attribution. It does not carry raw transcript text.
- Claims extracted from the same transcript share the same transcript `source_ref`.

Reprocessing the same transcript source must be idempotent and manifest-driven. Before committing, the producer loads existing transcript-backed claims for the same `source_ref`, compares proposed items by the existing claim dedup identity plus source-ref/source-as-of metadata, and returns a `TranscriptReprocessManifest` or equivalent structured report:

- unchanged proposal: no-op; do not add corroboration volume;
- new proposal: commit through `commit_claim`;
- changed assertion for the same source/dedup family: fork, supersede through a service-approved path, or surface a review warning; never overwrite immutable assertion/source fields;
- missing prior proposal: retain the existing claim unless a reviewed service path tombstones/suppresses it; never delete just because a new model run omitted it;
- tombstoned/dismissed/contradicted claim: do not resurrect; report the skipped proposal with a structured warning;
- feedback and contradiction edges remain attached to the existing claim lifecycle and are not dropped during rerun.

The manifest contains claim ids only in internal proof contexts where allowed, and no raw transcript snippets, names, paths, provider ids, or model output.

Every transcript-mapped claim type whose registry default is `State` must explicitly set `temporal_scope = Some(TemporalScope::PointInTime)` in the `ClaimProposal`. L1 tests must assert this for every non-meeting claim type W6 maps.

### Section 2.5 - One-Transcript-One-Source Trust Rule

Trust computation must respect ADR-0126 source diversity:

- A transcript source may contribute many claims, but it is still one source.
- Quote fragments, speakers, extraction phases, chunks, and prompt attempts are evidence locations inside the source, not independent sources.
- Multiple imports of the same meeting transcript from the same provider collapse to one source identity.
- The trust audit must verify that existing corroboration math does not count `workspace_file:quill_transcript` rows from the same meeting as independent evidence merely because quote refs differ.
- Current corroboration storage coalesces by `data_source`; that is insufficient for W6. W6 chooses the conservative no-migration path:
  - transcript proposals are duplicate-preflighted before `commit_claim`; matching proposals are skipped or routed to review and must not reach `commit_claim`'s `CommittedClaim::Reinforced` path;
  - transcript claim production does not insert, update, or count `claim_corroborations` rows whose corroborating source is the same transcript source or another transcript source from the same provider class;
  - transcript-backed claims render cautiously (`needs_verification` or the existing single-source trust band) until independently corroborated by a non-transcript source through the existing trust path;
  - multiple quotes, speakers, extraction phases, prompt attempts, reruns, and same-provider transcript repeats never raise independent-source confidence;
  - same-provider different transcripts may remain distinct in provenance/source identity, but they still do not positively reinforce trust in W6 unless a later source-ref-aware corroboration migration is approved;
  - if L1 wants transcript-to-transcript positive corroboration, it must return to L0 with a storage/trust migration that keys reinforcement by concrete `source_ref` or equivalent source edge id, plus registered migration slot, backfill/rollback, and data-migration review.
- No W6 migration is required for the approved conservative path. The reserved W6 migration block remains unused unless a later approved packet selects the storage/trust fix.
- Tests must include three cases: same transcript with multiple quotes, same provider with two different transcripts, and different source classes corroborating the same claim.

Do not seed arbitrary trust scores to avoid cautious rendering. Transcript claims render through the same trust compiler/recompute path as other producers.

### Section 2.6 - Quote Verification and Quote Wall Readiness

Quote Wall readiness means:

- Quote-bearing transcript claims include exact, bounded evidence snippets only when a verifier confirms the normalized quote exists in the canonical transcript source.
- Quote verification runs before the claim service boundary. It may inspect raw transcript text in memory, but it returns only a bounded snippet, source locator, and verification status to the service.
- If exact verification fails, W6 must either drop quote metadata or classify the text as non-verbatim model-supplied evidence. It must not display it as a quote.
- Quote snippets live in structured metadata/provenance fields such as `evidence_quote`, `speaker`, `quote_kind`, `source_offset` or another approved locator. The claim text remains the assertion.
- The rendered source label is display-safe: source class/kind, meeting date or source-as-of, trust band, redaction state, and provenance affordance. It never exposes raw local paths, raw provider ids, raw prompt text, or hidden claim ids by default.
- App-local surfaces may show human-readable meeting labels under the local single-user topology. MCP/published surfaces use ADR-0108 rendering and ADR-0125 sensitivity ceilings.
- If no quote is present, the claim can still be valid. Quote presence improves explainability, not trust by itself.
- Fabricated or paraphrased quotes are forbidden. If the model produces an unsupported quote, drop the quote metadata and keep/drop the claim based on extractor confidence rules.

Quote Wall readiness uses a typed projection payload, not ad hoc metadata reads. The exact type name is an L1 decision, but the payload must include: claim reference usable by the local app, claim type, assertion text or approved display text, exactness status, bounded `evidence_quote` when policy-allowed, optional display-safe speaker label, source locator, source-as-of, transcript source kind, trust band, sensitivity/redaction state, provenance envelope, and correction/feedback affordance. It must not include raw transcript text beyond the verified snippet, raw paths, raw provider ids, prompt/model output, hidden claim ids on non-app surfaces, or unbounded metadata JSON.

Quote sensitivity is the maximum of claim-type default sensitivity, meeting/source sensitivity, speaker/person sensitivity when known, and quote-content policy. Person assessments, employment/role judgments, sensitive customer/account claims, or quotes containing private notes default to at least the registry sensitivity and may be raised to Confidential/UserOnly. A quote-bearing claim can be committed without exposing the quote on a given surface.

### Section 2.7 - Runtime Provenance Rendering

W6 must wire transcript source typing through the runtime reader/render path:

- Parse `workspace_file:<kind>` claim data sources into `DataSource::WorkspaceFile { kind }` for Quill, Granola, and the approved generic transcript kind.
- Preserve a document/meeting source identifier in provenance so rendered output can say "workspace transcript source as of <date>" without raw paths or provider ids.
- `get_entity_intelligence`, daily briefing, and Quote Wall projection tests must fail if transcript-backed claims render as `DataSource::Other` or lose source-as-of.
- MCP-facing projections use W5 redaction and sensitivity policy before source detail leaves the app.

### Section 2.8 - Logs, Audit, and Prompt-Injection Boundaries

Transcript text is untrusted input for prompts and private content for observability:

- Continue using `wrap_user_data`, hostile-input preambles, and bounded model output parsing.
- Treat phase-to-phase model output as untrusted derived content. When Phase 1 output is fed into Phase 2 or Phase 3, or any model/parser output is reused in a later prompt, it must be wrapped/sanitized with the same ADR-0093 treatment as transcript text and must not be allowed to set authority fields.
- The model output cannot choose actor, subject authority, sensitivity, source authority, trust score, lifecycle state, or target service.
- Internal audit/log/result records contain stable codes, lengths, phase ids, schema versions, and counts only. They do not contain raw transcript snippets, claim text, meeting titles if avoidable, prompt text, response text, or model-output previews.
- User-visible transcript toasts, sync states, and rendered errors use ADR-0083 product vocabulary. They may describe the user-facing outcome, but must not display internal codes, raw transcript snippets, quote snippets, claim text, local paths, provider ids, prompt text, response text, model-output previews, or raw parse failures.
- Any retained diagnostic artifact with raw content must live in an encrypted, user-local content store with explicit render policy. It must not be a plaintext audit sidecar or CI artifact.

Known current leaks that W6 must explicitly close before code shipment:

- `write_audit_entry(..., "transcript-p1" | "transcript-p2" | "transcript-p3", ..., phase_output)` raw model-output sidecars.
- `log::info!("Phase N output ... {}", preview)` raw response previews.
- `TranscriptResult.message` and serialized frontend/sync failure state that include strings such as "Raw output: ...".
- UI toasts, Quill sync status, processing-log, or diagnostic rows that persist source/destination local paths, prompt text, response text, quote snippets, claim text, meeting titles, or raw parse failures.
- fixtures or proof artifacts that include real transcript snippets, provider ids, local paths, names, emails, account names, or meeting titles.

---

## Section 3 - Acceptance Criteria

**AC1 - L0 closure and current tag status.** DOS-511 L0 closes when this packet passes L0 review and the verdict is recorded on Linear. Per the 2026-06-07 steering override, W6 implementation no longer slips past the v1.4.9 tag: AC2-AC23 are now required in the version completion path.

**AC2 - Service-owned producer.** Transcript-derived claims are committed through a new or existing service wrapper that calls `services::claims::commit_claim`. No transcript code directly writes claim tables. The producer accepts only typed parsed items, verified quote snippets/locators, source identity, and resolved subjects; it does not accept raw transcript text or raw model output.

**AC3 - Registered claim types and resolved subjects only.** Every emitted transcript claim uses a registered `ClaimType` and passes subject-kind validation. Any new claim type requires registry metadata, tests, and L0-reviewed rationale. L1 must implement the subject-resolution matrix for none/one/multiple/conflicting account, project, person, and meeting subjects; first-linked-entity fallback is forbidden.

**AC4 - Point-in-time transcript evidence.** Direct transcript claims use `TemporalScope::PointInTime`, immutable text/source fields, and meeting occurrence time as `source_asof`. Derived current-state claims, if any, are produced by a separate approved derivation path. Tests must assert every transcript-mapped non-meeting claim with a registry default of `State` explicitly sets `TemporalScope::PointInTime` in its `ClaimProposal`.

**AC5 - Source identity is stable and singular.** Every claim from one transcript shares one opaque transcript `source_ref`, canonically `workspace_file:<file_id>`. Quote/chunk/speaker/phase identifiers are locators inside provenance, not independent source ids. Raw local paths, provider ids, and quote offsets never become primary source refs.

**AC6 - Source taxonomy and identity algorithm.** Quill and Granola transcript sources use the existing workspace transcript kinds where available. Generic transcript files get an approved source-kind mapping with freshness/rendering tests before shipment. L1 must define and test canonical transcript identity from the workspace lifecycle `file_id` for the persisted transcript destination, with normalized/hash provider and content identifiers used only for duplicate detection/provenance. Temp paths, raw source paths, raw destination paths, and raw provider ids are forbidden in `source_ref`, logs, result messages, proof artifacts, and rendered provenance.

**AC7 - One-source corroboration.** W6 ships the conservative no-migration trust path. Tests prove transcript proposals are duplicate-preflighted before `commit_claim`, duplicate/canonical-match transcript proposals do not return `CommittedClaim::Reinforced`, and multiple quotes, extraction phases, reruns, and same-provider transcript repeats do not insert/update transcript-source `claim_corroborations` rows or raise independent-source confidence. Same provider with two different transcript `source_ref`s must remain distinct in provenance but still not positively reinforce trust in W6. Independent non-transcript sources can corroborate normally.

**AC8 - Idempotent reprocessing.** Re-running transcript processing for the same source builds a reprocessing manifest and does not duplicate claims, resurrect tombstones, drop user feedback, delete omitted prior claims, create artificial corroboration, or overwrite immutable assertion/source fields. W6 L1 supports only fresh single-transcript intake and explicit single-meeting reprocess; bulk/historical backfill is out of scope.

**AC9 - Quote Wall readiness.** Quote-bearing claims carry exact bounded quote metadata, speaker when safely known, source-as-of, trust band, sensitivity, and rendered provenance. Quote Wall does not read raw transcript files or unprovenanced capture rows as authority. A quote can be marked exact only after verifier confirmation against the canonical transcript source.

**AC10 - No quote-only authority.** No standalone quote table or quote-only claim type ships unless a reviewed registry addition defines its semantics. Quote snippets explain claims; they do not replace claims.

**AC11 - Sensitivity mapping.** Transcript claim and quote sensitivity derives from claim type defaults plus meeting/source context, speaker/person context when known, and quote-content policy. Quote metadata may be redacted even when the underlying claim remains visible. `Confidential` and `UserOnly` transcript-backed claims do not cross MCP or publish-style surfaces.

**AC12 - Hostile-input guard.** Transcript source text and phase-to-phase model output cannot set actor, subject authority, sensitivity, source authority, trust, lifecycle, or tool routing. Tests cover prompt-injection attempts in transcript text and in reused Phase 1/2 model output before later prompts.

**AC13 - Observability privacy.** Observability and diagnostic surfaces in touched transcript paths stop logging, auditing, returning in result messages, displaying in toasts/sync/error states, or persisting in logs/audit/proof/diagnostic artifacts raw transcript text, raw prompt text, raw model response text, unverified quote snippets, claim text, local paths, provider ids, account names, person names, meeting titles, or raw parse failures. This does not prohibit policy-allowed, verifier-produced `evidence_quote` metadata or claim assertion/display text from being persisted and rendered through the claim substrate, runtime projections, and Quote Wall paths required by AC9, AC16, and AC21. L1 must explicitly remove/sanitize `transcript-p1/p2/p3` raw audit sidecars, `Phase N output` previews, `TranscriptResult.message` raw output strings, frontend transcript toasts, Quill sync failure state, and processing-log source/destination paths before W6 code ships. Tests or static checks prove internal logs/audit/result outputs use codes/counts/lengths only, and prove user-visible transcript toasts/errors use ADR-0083 product copy without visible internal codes or private payloads.

**AC14 - Legacy read-model compatibility.** Existing capture/metadata/dynamics/commitment outputs continue to work unless explicitly replaced. The W6 claim proof does not depend on capture rows as the source of truth.

**AC15 - Trust recomputation audit.** L1 inventories claim type metadata, producer path, trust inputs, recompute trigger, and surface behavior per the K-in solution on claim producers. The W6 conservative path must prove transcript-source volume does not positively reinforce trust. Missing recompute behavior renders as `needs_verification` or blocks shipment; arbitrary seeded trust is forbidden.

**AC16 - End-to-end runtime surface proof.** A synthetic transcript input flows through the transcript processor, quote verifier when applicable, `commit_transcript_claims`, `commit_claim`, and then appears in an entity-intelligence or daily-briefing path with source-as-of, trust band, sensitivity, and provenance rendered through the existing runtime, not a parallel SQL/prose shortcut. Runtime provenance parsing must render transcript `data_source` as `DataSource::WorkspaceFile { kind }` and must fail the test if it falls back to `DataSource::Other`. The runtime audit includes `build_intelligence_context()` and `gather_account_context()` or proves they do not consume the affected transcript-backed claim classes.

**AC17 - Correction loop proof.** A correction/dismissal/corroboration against a transcript-backed claim flows through the same `claim_feedback` path as other claims, updates lifecycle/trust/receipt state, and changes the follow-up surface.

**AC18 - Quote source typing proof.** A fixture with two quote-bearing claims from one transcript renders display-safe source typing and one-source trust behavior. A fixture with same-provider different transcripts proves distinct concrete source identity. A fixture with a disallowed sensitivity redacts or omits the quote as required.

**AC19 - No PII fixtures.** Tests, docs, proof bundles, screenshots, eval outputs, commit messages, and PR bodies use generic accounts, people, projects, meetings, and domains only.

**AC20 - Gates.** Before code ships, run focused transcript/claim tests plus the standard gate set required by AGENTS.md for code changes: `cargo clippy -- -D warnings`, `cargo test`, and `pnpm tsc --noEmit`. For this docs-only L0 packet, run markdown/diff hygiene and sensitive-content scans.

**AC21 - Quote exactness verifier.** L1 implements a verifier that normalizes model-supplied quote candidates and confirms them against the canonical transcript source before persisting `evidence_quote` or Quote Wall metadata. Failed verification stores no exact quote; if retained at all, it is labeled as non-verbatim evidence text.

**AC22 - Runtime provenance parser.** L1 updates every W6-consuming runtime reader/projection path so transcript source kinds survive claim row -> runtime provenance -> rendered surface. Tests cover Quill, Granola, and the approved generic transcript kind.

**AC23 - No bulk backfill.** W6 implementation does not run claim production for historical corpora, provider-wide sync, migration-time replay, or background batches. Any bulk mode requires a new L0 packet with performance-review approval, batch/concurrency caps, checkpoint/resume, dry-run counts, quote-verifier cost bounds, recompute/invalidation coalescing, stop conditions, and rollback.

---

## Section 4 - Test and Proof Plan

Minimum L1 proof bundle:

1. **Unit tests:** claim type mapping, subject validation matrix, source identity construction, source-as-of fallback, quote metadata bounding, quote exactness verification, and injection text unable to set authority fields.
2. **Service tests:** `commit_transcript_claims` commits through `commit_claim`, rejects unknown claim types, handles ambiguous subjects, respects tombstones, and produces idempotent same-source behavior.
3. **Trust tests:** duplicate/canonical-match transcript proposals are skipped or routed to review before `commit_claim` and never return `CommittedClaim::Reinforced`; same transcript with multiple quotes does not raise independent-source confidence; same provider with two concrete transcript sources remains distinct in provenance but does not positively reinforce transcript trust in W6; a second independent non-transcript source can still corroborate normally.
4. **Surface tests:** synthetic transcript input flows through the processor into entity intelligence / daily briefing with trust, provenance, sensitivity, source-as-of, and structured transcript source kind.
5. **Feedback tests:** typed claim feedback against a transcript-backed claim updates lifecycle/trust/receipt behavior.
6. **Quote tests:** Quote Wall projection renders exact bounded quote snippets only when verified against the canonical transcript and policy-allowed.
7. **Privacy tests:** logs/audit/result records for transcript processing contain no raw transcript, quote, prompt, response, claim text, local path, provider id, account name, person name, meeting title, or raw parse failure; user-visible transcript toasts/errors use ADR-0083 product copy without visible internal codes; tests cover the known `transcript-p1/p2/p3`, `Phase N output`, `TranscriptResult.message`, UI toast/sync failure, and processing-log paths.
8. **Reprocessing/performance tests:** single-transcript reprocess manifest preserves feedback/tombstones and does not duplicate claims or trigger transcript corroboration; tests assert bulk/historical entry points are not wired to W6 claim production.
9. **Fixture governance:** all test data uses generic entities and synthetic transcript text.

Suggested focused commands before full gates:

```bash
cargo test --lib transcript_claims
cargo test --lib processor::transcript
cargo test --lib claims::tests
cargo test --lib get_entity_intelligence
```

Exact test module names may change in L1; the proof bundle must name the commands actually run.

---

## Section 5 - Intelligence Loop Integration Check

1. **Claim model:** Yes. Transcript outcomes become first-class claims with registered claim types, explicit subject attribution, point-in-time temporal scope, sensitivity, lifecycle state, and feedback behavior. Legacy capture rows are read models, not the intelligence source of truth.

2. **Provenance and trust:** Every transcript claim carries `source_asof`, `observed_at`, `data_source`, `source_ref`, source attribution, extraction metadata, quote locator metadata when present, and trust recompute inputs. Trust bands come from the shared compiler/recompute path; W6 transcript sources do not positively reinforce trust through transcript-source corroboration.

3. **Signals and invalidation:** Claim commits and feedback use existing claim/service signals and invalidation. Transcript processing emits structured producer results; it does not silently mutate derived surfaces.

4. **Runtime and surfaces:** `get_entity_intelligence`, daily briefing, `build_intelligence_context()`, `gather_account_context()`, and Quote Wall projection/readiness consume transcript claims through the same runtime readers as other claims or are proven not to consume the affected classes. MCP sees only W5-approved, sensitivity-eligible transcript-backed claims.

5. **Feedback loop:** User corrections, dismissals, corroborations, contradictions, and source fixes flow through `claim_feedback`. Reprocessing respects tombstones, feedback, contradictions, supersession, and immutable source identity.

---

## Section 6 - K-In Research Summary

| Source | Relevance to W6 |
| --- | --- |
| `.docs/plans/v1.4.9-waves.md` | Defines W6 as transcript substrate and, after the 2026-06-07 steering override, a v1.4.9 version-tag gate; DOS-343/DOS-327/DOS-511 scope; point-in-time and one-source rules; proof requirement. |
| `.docs/decisions/0044-meeting-scoped-transcript-intake.md` | Transcript intake is meeting-scoped, immutable, and not reprocessed on briefing reruns. |
| `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md` | Claim temporal scope, sensitivity tiers, and registry metadata contract. Internal can cross MCP; Confidential/UserOnly cannot. |
| `.docs/decisions/0126-memory-substrate-invariants.md` | Claim immutability, feedback through `claim_feedback`, fork-not-winner contradiction handling, and source diversity over volume. |
| `.docs/decisions/0120-observability-contract.md` | No raw user content, prompt text, response text, or transcript fragments in logs. |
| `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md` | New claim producers require runtime-wide trust, provenance, recompute, and surface audit. |
| `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` | New prompt channels and MCP/output paths must compose centralized sensitivity gates instead of copying ad hoc checks. |
| `.docs/plans/abilities-runtime-producer-audit-2026-05-23.md` | Identifies `processor/transcript.rs` as producing transcript artifacts without a direct claim producer. |

K-in conclusion: W6 should not build a quote surface first. The missing substrate is a service-owned transcript claim producer with source identity, point-in-time temporal semantics, trust recompute, privacy-safe observability, and feedback parity.

---

## Section 7 - L0 Review Verdict

**Verdict:** APPROVE. DOS-511/W6 may move to L1 against this packet.

Final cycle approvals were recorded on the packet text that includes duplicate preflight before `commit_claim`, conservative no-transcript-corroboration trust behavior, path-backed `workspace_file` source identity, no bulk backfill, ADR-0083 user-visible copy separation, and AC13 observability/Quote Wall scoping.

| Lane | Final verdict | Notes |
| --- | --- | --- |
| `/codex challenge` | APPROVE | Final adversarial pass found no concrete L0 blocker after AC13 was scoped to observability/diagnostic surfaces while preserving verified claim/Quote Wall evidence. |
| `ce-feasibility-reviewer` | APPROVE | Current packet is implementable against existing claim writer, workspace file lifecycle, runtime reader, and transcript processor constraints. |
| `ce-security-lens-reviewer` | APPROVE | Prompt-boundary, raw-output, source identity, visible-copy, and observability privacy concerns are covered at plan altitude. |
| `ce-data-migrations-reviewer` / trust lane | APPROVE | W6 chooses the no-migration conservative trust path; duplicate/canonical matches must skip or route to review before `commit_claim` can return `CommittedClaim::Reinforced`. |
| `ce-performance-reviewer` | APPROVE | W6 remains bounded to fresh single-transcript intake and explicit single-meeting reprocess; historical/bulk backfill returns to L0. |
| `ce-product-lens-reviewer` / Quote Wall lane | APPROVE | Quote Wall readiness is projection over first-class claims, not a quote-only authority layer. |
| `ce-learnings-researcher` / K-in | APPROVE | Prior substrate and ADR conflicts are applied, including ADR-0083 product vocabulary and ADR-0120 observability privacy. |

Resolved L0 blockers:

1. W6 uses current path-backed workspace lifecycle rows for transcript source identity; raw paths and raw provider ids are never source authority.
2. W6 forbids transcript-source trust reinforcement and `CommittedClaim::Reinforced` producer outcomes by preflighting duplicate/canonical matches before `commit_claim`.
3. W6 excludes historical/provider-wide/bulk backfill until a separate L0 packet defines rollout, bounds, and performance controls.
4. Phase-to-phase model output is treated as hostile input under ADR-0093.
5. Internal logs/results/audit use stable shape data only; visible transcript toasts/errors use ADR-0083 product copy without visible internal codes.
6. AC13 privacy applies to observability and diagnostic surfaces, while policy-allowed verified `evidence_quote` metadata and claim display/assertion text remain valid through claim substrate, runtime projection, and Quote Wall paths.

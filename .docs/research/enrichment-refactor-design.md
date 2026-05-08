# DOS-241 Enrichment Refactor Design

Status: W0-A research spike design doc for v1.4.1.

Scope: no code changes. This document answers the DOS-241 acceptance criteria and the v1.4.1 W0-A gate questions about claim-based generation granularity and signal/job/claim ownership boundaries.

Sources read:

- `.docs/plans/v1.4.1-waves.md`, especially W0-A and the 2026-05-07 founder comment requiring per-surface mapping for briefing, meeting detail, entity detail, actions, and email.
- ADR-0102, ADR-0103, ADR-0104, ADR-0105, ADR-0110, ADR-0112, ADR-0113, ADR-0114, ADR-0117.
- Current enrichment-adjacent code paths in `intel_queue.rs`, `services/intelligence.rs`, `services/claims.rs`, `abilities/get_entity_context.rs`, `abilities/prepare_meeting/`, `meeting_prep_queue.rs`, `services/meetings.rs`, `enrichment.rs`, `prepare/email_enrich.rs`, `workflow/deliver.rs`, `processor/email_actions.rs`, `services/actions.rs`, `services/emails.rs`, `google_api/gmail.rs`.

## Recommendation

Refactor enrichment into four explicit stages:

1. **Acquire**: read raw source material through mode-aware providers and outbox-backed external calls.
2. **Synthesize**: run Transform abilities that return `AbilityOutput<ClaimProposalSet>` or surface-specific DTOs with complete provenance.
3. **Apply**: run Maintenance abilities that decide propose vs commit, write claims or read models, emit policy-governed signals, and audit the mutation.
4. **Read**: render surfaces from Read abilities and materialized read models. Surfaces do not call LLMs or bypass claims to mutate source-of-truth tables.

The key design choice is a **tiered gated commit policy**. AI-authored enrichment is proposed by default and only auto-commits when a claim type is marked low blast radius, has complete provenance, clears tombstone/user-override checks, has no unresolved contradiction, and passes trust thresholds from ADR-0114. This activates ADR-0113 for enrichment without forcing every deterministic cache refresh into a human inbox.

## A. Current-State Audit

| Pipeline | Current files | Trigger | LLM or external calls | Current mutations/signals | Pain point |
| --- | --- | --- | --- | --- | --- |
| Entity intelligence enrichment | `src-tauri/src/intel_queue.rs`, `src-tauri/src/services/intelligence.rs` | Manual refresh, queue item, content/calendar/onboarding/hygiene triggers | Glean provider or PTY/Claude, including dimension fanout | Writes `entity_assessment` projections, claim-shaped projection rows, stakeholder side writes, commitment bridges, relationship rows, disk snapshots, report invalidations, UI events, and signals | Transform, Maintenance, cache projection, and orchestration are coupled. Progressive dimension writes can make partial synthesis look authoritative. Provenance is incomplete at claim write time. |
| Glean entity enrichment | `src-tauri/src/intelligence/glean_provider.rs`, `src-tauri/src/intel_queue.rs` | Entity enrichment when remote context provider is Glean | Glean enrichment API, with PTY fallback | Same entity-intelligence write/finalize path as PTY | Glean-derived evidence can be laundered into AI-looking fields unless upstream source refs remain attached through claims. External retry/idempotency is not outbox-owned. |
| Progressive dimension enrichment | `src-tauri/src/intel_queue.rs` | Parallel dimension extraction | PTY/Claude per dimension | Calls progressive snapshot/claim projection and emits progress events before final composition | A partial Transform result can mutate durable knowledge before final validation, contradiction checks, and trust scoring finish. |
| Meeting prep and briefing refresh | `src-tauri/src/meeting_prep_queue.rs`, `src-tauri/src/services/meetings.rs`, `src-tauri/src/intelligence/lifecycle.rs` | Queue sweep, manual full refresh, relink/quality repair | Mechanical prep plus optional PTY prep enrichment; full refresh can first trigger entity enrichment | Writes `meeting_prep.prep_context_json`, `prep_frozen_json`, meeting intelligence state, prep-ready events | Cached prep is treated as durable briefing content. Full refresh is an orchestration that can chain entity mutations and meeting materialization without a claim boundary. |
| `prepare_meeting` ability | `src-tauri/src/abilities/prepare_meeting/` | Ability invocation | Provider-backed Transform, with `get_entity_context` child Read | No direct mutation; returns `MeetingBrief`; has draft claim mapping helper | This is the target shape, but claim draft application is not yet a separate Maintenance ability with ADR-0113 gating. |
| Entity context read | `src-tauri/src/abilities/get_entity_context.rs` | Entity detail and child abilities | None beyond Read services | No mutation; produces provenanced context from claims | This is the correct Read boundary, but legacy surfaces still read `entity_assessment` and enrichment snapshots directly in places. |
| Email ingestion and enrichment | `src-tauri/src/google_api/gmail.rs`, `src-tauri/src/prepare/orchestrate.rs`, `src-tauri/src/prepare/email_enrich.rs`, `src-tauri/src/workflow/deliver.rs`, `src-tauri/src/db/emails.rs` | Gmail refresh, background poll, daily prepare, retry failed emails | Gmail API, PTY/Claude for summary/sentiment/urgency/noise and batch enrichment | Upserts raw emails; writes contextual summary, entity link, sentiment, urgency, commitments/questions columns, enrichment state; emits email progress | Raw source cache, AI synthesis, retry state, and display columns share one table path. Some flows parse commitments into actions directly. |
| Email action extraction | `src-tauri/src/processor/email_actions.rs`, `src-tauri/src/services/emails.rs` | High-priority email body fetch or user promotion | Gmail body fetch, PTY/Claude extraction | Writes suggested `actions`; promotion writes tracked action; dismissals shadow-write email tombstone claims | Extracted actions can become rows without claim proposals, trust, field provenance, or explicit user/policy commit decision. |
| Actions and commitments | `src-tauri/src/services/actions.rs`, `src-tauri/src/services/commitment_bridge.rs`, `src-tauri/src/db/actions.rs` | User accept/reject/dismiss, entity enrichment bridge | None in action service | Mutates action status, rejection patterns, commitment bridge tombstones, and action signals | User lifecycle handling is strong, but AI-created suggestions arrive from mixed sources and should be rooted in claims/proposals before action rows become canonical. |
| Clay and Gravatar people enrichment | `src-tauri/src/enrichment.rs`, `src-tauri/src/clay/`, `src-tauri/src/gravatar/` | Background sweep or wake | Clay/Smithery, Gravatar, file/avatar IO | Updates sync state, person profile/cache/avatar files, emits profile discovery signal | External calls and local mutations are direct. Fetched profile facts need source provenance and idempotent outbox handling. |
| Transcript/file extraction | `src-tauri/src/processor/transcript.rs`, `src-tauri/src/processor/enrich.rs`, capture/meeting commands | Inbox file or meeting capture processing | PTY/Claude extraction | Writes artifacts, signals, captured commitments, actions, entity links | These are ingestion pipelines that feed enrichment. Extracted facts should enter as source-backed claim proposals, not side tables only. |
| Reports, briefing narratives, week narratives | `src-tauri/src/workflow/deliver.rs`, `src-tauri/src/reports/` | Daily execution, report generation | PTY/Claude synthesis | Writes report/briefing artifacts and invalidates caches | These are mostly Transform/Publish-adjacent artifacts. They must not back-write durable claims unless routed through Maintenance. |

Current cross-cutting failure classes:

- **Whole-object rewrites**: `IntelligenceJson`, meeting prep blobs, and email display columns are updated as snapshots, so one new fact can rewrite unrelated fields.
- **Transform/Maintenance conflation**: LLM or Glean output paths often compose, validate, persist, emit, and invalidate in one function.
- **User correction risk**: several paths have local protections, but the system lacks one universal "user claim wins unless explicit contradiction" gate.
- **Silent or partial authority**: progressive writes and best-effort side writes can leave a surface believing enrichment succeeded when downstream side effects failed.
- **Provenance gaps**: transitional claim projection currently has empty or thin provenance for enrichment-authored claims.
- **External-call durability gaps**: Gmail, Glean, Clay, Gravatar, and PTY calls are mostly direct calls with local retry state rather than ADR-0103 outbox jobs.

## B. Ability Category Mapping

ADR-0102 category split for the target architecture:

| Current pipeline | Read | Transform | Maintenance | Publish |
| --- | --- | --- | --- | --- |
| Entity intelligence | Gather entity context, source claims, raw signals, source cache rows | Produce entity claim proposals: risks, wins, current state, value delivered, stakeholder engagement, company context, open loops | Apply proposals, project claim read models, sync low-risk read models, emit claim-derived signals | None |
| Meeting briefing | Read meeting metadata, attendees, active claims, linked subjects | `prepare_meeting` produces a `MeetingBrief` and optional meeting claim proposals | Cache/materialize briefing; apply meeting claim proposals under gate | None unless a future send/share flow is added |
| Entity detail | `get_entity_context` reads canonical claims | Optional entity summary Transform for display-only narrative | None during read; only claim-application jobs mutate | None |
| Actions | Read active actions, claim proposals, open-loop claims, dismissal tombstones | Extract action/open-loop/commitment proposals from meetings, emails, transcripts, entity intelligence | Create suggested action rows only after proposal commit policy or user accept; apply dismiss/reject tombstones | External task-system sync would be Publish and is out of enrichment scope |
| Email | Read Gmail cache rows, email claims, linked entity context | Email triage Transform proposes summary, entity link, urgency, sentiment, noise verdict, questions, commitments, reply-needed claims | Upsert raw email cache, apply proposals, update read model columns, record dismissals/tombstones | Gmail archive/unarchive is Publish-like external side effect and must use ADR-0117 Pencil/Pen semantics |
| Clay/Gravatar | Read person candidates and provider cache | Normalize provider facts into person/profile claim proposals | Apply profile claims, avatar cache materialization, sync state | None |
| Transcript/file extraction | Read raw artifact and linked entity context | Extract meeting/event/action/entity claim proposals | Apply claims and derive read models/actions under gate | None |
| Reports and narratives | Read claims/read models | Synthesize report/briefing narrative DTO | Cache report artifact | Sending/exporting is Publish and outside enrichment |

### Per-Surface Consumer Mapping

| Surface | Current consumption shape | Target Read owner | Target Transform owner | Target Maintenance owner | Target surface contract |
| --- | --- | --- | --- | --- | --- |
| Briefing | Reads schedule, prep blobs, email summaries, actions, signals, daily narrative artifacts | Read abilities for day schedule, active actions, email triage read model, `prepare_meeting` outputs for meetings | Briefing narrative Transform may summarize already-provenanced inputs but cannot invent durable facts | Cache daily briefing artifacts and invalidate them when source claim versions change | Briefing is a composed read view. It may show proposed claims, but it does not make them active without a Maintenance apply step. |
| Meeting detail | Reads meeting metadata, `meeting_prep`, linked entities, captures, actions, transcript-derived data | Meeting detail Read ability over meeting metadata, active meeting claims, linked subject claims, active/proposed actions | Post-meeting and pre-meeting synthesis produce meeting claim proposals: topics, attendee context, open loops, changes, suggested outcomes | Apply meeting proposals, update `meeting_prep` as a read model, emit prep-ready only after materialization succeeds | Meeting detail differentiates active knowledge, proposed analysis, and cached prep. User edits write user claims or tombstones. |
| Entity detail | Mixed reads from `entity_assessment`, intelligence snapshots, signals, and claims | `get_entity_context` is canonical for account/project/person context | Entity enrichment Transform returns per-claim proposals, not a whole entity assessment | Apply entity claims, project legacy `entity_assessment` from claims, emit entity-intelligence-updated from committed claim changes | Entity detail is claim-first. Legacy assessment columns are a projection, never the authority for new AI truth. |
| Actions | Reads `actions`, AI backlog commitments, email commitments, rejection patterns, dismissal state | Action Read ability over active actions plus proposed action/open-loop claims | Extractor Transforms produce `OpenLoop`, `SuggestedOutcome`, commitment/action proposals with source refs | User accept/reject/dismiss and policy-safe auto-suggest create/update action rows; tombstones suppress resurrection | Suggested actions are proposals until accepted or policy-committed. Rejection is quality feedback; dismissal is preference and should not penalize source trust. |
| Email | Reads `emails` table columns for contextual summary, entity link, sentiment, urgency, commitments, questions, noise, reply debt | Email Read ability over raw Gmail cache, email claims, linked entity claims, dismissal tombstones | Email triage Transform emits `EmailSummary`, `EmailEntityLink`, `EmailUrgency`, `EmailSentiment`, `EmailNoiseVerdict`, `EmailQuestion`, `EmailCommitment`, `ReplyNeeded` proposals | Apply email proposals, update display columns as read model, persist dismissals, manage retry state | Raw Gmail facts and AI synthesis are separate. Gmail archive/unarchive is an external Publish side effect, not enrichment Maintenance. |

### Claim Granularity Decision

Enrichment generation must be claim-based at the smallest reviewable fact, not snapshot-based:

- Entity enrichment writes one claim per risk, win, stakeholder assessment, current-state statement, value-delivered statement, company-context fact, entity summary, open loop, or commitment.
- Meeting enrichment writes one claim per meeting topic, attendee context item, open loop, change marker, event note, or suggested outcome.
- Email enrichment writes one claim per summary, link, urgency/sentiment/noise verdict, extracted question, extracted commitment, and reply-needed assertion.
- Action extraction writes a proposal per suggested action or open loop. The action row is a derived workflow object after commit/accept, not the first source of truth.
- Briefing narrative output is not durable knowledge by default. If a narrative contains a new assertion worth retaining, it must emit an explicit claim proposal with citations.

### Ownership Boundary Decision

- **Claims** own durable knowledge, contradiction state, tombstones, provenance, trust, and user corrections.
- **Signals** own observations and invalidation notifications. They do not own truth and do not authorize claim writes.
- **Jobs** own scheduling, retry, coalescing, leases, idempotency, and outbox execution.
- **Read models** own cache/performance shape. They are rebuildable from claims/raw source cache and never override claims.
- **Surfaces** own presentation and user actions. They read via ability/services and dispatch explicit commands; they do not run LLMs or mutate enrichment state directly.

## C. Propose/Commit Boundary Decision

Options evaluated:

| Option | Benefits | Failure mode | Decision |
| --- | --- | --- | --- |
| Keep current auto-commit | Lowest migration cost; current surfaces stay populated | Continues clobber/user-correction risk, provenance gaps, and silent AI authority | Reject |
| System proposal for every enrichment output | Strongest safety and auditability | Floods users with low-value deterministic facts and cache refreshes; blocks background utility | Reject as universal policy |
| Tiered gated commit | Preserves safety for AI synthesis while allowing deterministic, corroborated, low-blast facts to flow | Requires policy config, trust compiler integration, and proposal UX/read models | Accept |

Recommended policy:

1. **Raw source facts** from integration caches can be committed by Maintenance when the source is system-of-record for that field, provenance is complete, and idempotency checks pass.
2. **Deterministic transforms** can auto-commit if they are pure functions over committed claims/raw source rows and cannot contradict a user-authored claim.
3. **AI/LLM/Glean synthesis** is proposed by default. It can auto-commit only when all of these are true:
   - Claim type is explicitly marked `auto_commit_allowed`.
   - Blast radius is low: display/contextual fact, not external action, Publish side effect, user preference, or account-critical field.
   - ADR-0105 provenance is complete, including prompt fingerprint for AI and upstream citations.
   - ADR-0114 trust score is at or above the configured threshold.
   - No active user-authored conflicting claim, user override, tombstone, or unresolved contradiction exists.
   - Source freshness and source_asof are within the claim type's freshness policy.
4. **Novel narrative analysis** that changes account state, risk, priority, suggested action, meeting recommendation, or email reply obligation routes to proposed state unless corroborated and policy-allowed.
5. **Actions and external side effects** require user acceptance or explicit policy preauthorization. Enrichment never directly publishes externally.

Open schema/config implication: ADR-0113 ships the data model and gate concept, but v1.4.1 needs the active agent strategy flipped to gated plus a concrete proposal/inbox/read API for enrichment proposals. If current `intelligence_claims.claim_state` cannot represent pending proposals cleanly, add an `intelligence_claim_proposals` table or ADR-0113 amendment before implementation.

## D. Trust and Provenance Integration

Every enrichment-authored claim or proposal must carry an ADR-0105 provenance envelope. Required fields:

- Ability identity, version, execution mode, actor, trigger, and job id.
- Prompt fingerprint and provider/model identity for AI/LLM-backed Transforms.
- Input snapshot references, not raw prompt-only provenance.
- Field-level source attributions with `DataSource`, source identifier, observed_at, source_asof when knowable, evidence weight, and synthesis marker.
- Child provenance for composed abilities such as `prepare_meeting` using `get_entity_context`.

Data source handling:

- Use the raw source class for raw evidence: Gmail, Calendar, Glean source document refs, Clay, Gravatar, transcript, capture, user note, or local DB source rows.
- Use `DataSource::Ai` only for the synthesis act. AI must cite upstream evidence and cannot be the only source for a fact that claims to describe the outside world unless the claim is explicitly "AI analysis".
- Glean answers should retain both the Glean retrieval source and the underlying document/contact/message source refs when available. Do not collapse them into `ai_enrichment`.
- Legacy transitional claim projection with empty `provenance_json` is not acceptable for v1.4.1 enrichment writes.

Trust score integration:

- Run ADR-0114 trust compiler before commit decisions.
- Inputs come from source reliability, freshness, corroboration, contradiction penalty, and user feedback.
- User-authored claims keep high trust and precedence. Agent claims can gain trust through corroboration and lose trust through rejection/correction.
- Dismissal and rejection semantics remain distinct: dismissal suppresses recurrence without source penalty; rejection lowers source/claim trust.
- The claim row should store the compiled trust score or a reference to the score computation version so reads can explain why a claim is active/proposed.

## E. Reversibility and DOS-12 Integration

All enrichment Maintenance writes must call ADR-0104 `check_mutation_allowed` and use a plan/apply split:

- **Plan**: compute claim proposals, projected read-model changes, signals to emit, and outbox jobs without mutating.
- **Apply**: inside a bounded transaction, re-check tombstones/user overrides/claim versions, write claims or proposals, write read-model projections, and enqueue signals/jobs.

Reversibility rules:

- User-authored claims and explicit field corrections are never overwritten by enrichment. Agent output may corroborate, contradict, or propose a replacement.
- If an enrichment output conflicts with a user-authored claim, create a contradiction record/proposal surfaced per ADR-0113 section 6; do not auto-resolve.
- Tombstone pre-gate remains mandatory before any trust gate. Dismissed email items, suggested actions, briefing callouts, and meeting/entity dismissed items must suppress resurrection at proposal time.
- Legacy `user_override` flags remain honored while projections migrate. A projection from claims must prefer user-authored active claims over AI claims for the same field.
- Every apply operation writes an audit record with before/after claim ids, read-model ids, source refs, and gate outcome.
- Undo is claim-level: withdraw/tombstone the AI claim or restore prior projection from active claims, not "rerun the whole snapshot".

Surface-specific reversibility:

- Briefing: dismissing a callout writes a tombstone/preference claim and invalidates briefing cache.
- Meeting detail: editing prep or notes writes user claims; future prep enrichment can only propose alternates.
- Entity detail: user corrections become account/person/project field correction claims and outrank agent claims.
- Actions: reject means quality feedback plus tombstone; dismiss means preference tombstone without quality penalty.
- Email: dismissed commitment/question/reply-needed items are email-subject tombstone claims and block re-proposal from later enrichment runs.

## F. Outbox Pattern Integration

ADR-0103 outbox applies to all enrichment external calls and local-plus-external sequences:

| External boundary | Target outbox job | Idempotency key | Apply behavior |
| --- | --- | --- | --- |
| Glean enrichment/retrieval | `glean.enrich_entity` or source retrieval job | provider, entity id, source version, prompt/input fingerprint | Store raw provider result and source refs; Transform proposals consume stored result |
| PTY/LLM Transform | `llm.transform.<ability>` | ability id/version, subject id, input snapshot hash, prompt fingerprint | Store completion/replay artifact; do not mutate claims until Maintenance apply |
| Gmail fetch/body read | `gmail.fetch_messages` / `gmail.fetch_body` | account, message/thread id, history id | Upsert raw cache rows with source_asof; enqueue triage Transform |
| Gmail archive/unarchive | `gmail.modify_labels` | user, thread ids, requested labels, command id | Local Pencil state first; Pen outbox call; reconcile or surface retry if Gmail fails |
| Clay/Gravatar | `profile.fetch.<provider>` | provider, person id/email/domain, provider etag/source_asof | Store provider cache; propose profile claims; apply avatar/read model separately |
| Avatar/file materialization | `cache.materialize_avatar` | person id, content hash | Write file/cache as rebuildable read model after source cache commit |

Durable invalidation job model requirements for DOS-236:

- Jobs are keyed by subject id, ability id, source claim versions, source_asof, and input hash.
- Claim changes enqueue invalidation jobs; signal bursts coalesce before job execution.
- Jobs store attempts, backoff, last error, lease owner, and terminal state.
- External results are replayable in Evaluate mode through ADR-0104 mode-aware clients.
- A successful Transform job produces proposals; a separate Maintenance job applies them. Retrying either stage must be idempotent.

This design requires DOS-236 to support both invalidation jobs and outbox jobs or define a shared substrate with separate job kinds. DOS-237 coalescing must coalesce by subject/ability/input hash rather than by broad "entity changed" events, or claim-level churn will overrun enrichment.

## G. Evaluation Harness Extension

ADR-0110 fixtures should be added per enrichment surface and per pipeline class.

Fixture families:

- `entity_enrichment`: committed claims/raw source rows in, expected entity claim proposals and trust gate outcomes out.
- `meeting_briefing`: meeting metadata, linked subject claims, calendar facts, provider replay in, expected `MeetingBrief` plus optional claim proposals out.
- `entity_detail`: claim set in, `get_entity_context` read output and provenance masks out.
- `actions`: email/transcript/meeting input in, expected action/open-loop proposals, dismissal suppression, and accept/reject apply diff out.
- `email_triage`: Gmail replay/raw email rows plus linked entity context in, expected summary/link/urgency/noise/commitment/question/reply-needed proposals out.
- `profile_enrichment`: Clay/Gravatar replay input in, expected profile/person claim proposals and avatar cache plan out.
- `outbox_replay`: transient/permanent external failures, duplicate completions, and stale input hashes.

Assertions by ability category:

- Read: exact DTO/provenance comparison with deterministic redaction/sensitivity masks.
- Transform: structured proposal-set comparison, prompt fingerprint regression classification, optional LLM-as-judge only for narrative wording.
- Maintenance: snapshot diff over `intelligence_claims`, proposal state, read models, signal queue, outbox queue, and audit rows.
- Publish: outbox/Pencil/Pen comparison only. No live external side effects in Evaluate.

Required regression gates:

- Prompt fingerprint changes classify expected output drift as PromptChange, ProviderDrift, InputChange, CanonicalizationBug, or LogicChange.
- Tombstone/user-override fixtures prove no ghost resurrection for email items, suggested actions, briefing dismissals, and entity field corrections.
- Contradiction fixtures prove agent claims do not overwrite user claims and instead surface proposed contradictions.
- Coalescing fixtures prove one claim burst creates bounded jobs and no duplicate external calls.
- Surface fixtures prove briefing, meeting detail, entity detail, actions, and email all render from active/proposed claims consistently.

## H. Deliverable Decisions, ADR Amendments, and W1/W2 Inputs

Accepted design decisions:

1. Enrichment is not a wrapper around the old queue. It becomes Acquire -> Transform -> Maintenance -> Read.
2. Claim granularity is per reviewable fact, not per `IntelligenceJson`, prep blob, or email row.
3. AI synthesis uses tiered gated commit. Current automatic agent commit is retired for v1.4.1 enrichment paths except policy-allowed, trusted, low-blast claims.
4. `get_entity_context` and `prepare_meeting` are the reference shapes for Read and Transform. Similar ability boundaries should be introduced for email triage, action extraction, profile enrichment, and entity claim synthesis.
5. `entity_assessment`, `meeting_prep`, email display columns, reports, and briefing artifacts are read models/caches after migration.
6. Signals notify and invalidate; jobs schedule and retry; claims own truth; surfaces present and collect user intent.
7. Enrichment may not perform external Publish side effects. Gmail archive/unarchive and future external task syncs use ADR-0117 Pencil/Pen outbox semantics.

Amendments or follow-up docs needed:

- **ADR-0113 / DOS-241 follow-up**: define the concrete pending proposal storage/API and the v1.4.1 active gate config for agent-authored enrichment claims.
- **ADR-0105 / ADR-0114**: require non-empty provenance and trust compiler output for enrichment claim commits; clarify `DataSource::Ai` plus upstream citation rules for Glean/LLM synthesis.
- **DOS-236**: include Transform/outbox job kinds, idempotency keys, source claim-version invalidation, provider replay artifacts, and stale-input handling.
- **DOS-237**: coalesce claim/source changes by subject, ability, and input hash; include the load gate for claim-level enrichment bursts.
- **DOS-235**: signal policy registry must distinguish observation, invalidation, user feedback, and read-model-materialized signals so claim writes do not cause uncontrolled propagation.
- **DOS-295**: targeted repair should operate on individual claim proposals/claims with source refs, not regenerate whole entity assessments.
- **ADR-0117**: no amendment needed if enrichment treats Gmail archive/unarchive as Publish/Pen and does not add external publish paths.

Review gates for implementation planning:

- W1-E DOS-306 should cite this doc as the boundary contract for signal/job/claim ownership.
- W1-B/C/D L0 plans should accept or explicitly reject the DOS-235/236/237 amendment bullets above.
- W2-H DOS-295 should cite the claim granularity decision and targeted repair fixture requirements.
- First implementation PRs should include fixtures for at least entity detail, meeting briefing, actions, and email before flipping any agent commit policy to gated in production.

# DOS-291 W6-D L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 6 - Validation suite.
- **Agent:** W6-D.
- **Linear issue:** DOS-291 - "Validation: ambiguous identity and primary-context selection" (DOS-291 content supplied verbatim in the authoring prompt for this packet).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. The only file created by this turn is `.docs/plans/v1.4.1-waves/W6-D-L0-packet.md`.
- **W6-D assignment:** the wave plan names W6-D as "DOS-291 ambiguous identity + primary-context selection" and assigns it "validation bundle + assertions on subject ambiguity edge cases." Source: `.docs/plans/v1.4.1-waves.md:637-640`.
- **W6 merge gate:** W6 requires L0 plan approvals, L2 approvals, L3 Suite E final with bundles 1-13 and 14-18 mandatory green, L4 `/qa`, L5 drift check, retro, and proof bundle. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Reviewer contract:** W6 L0 requires `qa-expert` for all six W6 agents, with `security-auditor` only for DOS-292. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Validation-suite numbering contract:** five new bundles 14-18, one per DOS-289 through DOS-293, all mandatory in the v1.4.1 release gate. W6-D maps to **bundle 16**. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/tests/fixtures/bundle-README.md:29-45`.
- **Fixture catalogue:** committed corpus at `src-tauri/tests/fixtures/bundle-README.md`; bundles 1-13 are present; the harness discovers only hyphenated `bundle-N` directories with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Runtime contract:** synthesized user-facing and agent-facing context must go through abilities; every ability output carries provenance once; Transform outputs cannot authorize mutation. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:341-366`, `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`.

## 2. Load-Bearing User Outcome

DOS-291 frames the user failure this bundle must prevent:

> "DailyOS needs to decide which account, project, person, or meeting subject a piece of work belongs to. Today many bugs come from treating ambiguity as confidence — shared domains, same-name people, parent/child accounts, recurring meeting inheritance, weak Linear/Glean links. This is adjacent to cross-entity content bleed, but the missing invariant is earlier — the system must know when it does not have enough evidence to select a primary subject."

The user harm is also explicit:

> "Wrong account/person/project becomes the center of the meeting briefing. All downstream intelligence is then built on the wrong context."

The load-bearing outcome for W6-D is therefore not "ambiguity gets a warning." It is: **DailyOS must refuse to render confident primary-subject context when evidence does not clear the selection threshold.** Ambiguous candidates render as ambiguity, as a confirmation request, or as a primary/secondary split — never as silent confident context.

Required behavior from DOS-291:

> "primary/secondary/ambiguous subject states are explicit; direct evidence outranks inherited evidence; user-confirmed subject outranks classifier confidence; ambiguous candidates render as ambiguity or confirmation request, not confident context; ability outputs include enough subject-selection metadata to explain why a subject was chosen."

This user outcome depends on existing Intelligence Loop substrate:

- **Claim model:** subject ambiguity is a claim/entity-linker property, not a render layer property. Entity-linker output and `subject_ref` resolution must encode primary/secondary/ambiguous state explicitly.
- **Provenance and trust:** ability outputs must include subject-selection provenance — which evidence selected the subject, what alternatives were considered, what the confidence margin was. ADR-0105 names `subject` as a provenance envelope field. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.
- **Signals and invalidation:** when the user confirms or overrides a subject, the resulting `WrongSubject` tombstone or user-confirmed link must invalidate dependent claims and survive subsequent enrichment.
- **Runtime and surfaces:** `prepare_meeting` and `get_entity_context` must expose subject-selection metadata so the user can see why a primary was chosen. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:268-290`.
- **Feedback loop:** user-confirmed subject must outrank classifier confidence and survive sync/re-enrichment.

The W6-D proof must cover these concrete DOS-291 edge cases:

- Two accounts share a domain; one meeting attendee matches both.
- Parent account and child account both match the same source.
- Two projects under one account have similar names.
- Two people have the same first name and similar titles.
- Recurring meeting series was historically about Account A but current event is about Account B.
- Email thread mentions two customers.
- Linear issue title matches an account but issue is internal platform work.
- User manually selected primary entity; classifier attempts to override it.

## 3. Pre-Work

- **Read W6 source of truth.** W6-D owns ambiguous identity + primary-context selection. Source: `.docs/plans/v1.4.1-waves.md:637-640`.
- **Read W6 merge gate.** L3 Suite E final requires bundles 1-13 + bundles 14-18 mandatory green, no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Acknowledged Amendment 1.** Amendment 1 recategorizes W3 stage-3b as `instrumentation-complete, data-sufficiency-pending`, relaxes W6's hard precondition, and says W6 starts against the partial baseline. W6-D scope is unaffected. Sources: `.docs/plans/v1.4.1-waves-amendments.md:15-23`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Mapped bundle number.** Bundles 1-13 exist; bundles 14-18 are new to W6, one per DOS-289 through DOS-293, in spec order. W6-D = bundle 16. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Read sibling W6-B packet.** W6-B (bundle 14) covers stale-current contradiction depth; W6-D's overlap with W6-B is bounded to the recurring-meeting-series-changed-subject case where temporal change + identity ambiguity intersect. Source: `.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:43-50`.
- **Read sibling W6-C packet.** W6-C (bundle 15) covers cross-surface consistency; W6-D is adjacent (subject selection upstream of cross-surface rendering) but distinct. The DOS-291 ticket explicitly notes the adjacency: "this is adjacent to cross-entity content bleed, but the missing invariant is earlier."
- **Bundle-4 prior art.** `bundle-4` already covers "same person linked across two accounts through ambiguous context" with the invariant "person-shaped account ambiguity cannot bleed Account B context into Account A enrichment." W6-D bundle 16 extends this to the full DOS-291 edge-case set; it does not duplicate bundle-4's person-domain-ambiguity coverage. Source: `src-tauri/tests/fixtures/bundle-README.md:36`.
- **Read ADRs.** Binding ADRs are ADR-0102 abilities runtime, ADR-0105 provenance + subject + field attribution, and ADR-0114 scoring unification (subject-fit confidence). Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`, `.docs/decisions/0114-scoring-unification.md:1-49`.
- **Read harness shape.** Loader requires `clock.txt`, `seed.txt`, `state.sql`, `inputs.json`, `provider_replay.json`, `external_replay.json`, `expected_output.json`, `expected_provenance.json`, `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.

## 4. Architecture

### 4.1 Bundle Assignment

W6-D owns **bundle 16**.

- **New fixture directory:** `src-tauri/tests/fixtures/bundle-16/`.
- **New substrate test file:** `src-tauri/tests/bundle16_ambiguous_identity_substrate_test.rs`.
- **Naming rationale:** project convention `bundleN_<topic>_substrate_test.rs` (sibling references: `bundle4_cross_entity_person_ambiguity_substrate_test.rs`).
- **Discovery rationale:** fixture directories must be hyphenated `bundle-N` with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Release-gate rationale:** W6/W7 requires bundles 14-18 mandatory green. **The W6-D PR itself must include the edit promoting bundle 16 to mandatory** in `src-tauri/src/release_gate.rs:26-38`; coordination-only with W7 is not acceptable. Source: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`.

### 4.2 Fixture Invariant

**Invariant:** Subject selection must distinguish primary / secondary / ambiguous states explicitly. Confident primary-subject context cannot render when evidence does not clear the configured threshold. Direct evidence outranks inherited; user-confirmed outranks classifier; ambiguous candidates render as ambiguity or confirmation request.

The invariant is not satisfied by adding ambiguity warnings to confident output. The bundle must prove subject-selection state shape, evidence-hierarchy ordering, user-override durability, and subject-selection provenance:

- Entity linker / subject resolver returns one of `primary`, `secondary`, `ambiguous`, or `unconfirmed`; the type is preserved through provenance.
- Direct evidence (attendee match, explicit Linear linkage, sender domain match) is weighted strictly above inherited evidence (parent account, recurring meeting series history).
- A user-confirmed subject persists across enrichment cycles; the classifier cannot silently override.
- Subject-selection provenance includes the chosen subject id, the alternative candidates, the evidence used, and the confidence margin.

### 4.3 Fixture Shape

Bundle 16 seeds a workspace exercising all eight DOS-291 ambiguity cases. Generic synthetic identifiers only — no real customer names, domains, or emails.

Required fixture files follow the loader contract:

- `clock.txt` fixes the test clock.
- `seed.txt` fixes randomization.
- `state.sql` seeds (machine-checkable scenario registry — one row per scenario, each with a stable `scenario_id` so an implementer cannot accidentally seed 7 of 8):
  - `scenario_id: same-domain-twins` — two accounts sharing `shared.example.com`; one attendee matches both.
  - `scenario_id: parent-child-account` — parent account + child account both matching one source.
  - `scenario_id: similar-project-names` — two same-named projects under one account.
  - `scenario_id: same-name-people` — two people sharing first name + similar title.
  - `scenario_id: recurring-series-subject-change` — recurring meeting historically Account A, current instance Account B.
  - `scenario_id: email-thread-two-customers` — email thread mentioning two customers.
  - `scenario_id: linear-title-internal-work` — Linear issue with title-matched ambiguity, actually internal platform work.
  - `scenario_id: user-confirmed-override-attempt` — one user-confirmed primary subject row at `user_confirmed_subjects` table (named row, NOT just "a row"); a subsequent classifier enrichment run attempts to write `accountA` as primary; expected post-state: the original user-confirmed row at `user_confirmed_subjects.id = <known-id>` is unchanged AND the classifier's `accountA` attempt is logged at `classifier_override_attempts` with `rejected = true`, NOT promoted to active primary.
- `inputs.json` drives `prepare_meeting`, `get_entity_context`, and (where relevant) `get_daily_readiness` through the harness.
- `provider_replay.json` includes **at least one attempted confident primary-subject talking point per ambiguous scenario** (`same-domain-twins`, `similar-project-names`, `same-name-people`, `email-thread-two-customers`, `linear-title-internal-work`), keyed by scenario_id, so the assembler validation can prove refusal/qualification on each. A single provider attempt total is not sufficient.
- `external_replay.json` pins downstream-source ambiguity (CRM domain match, Glean parent/child match).
- `expected_output.json` asserts: ambiguous candidates render as ambiguity/confirmation-request; user-confirmed primary survives override attempt; direct-evidence subject beats inherited.
- `expected_provenance.json` asserts subject-selection provenance per the W6-D invariant: chosen subject id + alternatives + evidence + margin.
- `expected_state.json` asserts post-action state for the user-override case: the user-confirmed link persists.
- `metadata.json` includes `bundle: 16`, `scenario_id: ambiguous-identity-primary-context`, `surfaces_exercised` covering meeting prep + entity context, dominant factors including subject-selection confidence + user-feedback override durability, and a pass/fail definition that fails if any ambiguous candidate renders as confident primary context. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.

### 4.4 Seeded Scenario Coverage

The bundle's eight scenario branches map directly to DOS-291's concrete edge cases:

1. **Same-domain twin accounts.** Two accounts share `shared.example.com`; an attendee matches both. Expected: ambiguous, not confident.
2. **Parent/child account match.** Parent matches by domain, child by attendee identity. Expected: child is primary (direct evidence) over parent (inherited).
3. **Same-name projects under one account.** Two projects share `Phoenix` as a prefix. Expected: ambiguous unless a Linear link or explicit attendee comment disambiguates.
4. **Same-first-name people, similar titles.** Two `Alex Example*` records. Expected: ambiguous; prompt for confirmation.
5. **Recurring series subject change.** Series historically Account A; current instance has Account B attendees. Expected: current evidence overrides historical inheritance. (Coordinates with W6-B's temporal precedence rules.)
6. **Email thread, two customers.** Thread mentions both `accountA` and `accountB`. Expected: ambiguous unless one customer is the direct meeting subject.
7. **Linear title-matched internal work.** Linear issue title contains `accountA` but the work is internal platform. Expected: not auto-attributed to accountA.
8. **User-override durability.** User manually selected `accountB` as primary; subsequent enrichment cycle attempts to override to `accountA`. Expected: user selection persists; override attempt is logged but rejected.

### 4.5 Subject-Selection Provenance

Every ability output exercised by bundle 16 must expose subject-selection provenance through ADR-0105's provenance envelope:

- `subject` field carries the chosen `subject_ref`.
- A subject-selection record (in `field_attributions` or a sibling structure) carries: alternative candidates considered, evidence used, confidence margin, and decision rule applied (direct-over-inherited, user-confirmed-wins, ambiguous-blocks).
- For ambiguous outcomes, the provenance carries the ambiguity reason and the alternatives so the user can confirm.

This is not a display-layer assertion. The packet asserts the provenance envelope shape on the ability output itself. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`, `.docs/decisions/0105-provenance-as-first-class-output.md:206-241`.

### 4.6 Intelligence Loop Check

- **Claim model:** subject-selection state lives on the claim/entity-linker layer, not on render strings; bundle 16 asserts on the substrate, not display output.
- **Provenance and trust:** every ability output asserted exposes subject-selection provenance through the ADR-0105 envelope.
- **Signals and invalidation:** `WrongSubject` tombstone and user-confirmed subject must invalidate dependent claims and survive subsequent enrichment.
- **Runtime and surfaces:** `prepare_meeting` and `get_entity_context` are the required consumers; both must expose subject-selection provenance. Tauri and MCP parity follows ability-registry invocation.
- **Feedback loop:** user-confirmed subject outranks classifier confidence; override attempts are logged but rejected.

## 5. Acceptance Criteria

DOS-291 Acceptance, quoted verbatim:

> "Entity linker returns primary only when evidence clears threshold; ambiguous primary context blocks confident briefing content; user-selected primary entity survives sync/re-enrichment; inherited links include inheritance reason and can be overridden by stronger current evidence; prepare_meeting and get_entity_context expose subject-selection provenance."

Testable decomposition:

1. **Entity linker threshold gate.** Given two same-domain twin accounts and an attendee matching both, the entity linker returns `ambiguous`, not a confident primary. Source: DOS-291 Required-behavior + edge case 1.
2. **Ambiguous primary blocks confident briefing content.** Provider replay attempting a confident primary-subject talking point in scenario 1 must be refused, qualified, or rendered as confirmation request. Source: DOS-291 Required-behavior.
3. **User-selected primary persists across re-enrichment (substrate row named).** Scenario `user-confirmed-override-attempt`: the named seeded row at `user_confirmed_subjects.id = <known-id>` is unchanged after the enrichment cycle, AND the classifier's `accountA` override attempt is recorded as `rejected = true` in `classifier_override_attempts`, AND `accountA` does not appear as active primary in `prepare_meeting` output. An implementation that satisfies this by logging the attempt while silently replacing the original user-confirmed row fails. Source: DOS-291 Required-behavior.
4. **Inherited links carry inheritance reason.** Scenario 5 (recurring series subject change): the historical inheritance is exposed as inheritance reason; current direct evidence overrides it. Source: DOS-291 Required-behavior.
5. **Direct evidence beats inherited.** Scenario 2 (parent/child): child is primary (direct attendee evidence); parent is inherited and does not silently win.
6. **`prepare_meeting` exposes subject-selection provenance.** Output provenance envelope includes chosen `subject_ref`, alternative candidates considered, evidence used, confidence margin, decision rule applied. Source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.
7. **`get_entity_context` exposes subject-selection provenance.** Same provenance shape as `prepare_meeting`.
8. **Lint or release-gate gate.** A harness assertion fails if any ambiguous candidate from bundle 16 renders as confident primary on `prepare_meeting`, `get_entity_context`, or `get_daily_readiness` output.
9. **Bundle 16 is mandatory — the W6-D PR itself flips the mandatory bit.** Implementation does not "coordinate with W7" or defer the wiring. The W6-D PR includes the edit promoting bundle 16 from tracked to mandatory in `src-tauri/src/release_gate.rs:26-38`. If that edit is missing, L2 review rejects. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`.
10. **No PII in seeded state.** All accounts, people, domains, projects use synthetic generic identifiers (`example.com`, `Alex Example`, `accountA`/`accountB`).

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-291 content is supplied verbatim in the authoring prompt for this packet.
- **Upstream unblock:** W6 starts after the W3 stage-3b precondition, as amended to instrumentation-complete. Sources: `.docs/plans/v1.4.1-waves.md:653-655`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Adjacent W6 coordination:** W6-D owns bundle 16; W6-B owns bundle 14 (stale-current; recurring-series temporal overlap noted in scenario 5); W6-C owns bundle 15 (cross-surface; subject-selection feeds in upstream). Sources: `.docs/plans/v1.4.1-waves.md:627-640`.
- **Prior art:** `bundle-4` already covers person-shaped account ambiguity at the bleed-prevention layer; W6-D bundle 16 covers subject-selection upstream of bleed. Source: `src-tauri/tests/fixtures/bundle-README.md:36`.
- **Not a DOS-290 takeover:** Cross-surface consistency is W6-C; W6-D bundle 16 may assert cross-surface for the subject-selection scenario but does not duplicate bundle 15's coverage.
- **Not a W6-A takeover:** W6-A's `dos291_ambiguous_identity_regression.rs` consumes bundle 16; W6-D produces the bundle.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Panel reason:** W6 merge gate requires `qa-expert` for all six W6 agents. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Security reviewer:** not required for W6-D. `security-auditor` is named only for DOS-292 (W6-E).
- **Review focus for `qa-expert`:**
  - All eight DOS-291 edge-case scenarios are seeded in `state.sql`.
  - Each scenario has at least one assertion in `expected_output.json` + `expected_provenance.json`.
  - Subject-selection provenance shape is asserted, not just rendered output.
  - User-override durability scenario is end-to-end (override attempt + persistence assertion).
  - Bundle 16 can become mandatory in the W6/W7 release gate.
  - No PII in any seeded state.

## 8. L0 Acceptance Gate

L0 passes only if the reviewer accepts all of the following:

1. **Problem fit:** the plan tests subject-selection upstream of cross-entity bleed, not the bleed-prevention layer (already covered by bundle-4).
2. **Bundle lock:** W6-D is locked to bundle 16 and implementation path `src-tauri/tests/bundle16_ambiguous_identity_substrate_test.rs`.
3. **Fixture lock:** bundle directory `src-tauri/tests/fixtures/bundle-16/` using loader-required files and `metadata.json` fields. Sources: `src-tauri/tests/fixtures/bundle-README.md:8-27`.
4. **Amendment acknowledgement:** Amendment 1 is acknowledged.
5. **Acceptance coverage:** every clause of DOS-291 Acceptance is decomposed into a testable assertion in §5.
6. **Eight edge-case scenarios seeded:** the bundle covers all eight DOS-291 concrete cases, not a subset.
7. **Subject-selection provenance shape:** provenance is asserted at the ability-output substrate, not at the render layer.
8. **Reviewer panel:** `qa-expert` is the only required L0 reviewer; no `security-auditor`.
9. **No PII:** all fixture identifiers are synthetic.

## 9. Out-Of-Scope

- Cross-entity content bleed beyond the subject-selection layer (bundle-4 + W6-C bundle 15).
- Writing implementation files in this packet authoring turn.
- Committing changes.
- Building a user-facing confirmation-request UI (the bundle asserts the ability output state; UI implementation is downstream).
- Adding new ADRs for subject-selection ranking (the ADR-0105 provenance envelope is sufficient).
- Treating DOS-282 W6-A's regression files as W6-D scope; `dos291_ambiguous_identity_regression.rs` is owned by W6-A and consumes bundle 16.
- DOS-289 stale-current, DOS-290 cross-surface, DOS-292 source lifecycle, DOS-293 sync/refresh (W6-B/C/E/F).
- Customer-specific identifiers anywhere in the bundle.

## 10. Changelog

- **V1 - 2026-05-15:** Initial W6-D L0 packet. Assigned DOS-291 to bundle 16; locked the eight DOS-291 edge-case scenarios; mapped subject-selection provenance through ADR-0105; named `qa-expert` as the only required L0 reviewer.

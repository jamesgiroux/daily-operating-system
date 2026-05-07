# Fixture Bundle Catalogue

The committed fixture corpus uses hyphenated bundle directories:
`bundle-1` through `bundle-13`. Do not add a parallel
`bundles/bundle_N` tree; the harness discovers only `bundle-N` directories
with a `metadata.json` manifest.

## Manifest Fields

Each bundle carries the W4/W6 manifest in `metadata.json`:

| field | purpose |
| --- | --- |
| `bundle` | Numeric bundle id used by loader coverage and release gates. |
| `scenario_id` | Stable scenario label for reports and downstream gates. |
| `invariant` | The named behavior the bundle proves. |
| `expected_render_policy` | Expected display posture such as show, warning, or suppress. |
| `surfaces_exercised` | Ability and scenario labels used for suite selection. |
| `source_lifecycle_refs` | Source, claim, feedback, or row ids that make the scenario assertable. |
| `anonymization_cert` | Synthetic-data certificate; fixture content must stay on `example.com`. |
| `retention_policy` | Fixture retention posture. |
| `prompt_fingerprint_baseline` | Canonical prompt hash for provider-backed bundles, or a stable no-provider baseline. |
| `prompt_template_version` | Prompt template version for provider-backed bundles. |
| `completion_text_hash` | SHA-256 of replay completion text when a provider replay is pinned. |
| `trust_factors_dominant` | Trust dimensions the fixture is meant to stress. |
| `pass_fail_definition` | Human-readable PASS/FAIL contract. |
| `fixture_design_notes` | Extra row mapping, scenario list, and coordination notes. |

## Bundle Scenarios

| bundle | scenario list | named invariant |
| --- | --- | --- |
| `bundle-1` | Claim-backed `get_entity_context` parity; same-domain sibling account; parent renewal meeting touching target and sibling; six paraphrases collapsed through `claim_corroborations`; target-account trust-band diversity; parent-account `WrongSubject` tombstone. | Active claim-backed entity context output for `dos287-target-example` stays ordered and excludes adjacent account context while substrate rows make cross-account bleed and correction cases assertable. |
| `bundle-2` | Provider hallucination with low corroboration and poor internal consistency. | Hallucinated account content stays below the NeedsVerification threshold and is not silently accepted as truth. |
| `bundle-3` | Withdrawn/stale source plus prior user dismissal. | Stale source resurrection does not re-promote a previously dismissed claim. |
| `bundle-4` | Same person linked across two accounts through ambiguous context. | Person-shaped account ambiguity cannot bleed Account B context into Account A enrichment. |
| `bundle-5` | First-person meeting-prep parity; attendee `WrongSubject` tombstone; user-edited superseding claim; duplicate/paraphrase corroboration; expired dormant claim; double-refresh no-resurrection guard. | `prepare_meeting` renders only active Riley context and never resurrects tombstoned, superseded, or expired dormant claims during meeting refresh. |
| `bundle-6` | Many weak corroborators against one strong contradictory source. | Raw corroboration count cannot drown out stronger source reliability. |
| `bundle-7` | Closed temporal-scope claim with post-closure evidence. | Closed-scope claims are not refreshed by evidence outside their valid window. |
| `bundle-8` | Public output with private-class source material nearby. | Sensitive/private content cannot leak into public-class rendered surfaces. |
| `bundle-9` | Recurring one-on-one with prior open loops. | Open loops remain attributed to the person, not a neighboring account or the meeting as a whole. |
| `bundle-10` | Multi-attendee meeting with a known account inferred from attendee domains. | Account-level topics are attributed to the shared known account. |
| `bundle-11` | Meeting brief with stale Glean evidence. | Stale source provenance is marked and stale claims do not render as current facts. |
| `bundle-12` | Meeting brief with revoked source evidence. | Revoked source evidence is masked and cannot render as a meeting-prep fact. |
| `bundle-13` | Bundle-1 target/adjacent pattern applied to `prepare_meeting` prompt evidence. | Source-ref matched adjacent-account claims are filtered before the provider boundary. |

## DOS-283 Coordination

`bundle-1` exercises the claim-backed `get_entity_context` read path. The legacy
`entity_context_entries` rows remain as substrate assertions for ownership and
bleed checks, but the active target-account claims now define the expected
output.

`bundle-5` drives `prepare_meeting`, so adding active claims changes the
canonical prompt input. Regenerate `provider_replay.json` by running the
harness, capturing the replay-miss `canonical_prompt_hash`, updating the
fixture, and rerunning.

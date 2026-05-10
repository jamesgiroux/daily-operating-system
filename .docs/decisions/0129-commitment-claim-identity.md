# ADR-0129: CommitmentClaim Identity and Owner Resolution

Date: 2026-05-10
Status: Accepted

## Context

Commitments extracted from account intelligence were previously bridged by an
LLM-supplied `commitment_id` and often stored the owner as prose in
`actions.context` with an `owner:` prefix. Re-enrichment could emit the same
commitment under a different source id, creating duplicate backlog actions, and
owner reassignment had no typed feedback path into later extraction.

## Decision

DailyOS commitments are represented as typed `CommitmentClaim` values in
`abilities-runtime::abilities::extractors::commitment`.

`CommitmentClaim` carries:

- `commitment_id`
- `account_id`
- `title` and `title_normalized`
- optional `due_normalized`
- optional `owner_raw`
- `owner: OwnerRef`
- optional per-claim trust score and trust band

`OwnerRef` is a closed enum:

- `Person { person_id, display_name, confidence, source }`
- `Team { label, confidence, source }`
- `Ambiguous { raw, reason, candidates }`
- `Unassigned`

Every extracted commitment receives one of these variants. Unknown or
conflicting owners are explicit `Ambiguous` values, not hidden in prose.

## Identity

`commitment_id` is derived by a pure function:

```text
sha256(
  normalize(title),
  account_id,
  due_normalized,
  owner_raw
)
```

The runtime formats the digest as `commitment:<hex>`.

Normalization rules:

- title: trim, collapse whitespace, lowercase ASCII
- due date: date-only `YYYY-MM-DD` when a timestamp has a date prefix
- owner raw: trim and collapse whitespace

Source ids, source labels, extraction timestamps, and LLM-provided ids are not
part of identity. They are recorded as sightings in
`action_commitment_sources`. Re-running enrichment with unchanged commitments
therefore preserves the action/commitment id set while appending source rows.

## Owner Resolution Contract

Owner resolution lives in `src-tauri/src/abilities/read/resolve_owner.rs` and
is deterministic for a given database snapshot.

Resolution order:

1. Existing `actions.owner_source = 'user_reassigned'` for the same
   `commitment_id`.
2. Team labels such as account team, customer success, product, legal, finance,
   customer, or joint ownership.
3. Exact email match.
4. Exact account-stakeholder person name.
5. Exact global person name.
6. High-confidence fuzzy account-stakeholder person name.
7. Explicit `Ambiguous` owner with candidate details when no unique match
   exists.

User reassignment writes structural owner columns on `actions`, not prose. The
resolver checks those columns first, so future enrichment preserves the user's
assignment.

## Storage

Commitment actions store:

- `actions.commitment_id`
- `actions.owner_raw`
- `actions.owner_entity_id`
- `actions.owner_confidence`
- `actions.owner_source`
- `actions.trust_score`
- `actions.trust_band`

`action_commitment_sources` stores every sighting of a commitment, including
source metadata, trust, owner raw text, and serialized `OwnerRef`.

Backlog commitment duplicates are prevented by a partial unique index on
`(title, account_id)` for backlog commitment rows. Migration v155 collapses
existing exact-title backlog duplicates before installing the guard.

## Trust

Commitment trust is computed through the W3-B Trust Compiler path
(`compile_trust`, which evaluates the canonical `FactorRegistry`). The Work
surface renders the stored trust band, score, and source count through an
"About this" affordance on each commitment card.

## Consequences

- Production rows cannot rely on `context = 'owner: ...'` as their only owner
  representation; those rows are migrated into structural owner columns and
  unresolved cases are marked ambiguous.
- LLM-provided commitment ids are compatibility input only; runtime identity is
  deterministic.
- Re-enrichment is append-only at the sighting layer and stable at the action
  identity layer for unchanged commitments.

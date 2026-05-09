# Claim Edges

`claim_edges` is a provenance-preserving projection from committed claims into
entity graph edges. It is not a new operational linking writer. W2-G only adds
the declarative frontmatter map compiler under `services::claims::link_map`.

## Data Model

`claim_edges` stores:

- `from_entity_id`, `to_entity_id`, `edge_type`: the directed graph edge.
- `origin_claim_id`: the immutable claim that produced the edge.
- `link_source`: one of `frontmatter_map`, `manual`, or `extracted`.
- `weight`, `confidence`: ranking inputs for downstream graph readers.
- `superseded_by`, `tombstoned_at`: edge lifecycle fields.
- `created_at`: copied from the origin claim commit time.

`claim_edges_active` returns rows where `superseded_by IS NULL` and
`tombstoned_at IS NULL`.

`backlinks` is a convenience view keyed by `to_entity_id` for "what points at
this entity?" readers.

## Frontmatter Map

`CLAIM_LINK_MAP` is a const slice of `LinkRule`:

- `field`: exact `intelligence_claims.field_path` match.
- `edge_type`: persisted edge type.
- `direction`: `Forward` means subject -> linked entity; `Incoming` means
  linked entity -> subject.
- `fanout`: false keeps only the first target; true emits one edge per target.
- `subject_type`: subject-kind filter. Rules do not fire for other subject
  kinds.

## Map Table

| subject_type | field | edge_type | direction | fanout |
| --- | --- | --- | --- | --- |
| `CanonicalSubjectType::Meeting` | `account` | `mentions_account` | `EdgeDirection::Forward` | `false` |
| `CanonicalSubjectType::Meeting` | `project` | `mentions_project` | `EdgeDirection::Forward` | `false` |
| `CanonicalSubjectType::Account` | `stakeholders` | `has_stakeholder` | `EdgeDirection::Forward` | `true` |
| `CanonicalSubjectType::Person` | `linked_entities` | `has_stakeholder` | `EdgeDirection::Incoming` | `true` |

The compiler reads target IDs from JSON strings, arrays, or objects containing
common ID fields (`id`, `entity_id`, `person_id`, `account_id`, `project_id`,
`meeting_id`) and falls back to comma, semicolon, or newline separated text.

## Lifecycle

`commit_claim` writes claim edges in the same SQLite transaction as the origin
claim insert.

When a claim explicitly supersedes another claim, active edges from the old
claim are marked with `superseded_by = <replacement claim id>`.

When a tombstone claim lands for the same subject, claim type, and field path,
active edges for matching prior claims are marked with `tombstoned_at`. This is
the edge-level counterpart to the ADR-0113 tombstone pre-gate: re-enrichment of
the same field cannot silently resurrect old edges into `claim_edges_active`.

Withdrawn or tombstoned feedback lifecycle transitions also mark the affected
claim's edges with `tombstoned_at`.

## Extension Point

Future work can add field mappings through the `frontmatter_link_map!` macro and
the `ClaimLinkMap` trait without adding new claim-type variants. Claim-type
taxonomy changes remain out of scope for W2-G and land in W4.

# Evaluation Evidence Packet Template

## Packet

- Packet id:
- Title:
- Scope:
- Owning issue:
- Generated at:
- Status: internal / release / public-comparison

## Included Records

| Record | SHA-256 | Suite | Mode | Result | Privacy | Publishable |
| --- | --- | --- | --- | --- | --- | --- |

## Artifact Index

| Artifact | SHA-256 | Kind | Privacy | Publishable | Redaction |
| --- | --- | --- | --- | --- | --- |

## Methodology

Describe the corpus, fixture set, adapter, command, model/config, thresholds,
and dataset source/license/hash.

## Metrics

Keep metric families separate:

- performance regression
- retrieval recall/precision
- answer quality
- provenance quality
- temporal correctness
- trust-band correctness
- surface safety
- public comparison

## Caveats

State what this packet proves and what it does not prove. For public comparison,
state clearly when retrieval recall is being reported rather than generated
answer quality.

## Failure Appendix

List every failure, excluded run, skipped fixture, and exclusion reason. Hidden
excluded failures make the packet non-publishable.

## Privacy Review

- Customer data absent:
- Absolute local paths absent:
- Identity maps absent:
- Private fixture payloads absent:
- Judge transcripts reviewed or excluded:
- Redaction status:

## Public Claim Rule

State the exact public claim this packet supports. If no public claim is
supported, mark the packet internal-only.

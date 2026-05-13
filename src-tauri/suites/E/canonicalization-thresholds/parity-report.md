# Canonicalization Parity Report

- Schema: `canonicalization-parity-report:v2`
- Mode: `shadow`
- Comparator thresholds: `adr-0131-thresholds:v1`
- Corpus: `/private/tmp/dailyos-w4-b2/src-tauri/suites/E/canonicalization-thresholds`
- Pair count: 540

## Bucket Composition

| Bucket | Pairs | Target |
|---|---:|---:|
| `positive_paraphrases` | 200 | 37.04% |
| `hard_negatives` | 150 | 27.78% |
| `contradictions` | 75 | 13.89% |
| `asymmetric_qualifiers` | 50 | 9.26% |
| `low_trust_duplicates` | 25 | 4.63% |
| `tombstone_shadowed` | 10 | 1.85% |
| `cross_tier` | 10 | 1.85% |
| `cross_workspace` | 10 | 1.85% |
| `legacy_unmigrated` | 10 | 1.85% |

## Gate Metrics

| Metric | Value | Numerator | Denominator | Target |
|---|---:|---:|---:|---|
| `true_merge_precision` | 1.0000 | 225 | 225 | >= 0.98 on should_merge corpus |
| `true_merge_recall` | 1.0000 | 225 | 225 | >= 0.95 on should_merge corpus |
| `true_fork_recall` | 1.0000 | 240 | 240 | >= 0.95 on should_fork corpus |
| `contradiction_detection` | 1.0000 | 75 | 75 | >= 0.97 on should_contradict corpus |
| `false_merge_rate` | 0.0000 | 0 | 540 | <= 0.005; false merges are double-weighted at gate review |
| `ambiguous_rate_per_bucket` | 0.0000 | 0 | 540 | <= 0.05 per label bucket |
| `tombstone_bypass_rate` | 0.0000 | 0 | 10 | = 0 |
| `cross_tier_merge_rate` | 0.0000 | 0 | 10 | = 0 |
| `cross_account_merge_rate` | 0.0000 | 0 | 38 | = 0 |
| `cross_workspace_merge_rate` | 0.0000 | 0 | 10 | = 0 |
| `legacy_unmigrated_merge_rate` | 0.0000 | 0 | 10 | = 0 |

## Per-Bucket Expected vs V2

### `asymmetric_qualifiers` (50, target 9.26%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 50
- V2 decisions:
  - `fork`: 50

### `contradictions` (75, target 13.89%)
- Expected mismatches: 0
- Expected decisions:
  - `contradict`: 75
- V2 decisions:
  - `contradict`: 75

### `cross_tier` (10, target 1.85%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 10
- V2 decisions:
  - `fork`: 10

### `cross_workspace` (10, target 1.85%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 10
- V2 decisions:
  - `fork`: 10

### `hard_negatives` (150, target 27.78%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 150
- V2 decisions:
  - `fork`: 150

### `legacy_unmigrated` (10, target 1.85%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 10
- V2 decisions:
  - `fork`: 10

### `low_trust_duplicates` (25, target 4.63%)
- Expected mismatches: 0
- Expected decisions:
  - `merge`: 25
- V2 decisions:
  - `merge`: 25

### `positive_paraphrases` (200, target 37.04%)
- Expected mismatches: 0
- Expected decisions:
  - `merge`: 200
- V2 decisions:
  - `merge`: 200

### `tombstone_shadowed` (10, target 1.85%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 10
- V2 decisions:
  - `fork`: 10

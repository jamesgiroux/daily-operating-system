# Canonicalization Parity Report

- Schema: `canonicalization-parity-report:v2`
- Mode: `shadow`
- Comparator thresholds: `adr-0131-thresholds:v1`
- Corpus: `/private/tmp/dailyos-w4-b2/src-tauri/suites/E/canonicalization-thresholds`
- Pair count: 500

## Bucket Composition

| Bucket | Pairs | Target |
|---|---:|---:|
| `positive_paraphrases` | 200 | 40% |
| `hard_negatives` | 150 | 30% |
| `contradictions` | 75 | 15% |
| `asymmetric_qualifiers` | 50 | 10% |
| `low_trust_duplicates` | 25 | 5% |

## Gate Metrics

| Metric | Value | Numerator | Denominator | Target |
|---|---:|---:|---:|---|
| `true_merge_precision` | 1.0000 | 225 | 225 | >= 0.98 on should_merge corpus |
| `true_merge_recall` | 1.0000 | 225 | 225 | >= 0.95 on should_merge corpus |
| `true_fork_recall` | 1.0000 | 200 | 200 | >= 0.95 on should_fork corpus |
| `contradiction_detection` | 1.0000 | 75 | 75 | >= 0.97 on should_contradict corpus |
| `false_merge_rate` | 0.0000 | 0 | 500 | <= 0.005; false merges are double-weighted at gate review |
| `ambiguous_rate_per_bucket` | 0.0000 | 0 | 500 | <= 0.05 per label bucket |
| `tombstone_bypass_rate` | 0.0000 | 0 | 0 | = 0 |
| `cross_tier_merge_rate` | 0.0000 | 0 | 0 | = 0 |
| `cross_account_merge_rate` | 0.0000 | 0 | 38 | = 0 |
| `cross_workspace_merge_rate` | 0.0000 | 0 | 0 | = 0 |
| `legacy_unmigrated_merge_rate` | 0.0000 | 0 | 0 | = 0 |

## Per-Bucket Expected vs V2

### `asymmetric_qualifiers` (50, target 10%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 50
- V2 decisions:
  - `fork`: 50

### `contradictions` (75, target 15%)
- Expected mismatches: 0
- Expected decisions:
  - `contradict`: 75
- V2 decisions:
  - `contradict`: 75

### `hard_negatives` (150, target 30%)
- Expected mismatches: 0
- Expected decisions:
  - `fork`: 150
- V2 decisions:
  - `fork`: 150

### `low_trust_duplicates` (25, target 5%)
- Expected mismatches: 0
- Expected decisions:
  - `merge`: 25
- V2 decisions:
  - `merge`: 25

### `positive_paraphrases` (200, target 40%)
- Expected mismatches: 0
- Expected decisions:
  - `merge`: 200
- V2 decisions:
  - `merge`: 200

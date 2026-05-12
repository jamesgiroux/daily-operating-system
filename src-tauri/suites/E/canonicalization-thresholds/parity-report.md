# Canonicalization Parity Report

- Schema: `canonicalization-parity-report:v1`
- Mode: `shadow`
- Comparator thresholds: `adr-0131-thresholds:v1`
- Corpus: `/private/tmp/dailyos-w4-b2/src-tauri/suites/E/canonicalization-thresholds`
- Pair count: 10

## Bucket Composition

| Bucket | Pairs | Target |
|---|---:|---:|
| `positive_paraphrases` | 2 | 40% |
| `hard_negatives` | 2 | 30% |
| `contradictions` | 2 | 15% |
| `asymmetric_qualifiers` | 2 | 10% |
| `low_trust_duplicates` | 2 | 5% |

## Gate Metrics

| Metric | Value | Numerator | Denominator | Target |
|---|---:|---:|---:|---|
| `true_merge_precision` | 1.0000 | 2 | 2 | >= 0.98 on should_merge corpus |
| `true_merge_recall` | 1.0000 | 2 | 2 | >= 0.95 on should_merge corpus |
| `true_fork_recall` | 1.0000 | 6 | 6 | >= 0.95 on should_fork corpus |
| `contradiction_detection` | 1.0000 | 2 | 2 | >= 0.97 on should_contradict corpus |
| `false_merge_rate` | 0.0000 | 0 | 10 | <= 0.005; false merges are double-weighted at gate review |
| `ambiguous_rate_per_bucket` | 0.0000 | 0 | 10 | <= 0.05 per label bucket |
| `tombstone_bypass_rate` | 0.0000 | 0 | 1 | = 0 |
| `cross_tier_merge_rate` | 0.0000 | 0 | 1 | = 0 |
| `cross_account_merge_rate` | 0.0000 | 0 | 1 | = 0 |
| `cross_workspace_merge_rate` | 0.0000 | 0 | 1 | = 0 |
| `legacy_unmigrated_merge_rate` | 0.0000 | 0 | 1 | = 0 |

## Divergence Counts

| Divergence | Pairs |
|---|---:|
| `v1_fork_v2_contradict` | 2 |
| `v1_fork_v2_fork` | 5 |
| `v1_fork_v2_merge` | 1 |
| `v1_merge_v2_fork` | 1 |
| `v1_merge_v2_merge` | 1 |

## Per-Bucket Divergence

### `asymmetric_qualifiers` (2, target 10%)
- `v1_fork_v2_fork`: 2

### `contradictions` (2, target 15%)
- `v1_fork_v2_contradict`: 2

### `hard_negatives` (2, target 30%)
- `v1_fork_v2_fork`: 1
- `v1_merge_v2_fork`: 1

### `low_trust_duplicates` (2, target 5%)
- `v1_fork_v2_fork`: 2

### `positive_paraphrases` (2, target 40%)
- `v1_fork_v2_merge`: 1
- `v1_merge_v2_merge`: 1

# Canonicalization Parity Report

- Schema: `canonicalization-parity-report:v1`
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
| `true_merge_precision` | 1.0000 | 200 | 200 | >= 0.98 on should_merge corpus |
| `true_merge_recall` | 1.0000 | 200 | 200 | >= 0.95 on should_merge corpus |
| `true_fork_recall` | 0.8100 | 162 | 200 | >= 0.95 on should_fork corpus |
| `contradiction_detection` | 0.0000 | 0 | 75 | >= 0.97 on should_contradict corpus |
| `false_merge_rate` | 0.0000 | 0 | 500 | <= 0.005; false merges are double-weighted at gate review |
| `ambiguous_rate_per_bucket` | 0.0000 | 0 | 500 | <= 0.05 per label bucket |
| `tombstone_bypass_rate` | 0.0000 | 0 | 0 | = 0 |
| `cross_tier_merge_rate` | 0.0000 | 0 | 0 | = 0 |
| `cross_account_merge_rate` | 0.0000 | 0 | 38 | = 0 |
| `cross_workspace_merge_rate` | 0.0000 | 0 | 0 | = 0 |
| `legacy_unmigrated_merge_rate` | 0.0000 | 0 | 25 | = 0 |

## Divergence Counts

| Divergence | Pairs |
|---|---:|
| `v1_fork_v2_contradict` | 38 |
| `v1_fork_v2_fork` | 257 |
| `v1_fork_v2_merge` | 193 |
| `v1_merge_v2_fork` | 5 |
| `v1_merge_v2_merge` | 7 |

## Per-Bucket Divergence

### `asymmetric_qualifiers` (50, target 10%)
- `v1_fork_v2_fork`: 50

### `contradictions` (75, target 15%)
- `v1_fork_v2_fork`: 75

### `hard_negatives` (150, target 30%)
- `v1_fork_v2_contradict`: 38
- `v1_fork_v2_fork`: 112

### `low_trust_duplicates` (25, target 5%)
- `v1_fork_v2_fork`: 20
- `v1_merge_v2_fork`: 5

### `positive_paraphrases` (200, target 40%)
- `v1_fork_v2_merge`: 193
- `v1_merge_v2_merge`: 7

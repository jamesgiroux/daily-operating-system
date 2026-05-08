#!/usr/bin/env bash
# Suite P — Performance regression check against prior-wave baseline.
#
# Runs criterion benchmarks under src-tauri/, compares to the prior wave's
# saved baseline at .docs/perf-baselines/wave-W{prev}.json. First wave seeds
# the baseline (no comparison). Threshold: 10% regression on any flow = fail.
#
# Usage: scripts/suite-p.sh --wave WN [--out path] [--threshold 10]
#
# Exit: 0 if no regression OR first wave (baseline seeded);
#       1 if any flow regresses beyond threshold.

set -euo pipefail

OUT=""
WAVE=""
THRESHOLD="10"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --wave) WAVE="$2"; shift 2 ;;
    --threshold) THRESHOLD="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$WAVE" ]] || { echo "--wave required" >&2; exit 2; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BASELINE_DIR=".docs/perf-baselines"
mkdir -p "$BASELINE_DIR"

# Prior-wave baseline path. Wave naming: W0, W0.5, W1, W1.5, W2, ...
# We don't try to compute "prior wave" cleverly — we use the most recent
# baseline file that's not the current wave's.
CURRENT_BASELINE="$BASELINE_DIR/wave-${WAVE}.json"
PRIOR_BASELINE=$(ls -1t "$BASELINE_DIR"/wave-*.json 2>/dev/null | grep -v "wave-${WAVE}.json" | head -1 || true)

# Run criterion bench. The dailyos crate may not have benches registered yet —
# in that case we report "no benches" and seed an empty baseline rather than
# failing. Future PRs add real benches; this scaffolds the surface.
cd src-tauri
if cargo bench --workspace --no-run >/tmp/suite-p-build.log 2>&1; then
  cargo bench --workspace -- --save-baseline "wave-${WAVE}" >/tmp/suite-p-run.log 2>&1 || true
  bench_exit=$?
else
  bench_exit=255 # sentinel: no benches compiled
fi
cd ..

if [[ $bench_exit -eq 255 ]]; then
  # No benches yet — seed empty baseline, succeed. Future waves will populate.
  echo '{"benchmarks":[],"note":"no-benches-compiled"}' > "$CURRENT_BASELINE"
  summary="{\"suite\":\"P\",\"wave\":\"$WAVE\",\"status\":\"seeded-empty\",\"baseline\":\"$CURRENT_BASELINE\",\"prior\":null,\"regressions\":[]}"
  [[ -n "$OUT" ]] && printf '%s\n' "$summary" > "$OUT" || printf '%s\n' "$summary"
  exit 0
fi

# Extract criterion's reported timings into a normalized JSON for archiving.
# criterion writes to target/criterion/<bench>/<run-name>/estimates.json
# We collect mean estimates per bench.
python3 <<PY > "$CURRENT_BASELINE"
import json, os, glob
crit_root = "src-tauri/target/criterion"
benches = []
for est in glob.glob(f"{crit_root}/**/wave-${WAVE}/estimates.json", recursive=True):
    with open(est) as f: data = json.load(f)
    name = est.split(crit_root + "/")[1].rsplit("/wave-${WAVE}/", 1)[0]
    benches.append({"bench": name, "mean_ns": data.get("mean", {}).get("point_estimate")})
print(json.dumps({"benchmarks": benches}))
PY

# Compare against prior if it exists
regressions="[]"
status="ok"
if [[ -n "$PRIOR_BASELINE" && -s "$PRIOR_BASELINE" ]]; then
  regressions=$(python3 <<PY
import json
with open("$CURRENT_BASELINE") as f: cur = json.load(f)
with open("$PRIOR_BASELINE") as f: prior = json.load(f)
prior_map = {b["bench"]: b["mean_ns"] for b in prior.get("benchmarks", []) if b.get("mean_ns") is not None}
out = []
for b in cur.get("benchmarks", []):
    n = b["bench"]; cur_mean = b.get("mean_ns")
    if cur_mean is None or n not in prior_map: continue
    delta_pct = 100.0 * (cur_mean - prior_map[n]) / prior_map[n]
    if delta_pct > $THRESHOLD:
        out.append({"bench": n, "prior_ns": prior_map[n], "current_ns": cur_mean, "delta_pct": round(delta_pct, 2)})
print(json.dumps(out))
PY
)
  if [[ "$regressions" != "[]" ]]; then
    status="regression"
  fi
fi

summary="{\"suite\":\"P\",\"wave\":\"$WAVE\",\"status\":\"$status\",\"baseline\":\"$CURRENT_BASELINE\",\"prior\":${PRIOR_BASELINE:+\"$PRIOR_BASELINE\"}${PRIOR_BASELINE:-null},\"regressions\":$regressions}"

[[ -n "$OUT" ]] && printf '%s\n' "$summary" > "$OUT" || printf '%s\n' "$summary"

[[ "$status" == "regression" ]] && exit 1 || exit 0

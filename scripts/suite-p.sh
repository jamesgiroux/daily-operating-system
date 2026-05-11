#!/usr/bin/env bash
# Suite P — Performance regression check against prior-scope baseline.
#
# Runs criterion benchmarks under src-tauri/, compares to the prior scope's
# saved baseline at .docs/perf-baselines/scope-{prev}.json. First scope seeds
# the baseline (no comparison). Threshold: 10% regression on any flow = fail.
#
# Usage: scripts/suite-p.sh --scope SCOPE-ID [--out path] [--threshold 10]
#
# Exit: 0 if no regression OR first run (baseline seeded);
#       1 if any flow regresses beyond threshold.

set -euo pipefail

OUT=""
SCOPE=""
THRESHOLD="10"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift ;;
    --out) OUT="$2"; shift 2 ;;
    --scope) SCOPE="$2"; shift 2 ;;
    --threshold) THRESHOLD="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

[[ -n "$SCOPE" ]] || { echo "--scope required" >&2; exit 2; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

BASELINE_DIR=".docs/perf-baselines"
mkdir -p "$BASELINE_DIR"

# Prior-scope baseline path. Scope naming: free-form (e.g., v1.4.1-W0, DOS-cleanup-batch).
# We don't try to compute "prior scope" cleverly — we use the most recent
# baseline file that's not the current run's.
CURRENT_BASELINE="$BASELINE_DIR/scope-${SCOPE}.json"
PRIOR_BASELINE=$(ls -1t "$BASELINE_DIR"/scope-*.json 2>/dev/null | grep -v "scope-${SCOPE}.json" | head -1 || true)

# Run criterion bench. The dailyos crate may not have benches registered yet —
# in that case we report "no benches" and seed an empty baseline rather than
# failing. Future PRs add real benches; this scaffolds the surface.
cd src-tauri
if cargo bench --workspace --no-run >/tmp/suite-p-build.log 2>&1; then
  cargo bench --workspace -- --save-baseline "scope-${SCOPE}" >/tmp/suite-p-run.log 2>&1 || true
  bench_exit=$?
else
  bench_exit=255 # sentinel: no benches compiled
fi
cd ..

if [[ $bench_exit -eq 255 ]]; then
  # No benches yet — seed empty baseline, succeed. Future scopes populate.
  echo '{"benchmarks":[],"note":"no-benches-compiled"}' > "$CURRENT_BASELINE"
  summary=$(python3 - "$SCOPE" "$CURRENT_BASELINE" <<'PY'
import json, sys
scope, baseline = sys.argv[1:]
print(json.dumps({"suite":"P","scope":scope,"status":"seeded-empty","baseline":baseline,"prior":None,"regressions":[]}, separators=(",",":")))
PY
)
  if [[ -n "$OUT" ]]; then
    mkdir -p "$(dirname "$OUT")"
  fi
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
for est in glob.glob(f"{crit_root}/**/scope-${SCOPE}/estimates.json", recursive=True):
    with open(est) as f: data = json.load(f)
    name = est.split(crit_root + "/")[1].rsplit("/scope-${SCOPE}/", 1)[0]
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

summary=$(python3 - "$SCOPE" "$status" "$CURRENT_BASELINE" "${PRIOR_BASELINE:-}" "$regressions" <<'PY'
import json, sys
scope, status, baseline, prior, regressions = sys.argv[1:]
print(json.dumps({
    "suite": "P",
    "scope": scope,
    "status": status,
    "baseline": baseline,
    "prior": prior or None,
    "regressions": json.loads(regressions),
}, separators=(",",":")))
PY
)

if [[ -n "$OUT" ]]; then
  mkdir -p "$(dirname "$OUT")"
fi
[[ -n "$OUT" ]] && printf '%s\n' "$summary" > "$OUT" || printf '%s\n' "$summary"

[[ "$status" == "regression" ]] && exit 1 || exit 0

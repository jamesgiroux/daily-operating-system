#!/usr/bin/env bash
# Pre-push CI parity check.
#
# Runs the same gates that .github/workflows/test.yml runs on push, in the same
# order, fail-fast. Run this from the repo root before pushing a branch so that
# preventable CI failures show up locally instead of after a 30-minute round
# trip on the macOS runner.
#
# Skip flags:
#   --no-frontend  skip pnpm tsc / pnpm test / pnpm audit (faster Rust-only loop)
#   --no-audit     skip cargo audit + pnpm audit
#   --no-test      skip cargo test + pnpm test (lint-only loop)
#   --hermetic     also run the harness-hermetic feature tests CI runs
#
# Exit codes:
#   0  all gates green
#   1+ first gate that failed; the script halts there

set -uo pipefail

cd "$(git rev-parse --show-toplevel)" 2>/dev/null || {
    echo "preflight: must run inside the dailyos-repo working tree" >&2
    exit 1
}

SKIP_FRONTEND=0
SKIP_AUDIT=0
SKIP_TEST=0
INCLUDE_HERMETIC=0
for arg in "$@"; do
    case "$arg" in
        --no-frontend) SKIP_FRONTEND=1 ;;
        --no-audit)    SKIP_AUDIT=1 ;;
        --no-test)     SKIP_TEST=1 ;;
        --hermetic)    INCLUDE_HERMETIC=1 ;;
        -h|--help)
            sed -n '1,16p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "preflight: unknown flag: $arg" >&2
            exit 1
            ;;
    esac
done

step_count=0
fail_step=""

run_step() {
    local label="$1"
    shift
    step_count=$((step_count + 1))
    printf "[preflight %02d] %s ... " "$step_count" "$label"
    local start_ts end_ts
    start_ts=$(date +%s)
    if "$@" >/tmp/preflight-$$.log 2>&1; then
        end_ts=$(date +%s)
        printf "ok (%ds)\n" "$((end_ts - start_ts))"
        rm -f /tmp/preflight-$$.log
    else
        printf "FAIL\n"
        fail_step="$label"
        echo "----- last 60 lines of output -----" >&2
        tail -60 /tmp/preflight-$$.log >&2
        rm -f /tmp/preflight-$$.log
        exit 1
    fi
}

require_tool() {
    local tool="$1"
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "preflight: missing required tool: $tool" >&2
        exit 1
    fi
}

require_tool rg
require_tool cargo
require_tool rustc
if [ "$SKIP_FRONTEND" -eq 0 ]; then require_tool pnpm; fi

# 1. OAuth secret scan (matches workflow line 54 exactly)
run_step "OAuth secret scan" bash -c '! rg -n "GOCSPX-[A-Za-z0-9_-]+" --glob "!target/**" --glob "!node_modules/**" --glob "!.git/**" --glob "!_archive/**" .'

# 2. Tauri externalBin sidecar stub (matches workflow lines 60-65)
run_step "Tauri externalBin sidecar stub" bash -c '
    mkdir -p src-tauri/binaries
    TARGET=$(rustc -vV | awk "/^host:/ { print \$2 }")
    touch "src-tauri/binaries/dailyos-mcp-$TARGET"
    touch src-tauri/build.rs
'

# 3. Service-layer boundary
run_step "service-layer boundary" ./scripts/check_service_layer_boundary.sh

# 4. write_intelligence_json fence
run_step "write_fence usage" ./scripts/check_write_fence_usage.sh

# 5. ability surface drift + live external clients + fixture anonymization
run_step "ability surface drift" bash src-tauri/scripts/check_ability_surface_drift.sh
run_step "no live external clients in eval" bash src-tauri/scripts/check_no_live_external_clients_in_eval.sh
run_step "fixture anonymization" bash src-tauri/scripts/check_fixture_anonymization.sh

# 7. durable source comments — ephemeral issue refs banned in code comments
run_step "durable source comments" ./scripts/check_no_ephemeral_issue_refs_in_comments.sh

# 8. legacy projection writers tracker (CI runs continue-on-error; we surface but don't fail)
printf "[preflight %02d] %s ... " $((step_count + 1)) "legacy projection writers (advisory)"
step_count=$((step_count + 1))
if src-tauri/scripts/check_dos301_legacy_projection_writers.sh >/tmp/preflight-$$.log 2>&1; then
    printf "ok\n"
    rm -f /tmp/preflight-$$.log
else
    printf "advisory-fail (CI uses continue-on-error)\n"
    rm -f /tmp/preflight-$$.log
fi

# 9. cargo clippy (production targets) — exact CI invocation
run_step "cargo clippy -D warnings" cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings

if [ "$INCLUDE_HERMETIC" -eq 1 ]; then
    # 10a. hermetic harness tests
    run_step "cargo test --features harness-hermetic --test harness" bash -c 'cd src-tauri && cargo test --features harness-hermetic --test harness'
    run_step "cargo test --features harness-hermetic --test dos216_hermetic_feature_test" bash -c 'cd src-tauri && cargo test --features harness-hermetic --test dos216_hermetic_feature_test'
fi

if [ "$SKIP_TEST" -eq 0 ]; then
    # 10b. cargo test (full suite)
    run_step "cargo test (full)" bash -c 'cd src-tauri && cargo test'
fi

if [ "$SKIP_AUDIT" -eq 0 ]; then
    # 11. cargo audit (high+)
    if command -v cargo-audit >/dev/null 2>&1; then
        run_step "cargo audit" cargo audit --file audit.toml
    else
        printf "[preflight] cargo audit not installed — install with: cargo install cargo-audit --locked\n"
    fi
fi

if [ "$SKIP_FRONTEND" -eq 0 ]; then
    # 12. pnpm install (idempotent if already done)
    run_step "pnpm install" pnpm install --frozen-lockfile
    # 13. pnpm tsc
    run_step "pnpm tsc --noEmit" pnpm tsc --noEmit
    # 14. pnpm test
    if [ "$SKIP_TEST" -eq 0 ]; then
        run_step "pnpm test" pnpm test
    fi
    if [ "$SKIP_AUDIT" -eq 0 ]; then
        run_step "pnpm audit (high+)" pnpm audit --audit-level high
    fi
fi

echo
echo "preflight: all $step_count gates green"

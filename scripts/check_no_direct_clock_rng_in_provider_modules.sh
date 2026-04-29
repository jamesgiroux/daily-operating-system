#!/usr/bin/env bash
# DOS-259 (W2-B): forbid direct `Utc::now()` / `thread_rng()` in the new
# provider modules so they remain mode-aware (clock/RNG come through
# `ServiceContext` once W2-A lands; until then, providers must not anchor
# to wall clock or system RNG by accident).
#
# Coverage: intelligence/{provider,pty_provider,glean_provider}.rs only.
# The wave invariant for W2-A already covers `services/` and `abilities/`;
# this lint closes the gap for the W2-B-introduced provider modules.
#
# Pattern matches any of:
#   chrono::Utc::now(           Utc::now(           chrono::offset::Utc::now(
#   rand::thread_rng(           thread_rng(         rand::rng(
#
# Test files (`#[cfg(test)] mod tests { ... }`) are excluded — fixture-time
# clock/RNG calls are fine. The lint walks each file and skips lines that
# fall inside a `#[cfg(test)]` block. Inline `// dos259-exempt: <reason>`
# markers bypass the lint for documented edge cases.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

FILES=(
  "$ROOT_DIR/src-tauri/src/intelligence/provider.rs"
  "$ROOT_DIR/src-tauri/src/intelligence/pty_provider.rs"
  "$ROOT_DIR/src-tauri/src/intelligence/glean_provider.rs"
)

# Grandfathered allowlist: pre-W2-B `Utc::now()` calls in `glean_provider.rs`
# that timestamp serializable response / manifest fields. These migrate to
# `ctx.clock.now()` when W2-A's `ServiceContext` lands and the Glean impl
# can take a clock reference. Until then the lint accepts existing line
# numbers; any NEW clock call (added line, or shifted to a non-allowlisted
# line) trips the lint.
#
# When W2-A migrates these, this allowlist deletes entirely.
GLEAN_GRANDFATHERED_LINES=(292 470 486 525 622 1023 1364 1815)

# Pattern: clock or RNG call, in any qualified form.
PATTERN='\b(chrono::offset::Utc::now|chrono::Utc::now|Utc::now|rand::thread_rng|thread_rng|rand::rng)[[:space:]]*\('

violations=0
for file in "${FILES[@]}"; do
  [ -f "$file" ] || continue

  # Build a list of byte spans that are inside #[cfg(test)] mod blocks
  # using a simple line-level scanner. This is the same shape as the
  # W1/W2-A lints — accurate enough for the small file set.
  in_cfg_test=0
  brace_depth=0
  cfg_test_start_depth=0
  lineno=0

  while IFS= read -r line; do
    lineno=$((lineno + 1))

    # Track brace depth (fast — counts { and } per line).
    open_braces=$(echo "$line" | tr -cd '{' | wc -c | tr -d ' ')
    close_braces=$(echo "$line" | tr -cd '}' | wc -c | tr -d ' ')

    # Detect entering a #[cfg(test)] mod block.
    if [ "$in_cfg_test" -eq 0 ] && echo "$line" | grep -qE '^\s*#\[cfg\(test\)\]'; then
      # Look at next non-blank/non-attribute line — if it starts a `mod ... {`,
      # we're entering a cfg(test) block. For simplicity, mark as candidate
      # and confirm on the next opening brace.
      cfg_test_pending=1
      continue
    fi
    if [ -n "${cfg_test_pending:-}" ]; then
      if echo "$line" | grep -qE '^\s*(pub\s+)?mod\s+\w+\s*\{'; then
        in_cfg_test=1
        cfg_test_start_depth=$brace_depth
        brace_depth=$((brace_depth + open_braces - close_braces))
        cfg_test_pending=
        continue
      else
        cfg_test_pending=
      fi
    fi

    brace_depth=$((brace_depth + open_braces - close_braces))

    # Detect leaving a #[cfg(test)] mod block.
    if [ "$in_cfg_test" -eq 1 ] && [ "$brace_depth" -le "$cfg_test_start_depth" ]; then
      in_cfg_test=0
    fi

    # Skip lines inside cfg(test).
    [ "$in_cfg_test" -eq 1 ] && continue

    # Skip lines marked exempt within the previous 3 lines.
    if echo "$line" | grep -qE "$PATTERN"; then
      start=$((lineno - 3))
      [ "$start" -lt 1 ] && start=1
      if sed -n "${start},${lineno}p" "$file" 2>/dev/null \
          | grep -q "dos259-exempt:"; then
        continue
      fi
      # Grandfathered glean_provider.rs lines (W2-A will migrate).
      if [[ "$file" == *"intelligence/glean_provider.rs" ]]; then
        skip=0
        for gf_line in "${GLEAN_GRANDFATHERED_LINES[@]}"; do
          if [ "$gf_line" -eq "$lineno" ]; then
            skip=1
            break
          fi
        done
        [ "$skip" -eq 1 ] && continue
      fi
      printf '%s:%s:%s\n' "$file" "$lineno" "$line"
      violations=$((violations + 1))
    fi
  done < "$file"
done

if [ "$violations" -gt 0 ]; then
  echo
  echo "ERROR: ${violations} direct clock/RNG call(s) in DOS-259 provider modules."
  echo "Provider modules must not anchor to wall clock or system RNG directly."
  echo "Route through ServiceContext.clock / ServiceContext.rng once W2-A lands."
  echo "If intentionally needed (e.g., glean_chat retry jitter), add a"
  echo "  // dos259-exempt: <reason>"
  echo "comment within 3 lines above the call."
  exit 1
fi

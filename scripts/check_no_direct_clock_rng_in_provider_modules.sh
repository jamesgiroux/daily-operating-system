#!/usr/bin/env bash
# Forbid direct `Utc::now` / `thread_rng` in provider modules so they remain
# mode-aware. Providers should get clock/RNG through `ServiceContext`; until
# that wiring is available, they must not anchor to wall clock or system RNG by
# accident.
#
# Coverage: intelligence/{provider,pty_provider,glean_provider}.rs only.
# The existing clock/RNG invariant already covers `services/` and `abilities/`;
# this lint closes the provider-module gap.
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

if [ -n "${DOS259_LINT_FILES_OVERRIDE:-}" ]; then
  # Test seam: tests pass `DOS259_LINT_FILES_OVERRIDE` (colon-separated
  # absolute paths) to drive the lint against synthetic fixture files.
  # Production callers leave the env var unset and get the canonical
  # provider-module list below.
  IFS=':' read -r -a FILES <<< "$DOS259_LINT_FILES_OVERRIDE"
else
  FILES=(
    "$ROOT_DIR/src-tauri/src/intelligence/provider.rs"
    "$ROOT_DIR/src-tauri/src/intelligence/pty_provider.rs"
    "$ROOT_DIR/src-tauri/src/intelligence/glean_provider.rs"
  )
fi

# Grandfathered allowlist: existing `Utc::now()` calls in `glean_provider.rs`
# that timestamp serializable response / manifest fields. These migrate to
# `ctx.clock.now()` when the Glean implementation can take a clock reference.
# Until then the lint accepts each call annotated with a
# `// dos259-grandfathered: <reason>` marker within 3 lines above. Line-number
# allowlists rot on file edits; the marker is stable across edits.
#
# When those call sites migrate, the markers and this comment block delete.

# Pattern: clock or RNG call, in any qualified form.
PATTERN='\b(chrono::offset::Utc::now|chrono::Utc::now|Utc::now|rand::thread_rng|thread_rng|rand::rng)[[:space:]]*\('

violations=0
for file in "${FILES[@]}"; do
  [ -f "$file" ] || continue

  # Build a list of byte spans that are inside #[cfg(test)] mod blocks
  # using a simple line-level scanner. This is the same shape as the
  # Existing lints use this same line-level scanner; it is accurate enough for
  # the small file set.
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
      preceding="$(sed -n "${start},${lineno}p" "$file" 2>/dev/null)"
      # New code uses `dos259-exempt:` markers; grandfathered
      # calls (currently in glean_provider.rs only) use
      # `dos259-grandfathered:` markers. Both bypass the lint. Markers
      # are content-stable across line shifts (per L2 codex review).
      if echo "$preceding" | grep -q "dos259-exempt:" \
        || echo "$preceding" | grep -q "dos259-grandfathered:"; then
        continue
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
  echo "Route through ServiceContext.clock / ServiceContext.rng once the provider wiring supports it."
  echo "If intentionally needed (e.g., glean_chat retry jitter), add a"
  echo "  // dos259-exempt: <reason>"
  echo "comment within 3 lines above the call."
  exit 1
fi

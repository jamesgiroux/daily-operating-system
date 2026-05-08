#!/usr/bin/env bash
set -euo pipefail

# external replay guard.
#
# Eval/test code must use replay-backed external clients. This lint scans
# integration tests plus ability codepaths, all of which may be invoked by
# ExecutionMode::Evaluate through the ability registry.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [[ -n "${DOS383_EXTERNAL_CLIENT_LINT_ROOT_OVERRIDE:-}" ]]; then
  ROOT_DIR="$DOS383_EXTERNAL_CLIENT_LINT_ROOT_OVERRIDE"
elif [[ -d "src-tauri" ]]; then
  ROOT_DIR="$(pwd)"
elif [[ "$(basename "$(pwd)")" == "src-tauri" && -d "src" && -d "tests" ]]; then
  ROOT_DIR="$(cd .. && pwd)"
else
  ROOT_DIR="$DEFAULT_REPO_ROOT"
fi

scan_roots=()
for root in "$ROOT_DIR/src-tauri/tests" "$ROOT_DIR/src-tauri/src/abilities"; do
  if [[ -d "$root" ]]; then
    scan_roots+=("$root")
  fi
done

labels=(
  "external HTTP client constructor"
  "std::net raw socket"
  "std::net imported raw socket"
  "TcpListener raw socket"
  "TcpStream raw socket"
  "UdpSocket raw socket"
  "tokio::net raw socket"
  "tokio::net imported raw socket"
)

patterns=(
  "(^|[^[:alnum:]_])([A-Za-z_][A-Za-z0-9_]*::)*([A-Za-z_][A-Za-z0-9_]*Client|Client)::(new|builder)[[:space:]]*\\("
  "std::net::[A-Za-z0-9_:]+"
  "use[[:space:]]+std::net(::|[[:space:]]*\\{)"
  "(^|[^[:alnum:]_])TcpListener::(bind|from_std)[[:space:]]*\\("
  "(^|[^[:alnum:]_])TcpStream::(connect|from_std)[[:space:]]*\\("
  "(^|[^[:alnum:]_])UdpSocket::(bind|from_std)[[:space:]]*\\("
  "tokio::net::[A-Za-z0-9_:]+"
  "use[[:space:]]+tokio::net(::|[[:space:]]*\\{)"
)

allowed_external_constructor() {
  local rel_path="$1"
  local line="$2"

  if [[ "$rel_path" == "src-tauri/src/services/context.rs" ]]; then
    [[ "$line" =~ (^|[^[:alnum:]_:])((crate::|dailyos_lib::)?services::context::)?ExternalClients::default[[:space:]]*\( ]] && return 0
    [[ "$line" =~ (^|[^[:alnum:]_])Replay(Glean|Slack|Gmail|Salesforce)Client::new[[:space:]]*\( ]] && return 0
  fi

  return 1
}

reqwest_alias_patterns() {
  local file="$1"
  local aliases

  aliases="$(
    grep -oE 'use[[:space:]]+reqwest::Client([[:space:]]+as[[:space:]]+[A-Za-z_][A-Za-z0-9_]*)?[[:space:]]*;' "$file" 2>/dev/null \
      | sed -E 's/.*as[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*;/\1/' \
      | sed -E 's/^use[[:space:]]+reqwest::Client[[:space:]]*;$/Client/' || true
    grep -oE 'type[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*=[[:space:]]*reqwest::Client[[:space:]]*;' "$file" 2>/dev/null \
      | sed -E 's/type[[:space:]]+([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*=.*/\1/' || true
  )"

  while IFS= read -r alias; do
    [[ -z "$alias" ]] && continue
    printf '(^|[^[:alnum:]_])%s::(new|builder)[[:space:]]*\\(\n' "$alias"
  done <<< "$aliases"
}

violations=""

if [[ "${#scan_roots[@]}" -gt 0 ]]; then
  while IFS= read -r -d '' file; do
    rel_path="${file#$ROOT_DIR/}"

    for i in "${!patterns[@]}"; do
      found="$(grep -nE "${patterns[$i]}" "$file" 2>/dev/null || true)"
      if [[ -z "$found" ]]; then
        continue
      fi

      while IFS= read -r match; do
        [[ -z "$match" ]] && continue
        line_no="${match%%:*}"
        line="${match#*:}"
        if allowed_external_constructor "$rel_path" "$line"; then
          continue
        fi
        violations+="${rel_path}:${line_no}:${labels[$i]}: ${line}"$'\n'
      done <<< "$found"
    done

    while IFS= read -r alias_pattern; do
      [[ -z "$alias_pattern" ]] && continue
      found="$(grep -nE "$alias_pattern" "$file" 2>/dev/null || true)"
      if [[ -z "$found" ]]; then
        continue
      fi

      while IFS= read -r match; do
        [[ -z "$match" ]] && continue
        line_no="${match%%:*}"
        line="${match#*:}"
        if allowed_external_constructor "$rel_path" "$line"; then
          continue
        fi
        violations+="${rel_path}:${line_no}:external HTTP client constructor: ${line}"$'\n'
      done <<< "$found"
    done < <(reqwest_alias_patterns "$file")
  done < <(find "${scan_roots[@]}" -type f -name '*.rs' -print0)
fi

if [[ -n "$violations" ]]; then
  echo "Live external client constructors are forbidden in eval/test paths."
  echo "Use replay-backed ExternalClients from ServiceContext."
  echo
  echo "Allowed constructors:"
  echo "  - services::context::ExternalClients::default()"
  echo "  - replay wrapper constructors in services::context"
  echo
  echo "Violations:"
  printf "%s" "$violations"
  exit 1
fi

echo "No live external client constructors found in eval/test paths."

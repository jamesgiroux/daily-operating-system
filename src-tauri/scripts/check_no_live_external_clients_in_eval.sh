#!/usr/bin/env bash
set -euo pipefail

# DOS-383 external replay guard.
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
  "reqwest::Client::new"
  "reqwest::Client::builder"
  "Glean::new"
  "glean_api::client::Client::new"
  "glean::Client::new"
  "GleanMcpClient::new"
  "slack_morphism::Client::new"
  "slack_morphism::SlackClient::new"
  "SlackClient::new"
  "gmail::Client::new"
  "GmailClient::new"
  "REDACTED::Client::new"
  "SalesforceClient::new"
  "google_drive::Client::new"
  "GoogleDriveClient::new"
  "LinearClient::new"
  "ClayClient::new"
  "std::net::*"
  "std::net imported raw socket"
  "TcpListener raw socket"
  "TcpStream raw socket"
  "UdpSocket raw socket"
  "tokio::net::*"
  "tokio::net imported raw socket"
)

patterns=(
  "reqwest::Client::new[[:space:]]*\\("
  "reqwest::Client::builder[[:space:]]*\\("
  "(^|[^[:alnum:]_])Glean::new[[:space:]]*\\("
  "glean_api::client::Client::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])glean::Client::new[[:space:]]*\\("
  "GleanMcpClient::new[[:space:]]*\\("
  "slack_morphism::Client::new[[:space:]]*\\("
  "slack_morphism::SlackClient::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])SlackClient::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])gmail::Client::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])GmailClient::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])REDACTED::Client::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])SalesforceClient::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])google_drive::Client::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])GoogleDriveClient::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])LinearClient::new[[:space:]]*\\("
  "(^|[^[:alnum:]_])ClayClient::new[[:space:]]*\\("
  "std::net::[A-Za-z0-9_:]+"
  "use[[:space:]]+std::net(::|[[:space:]]*\\{)"
  "(^|[^[:alnum:]_])TcpListener::(bind|from_std)[[:space:]]*\\("
  "(^|[^[:alnum:]_])TcpStream::(connect|from_std)[[:space:]]*\\("
  "(^|[^[:alnum:]_])UdpSocket::(bind|from_std)[[:space:]]*\\("
  "tokio::net::[A-Za-z0-9_:]+"
  "use[[:space:]]+tokio::net(::|[[:space:]]*\\{)"
)

has_file_exemption() {
  local file="$1"
  grep -qE '^[[:space:]]*//[[:space:]]*LINT-ALLOW:[[:space:]]*live-external-client[[:space:]]+\(justification:[[:space:]]*[^)]+' "$file"
}

violations=""

if [[ "${#scan_roots[@]}" -gt 0 ]]; then
  while IFS= read -r -d '' file; do
    rel_path="${file#$ROOT_DIR/}"

    case "$rel_path" in
      src-tauri/src/services/context.rs)
        continue
        ;;
    esac

    if has_file_exemption "$file"; then
      continue
    fi

    for i in "${!patterns[@]}"; do
      found="$(grep -nE "${patterns[$i]}" "$file" 2>/dev/null || true)"
      if [[ -z "$found" ]]; then
        continue
      fi

      while IFS= read -r match; do
        [[ -z "$match" ]] && continue
        line_no="${match%%:*}"
        line="${match#*:}"
        violations+="${rel_path}:${line_no}:${labels[$i]}: ${line}"$'\n'
      done <<< "$found"
    done
  done < <(find "${scan_roots[@]}" -type f -name '*.rs' -print0)
fi

if [[ -n "$violations" ]]; then
  echo "Live external client constructors are forbidden in eval/test paths."
  echo "Use replay clients through ServiceContext / ExternalClients::from_replay."
  echo
  echo "Allowed exemptions:"
  echo "  - src-tauri/src/services/context.rs owns the Live/Replay split"
  echo "  - file-level // LINT-ALLOW: live-external-client (justification: ...)"
  echo
  echo "Violations:"
  printf "%s" "$violations"
  exit 1
fi

echo "No live external client constructors found in eval/test paths."

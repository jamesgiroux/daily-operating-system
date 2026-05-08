#!/usr/bin/env bash
set -euo pipefail

# fixture anonymization guard.
#
# This lint is intentionally scoped to checked-in evaluation fixtures. Public
# vendor or competitor references may exist in production code, but fixture data
# must stay synthetic and anonymized.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [[ -n "${DOS216_FIXTURE_LINT_ROOT_OVERRIDE:-}" ]]; then
  ROOT_DIR="$DOS216_FIXTURE_LINT_ROOT_OVERRIDE"
elif [[ -d "src-tauri/tests/fixtures" ]]; then
  ROOT_DIR="$(pwd)"
elif [[ "$(basename "$(pwd)")" == "src-tauri" && -d "tests/fixtures" ]]; then
  ROOT_DIR="$(cd .. && pwd)"
else
  ROOT_DIR="$DEFAULT_REPO_ROOT"
fi

FIXTURE_ROOT="${DOS216_FIXTURE_LINT_FIXTURE_ROOT_OVERRIDE:-$ROOT_DIR/src-tauri/tests/fixtures}"

if [[ ! -d "$FIXTURE_ROOT" ]]; then
  echo "Fixture root not found: $FIXTURE_ROOT"
  exit 1
fi

violations=""

while IFS= read -r -d '' file; do
  rel_path="${file#$ROOT_DIR/}"

  if [[ "$(basename "$file")" == "fixture_identity_map.json" ]]; then
    violations+="${rel_path}:1:identity-map-file"$'\n'
    continue
  fi

  file_violations="$(
    REL_PATH="$rel_path" perl -ne '
      BEGIN {
        $allow = qr/LINT-ALLOW:\s*fixture-anonymization\s*\(justification:\s*[^)]+\)/;
        $email = qr/\b([A-Za-z0-9._%+\-]+)@([A-Za-z0-9.\-]+\.[A-Za-z]+)\b/;
        @checks = (
          ["phone-like-number", qr/(?<![A-Za-z0-9])(?:\+?1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4})(?![A-Za-z0-9])/],
          ["phone-like-number", qr/(?<![A-Za-z0-9])(?:\+?1[-.\s]?)?\d{3}[-.\s]\d{4}(?![A-Za-z0-9])/],
          ["known-customer-or-vendor:REDACTED", qr/\bsalesforce(?:\.com)?\b/i],
          ["known-customer-or-vendor:hubspot", qr/\bhubspot(?:\.com)?\b/i],
          ["known-customer-or-vendor:monday", qr/\bmonday(?:\.com)?\b/i],
          ["known-customer-or-vendor:notion", qr/\bnotion(?:\.so|\.com)?\b/i],
          ["known-customer-or-vendor:stripe", qr/\bstripe(?:\.com)?\b/i],
          ["known-customer-or-vendor:shopify", qr/\bshopify(?:\.com)?\b/i],
          ["known-customer-or-vendor:jane.app", qr/\bjane\.app\b/i],
          ["known-customer-or-vendor:jane-software", qr/\bJane\s+Software\b/i],
          ["known-customer-or-vendor:automattic", qr/\bautomattic\b/i],
          ["known-customer-or-vendor:wp-vip", qr/\bwp[-_ ]?vip\b/i],
          ["known-customer-or-vendor:REDACTED", qr/\bmerck\b/i],
          ["known-customer-or-vendor:REDACTED", qr/\bmsd\b/i],
          ["redacted-literal", qr/\bREDACTED\b/],
        );
      }

      next if /$allow/;

      while (/$email/g) {
        my $domain = lc $2;
        if ($domain ne "example.com" && $domain !~ /\.example\.com$/) {
          print "$ENV{REL_PATH}:$.:non-example-email\n";
          last;
        }
      }

      for my $check (@checks) {
        my ($label, $regex) = @$check;
        if (/$regex/) {
          print "$ENV{REL_PATH}:$.:$label\n";
        }
      }
    ' "$file"
  )"

  if [[ -n "$file_violations" ]]; then
    violations+="$file_violations"$'\n'
  fi
done < <(find "$FIXTURE_ROOT" -type f -print0 | sort -z)

if [[ -n "$violations" ]]; then
  echo "Fixture anonymization violations found."
  echo
  echo "Allowed per-line escape hatch:"
  echo "  LINT-ALLOW: fixture-anonymization (justification: ...)"
  echo
  echo "Violations:"
  printf "%s" "$violations"
  exit 1
fi

echo "No fixture anonymization violations found under ${FIXTURE_ROOT#$ROOT_DIR/}."

#!/usr/bin/env bash
# Configure branch protection on `public` repo's `dev` and `trunk`.
#
# Strategy: GET existing protection → merge our managed fields → PUT.
# Per-field endpoints don't exist for `required_linear_history`,
# `allow_force_pushes`, or `allow_deletions` — only the top-level PUT
# accepts them. So we do read-merge-write and preserve any fields we
# don't manage.
#
# Idempotent. `--dry-run` previews without applying. Requires `gh` CLI
# authenticated with admin scope on the public repo.
#
# Required check we add: `L2 / l2-summary` as an APP-BOUND check
# (bound to GitHub Actions app_id) so a third-party integration can't
# satisfy the gate by posting a same-named status.

set -euo pipefail

DRY_RUN=false
if [ "${1:-}" = "--dry-run" ]; then
  DRY_RUN=true
fi

REPO="${REPO:-jamesgiroux/daily-operating-system}"
BRANCHES=("dev" "trunk")
MANAGED_CHECK="L2 / l2-summary"

# GitHub Actions app_id is well-known: 15368
# (verifiable via `gh api /apps/github-actions --jq .id`)
GITHUB_ACTIONS_APP_ID=15368

merge_and_put() {
  local branch="$1"
  local current="$2"

  # Use Python for robust JSON merge. The current protection JSON has its own
  # shape (GET-response style); the PUT body needs flat boolean fields and
  # the expected nested shape for required_status_checks.
  local desired_body
  desired_body=$(MANAGED_CHECK="$MANAGED_CHECK" \
                 GITHUB_ACTIONS_APP_ID="$GITHUB_ACTIONS_APP_ID" \
                 CURRENT_PROTECTION_JSON="$current" \
                 python3 - <<'PY'
import json
import os
import sys

managed_check = os.environ["MANAGED_CHECK"]
gh_actions_app_id = int(os.environ["GITHUB_ACTIONS_APP_ID"])
current_raw = os.environ.get("CURRENT_PROTECTION_JSON", "").strip() or "{}"

try:
    current = json.loads(current_raw)
except json.JSONDecodeError:
    current = {}

# Helper: GET response wraps some flags as {"enabled": <bool>}; PUT wants <bool>.
def flat(obj, key, default=None):
    val = obj.get(key) if isinstance(obj, dict) else None
    if val is None:
        return default
    if isinstance(val, dict) and set(val.keys()) == {"enabled"}:
        return val["enabled"]
    return val

# Build required_status_checks. Preserve existing checks; add ours as app-bound.
existing_rsc = current.get("required_status_checks") or {}
existing_checks = list(existing_rsc.get("checks") or [])
existing_contexts = list(existing_rsc.get("contexts") or [])

# Migrate any prior unbound `contexts` to the `checks` list (existing ones may
# have been added before this script existed). Don't drop them.
existing_check_contexts = {c["context"] for c in existing_checks if isinstance(c, dict) and "context" in c}
for ctx in existing_contexts:
    if ctx not in existing_check_contexts:
        existing_checks.append({"context": ctx})  # Unbound migration

# Ensure our managed check is present and app-bound.
have_managed = False
for chk in existing_checks:
    if isinstance(chk, dict) and chk.get("context") == managed_check:
        chk["app_id"] = gh_actions_app_id
        have_managed = True
if not have_managed:
    existing_checks.append({"context": managed_check, "app_id": gh_actions_app_id})

# Normalize required_pull_request_reviews from GET-shape (user/team/app objects
# with URLs, slugs, etc.) to PUT-shape (string arrays of logins/slugs).
def normalize_review_dismissal(rpr):
    if not rpr:
        return None
    out = dict(rpr)
    dr = out.get("dismissal_restrictions") or {}
    if dr:
        out["dismissal_restrictions"] = {
            "users": [u["login"] for u in dr.get("users", []) if isinstance(u, dict) and "login" in u]
                     or list(dr.get("users", [])),
            "teams": [t["slug"] for t in dr.get("teams", []) if isinstance(t, dict) and "slug" in t]
                     or list(dr.get("teams", [])),
            "apps":  [a["slug"] for a in dr.get("apps", [])  if isinstance(a, dict) and "slug" in a]
                     or list(dr.get("apps", [])),
        }
    bcr = out.get("bypass_pull_request_allowances") or {}
    if bcr:
        out["bypass_pull_request_allowances"] = {
            "users": [u["login"] for u in bcr.get("users", []) if isinstance(u, dict) and "login" in u]
                     or list(bcr.get("users", [])),
            "teams": [t["slug"] for t in bcr.get("teams", []) if isinstance(t, dict) and "slug" in t]
                     or list(bcr.get("teams", [])),
            "apps":  [a["slug"] for a in bcr.get("apps", [])  if isinstance(a, dict) and "slug" in a]
                     or list(bcr.get("apps", [])),
        }
    return out

# Same shape for restrictions (top-level).
def normalize_restrictions(restr):
    if not restr:
        return None
    return {
        "users": [u["login"] for u in restr.get("users", []) if isinstance(u, dict) and "login" in u]
                 or list(restr.get("users", [])),
        "teams": [t["slug"] for t in restr.get("teams", []) if isinstance(t, dict) and "slug" in t]
                 or list(restr.get("teams", [])),
        "apps":  [a["slug"] for a in restr.get("apps", [])  if isinstance(a, dict) and "slug" in a]
                 or list(restr.get("apps", [])),
    }

put_body = {
    "required_status_checks": {
        "strict": True,
        "checks": existing_checks,
        # Newer API uses `checks`; `contexts` is deprecated but kept for
        # back-compat. Provide an empty contexts list to avoid 422.
        "contexts": [],
    },
    "enforce_admins": flat(current, "enforce_admins", default=False),
    "required_pull_request_reviews": normalize_review_dismissal(current.get("required_pull_request_reviews")),
    "restrictions": normalize_restrictions(current.get("restrictions")),
    "required_linear_history": True,  # forced below; this is the script's contract
    "allow_force_pushes": flat(current, "allow_force_pushes", default=False),
    "allow_deletions": flat(current, "allow_deletions", default=False),
    "block_creations": flat(current, "block_creations", default=False),
    "required_conversation_resolution": flat(current, "required_conversation_resolution", default=False),
    "lock_branch": flat(current, "lock_branch", default=False),
    "allow_fork_syncing": flat(current, "allow_fork_syncing", default=False),
}

# Force our managed booleans regardless of prior state (this script's contract).
put_body["required_linear_history"] = True
put_body["allow_force_pushes"] = False
put_body["allow_deletions"] = False

print(json.dumps(put_body, indent=2))
PY
)

  if [ "$DRY_RUN" = true ]; then
    echo "[dry-run] would PUT to repos/${REPO}/branches/${branch}/protection:"
    echo "$desired_body" | sed 's/^/    /'
    return
  fi

  echo "$desired_body" | gh api -X PUT \
    "repos/${REPO}/branches/${branch}/protection" \
    -H "Accept: application/vnd.github+json" \
    --input - >/dev/null
}

for branch in "${BRANCHES[@]}"; do
  echo "── ${REPO}@${branch} ──"

  # Distinguish 404 (no protection) from other GET errors. Only swallow 404.
  if current=$(gh api "repos/${REPO}/branches/${branch}/protection" 2>&1); then
    : # 200; current has the JSON
  else
    if echo "$current" | grep -q "Not Found"; then
      current="{}"
    else
      echo "::error::GET protection failed (non-404):"
      echo "$current" | sed 's/^/    /'
      exit 1
    fi
  fi
  if [ "$current" = "{}" ]; then
    echo "  (no existing protection — creating fresh)"
  else
    existing_count=$(echo "$current" \
      | python3 -c 'import json, sys; d=json.load(sys.stdin); rsc=d.get("required_status_checks") or {}; print(len(rsc.get("checks",[])) + len(rsc.get("contexts",[])))')
    echo "  ($existing_count existing required check(s); preserving)"
  fi

  merge_and_put "$branch" "$current"
  if [ "$DRY_RUN" = false ]; then
    echo "  ✓ ${branch}: merged 'L2 / l2-summary' (app-bound to GitHub Actions); preserved other settings"
  fi
  echo ""
done

cat <<EOF
Done.

Notes:
  - Read-merge-write: existing protection settings (other required checks,
    required PR reviews, CODEOWNERS restrictions, etc.) are preserved.
    Only the L2 / l2-summary check (added as app-bound), strict, linear-history,
    no-force-push, and no-deletion fields are explicitly managed.
  - The L2 / l2-summary check is app-bound to GitHub Actions (app_id=15368)
    so a third-party status integration can't satisfy the gate by posting a
    status with the same name.
  - Re-runnable safely.
  - 'enforce_admins' is preserved at whatever the existing protection has.
    Flip via UI when Phase 5/6 ship.
  - Preview-only: bash .github/scripts/configure-branch-protection.sh --dry-run

Admin override policy:
  - 'enforce_admins: false' = James can override via "merge without waiting"
    in true emergencies (logged by GitHub).
  - Any admin override requires a follow-up PR within 24h that re-runs the
    L2 panel against the merged commit.
  - Two legitimate flows currently route through admin override:
      1. PRs that modify L2 gate config (per check-config-fence.sh)
      2. Dependabot SHA-bump PRs (until per-bot fence carve-out is verified)
EOF

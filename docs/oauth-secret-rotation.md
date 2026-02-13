# OAuth Secret Rotation and History Rewrite

Use this runbook when a Google OAuth secret has been exposed in git history.

## Prerequisites

- Admin access to GitHub repository settings
- Admin access to Google Cloud OAuth credentials
- Permission to force-push branches and tags
- `git-filter-repo` installed locally

## 1. Rotate credentials

1. Create a new OAuth Desktop client in Google Cloud Console.
2. Set repository secret `DAILYOS_GOOGLE_SECRET` to the new client secret.
3. Validate release build and OAuth login using the new secret.
4. Revoke/delete the old OAuth client.

## 2. Rewrite history in a mirror clone

```bash
git clone --mirror git@github.com:jamesgiroux/daily-operating-system.git daily-operating-system-mirror.git
cd daily-operating-system-mirror.git
```

Create a replace-text file (example: `/tmp/replace-secrets.txt`):

```text
literal:GOCSPX-OLD-EXPOSED-SECRET==>REDACTED
```

Run rewrite:

```bash
git filter-repo --replace-text /tmp/replace-secrets.txt
```

Force-push rewritten refs:

```bash
git push --force --all
git push --force --tags
```

## 3. Developer recovery steps

After force-push, every contributor must refresh local clones:

```bash
git fetch --all --prune
git reset --hard origin/main
```

If local history diverged heavily, reclone:

```bash
mv daily-operating-system daily-operating-system.old
git clone git@github.com:jamesgiroux/daily-operating-system.git
```

## 4. Verify cleanup

```bash
rg -n "GOCSPX-" .
```

Confirm:

- No matches in source history for revoked secrets
- GitHub secret scanning alerts are resolved
- OAuth works in release artifacts using the new secret

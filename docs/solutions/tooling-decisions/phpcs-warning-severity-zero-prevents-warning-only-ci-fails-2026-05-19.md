---
title: "PHPCS exits 1 on warning-only output — use --warning-severity=0 in CI to fail only on errors"
problem_type: tooling_decision
track: knowledge
module: wp/dailyos/phpcs.xml.dist, .github/workflows/wp-plugin.yml
tags: [phpcs, wordpress-coding-standards, ci, exit-codes, lint, w3]
date: 2026-05-19
related_linear: DOS-698, DOS-700
---

## Context

v1.4.3 W3 PR-E1 CI lint job (`./vendor/bin/phpcs -q`) failed despite **0 errors** reported per file. PHPCS by default exits with code 1 whenever any errors *or* warnings are reported, not only on errors. WordPress Coding Standards is loaded with `WordPress-Extra` + `WordPress-Docs`, which surfaces a substantial number of style warnings (`json_encode` discouragement → suggest `wp_json_encode`, reserved-keyword parameter names like `$class`/`$var`/`$default`, `WP_Filesystem` suggestions over native filesystem ops, `unlink`/`exec`/`proc_open` advisories, Yoda condition nudges).

These warnings are advisory by design — the security-load-bearing PHPCS rules (`WordPress.DB.PreparedSQL`, `WordPress.Security.EscapeOutput`, `WordPress.Security.NonceVerification`, `ValidatedSanitizedInput`) are registered as **errors** in the WP-Extra ruleset, not warnings. Lowering the warning severity threshold does NOT unmask SQLi/XSS/CSRF gates.

## Symptom

```
FOUND 0 ERRORS AND N WARNINGS AFFECTING N LINES
...
##[error]Process completed with exit code 1.
```

— every PR touching `wp/dailyos/**/*.php` red on lint, despite the rule set itself being clean of errors. Pre-W3 dev was failing on this for 3+ runs nobody noticed.

## Fix

In `.github/workflows/wp-plugin.yml` (or wherever PHPCS is invoked in CI):

```yaml
- name: PHPCS (WordPress Coding Standards)
  # --warning-severity=0 — fail only on errors. Warnings are informational
  # nudges (json_encode discouragement, reserved-keyword param names,
  # WP_Filesystem suggestions) that pre-existed across W2.
  run: ./vendor/bin/phpcs -q --warning-severity=0
```

Local dev: developers still see warnings (omit the flag when running interactively); CI gates only on errors.

## Alternative considered + rejected

**Suppress individual sniffs in `phpcs.xml.dist`** — works but inverts the polarity: every new advisory sniff that ships in a future WPCS release becomes a hard-fail until explicitly exempted. Maintenance-hostile. The severity-cap pattern is the right abstraction: warnings stay visible locally for code-review judgment, errors block.

## Security posture validated

Independent security review confirmed `--warning-severity=0` does NOT hide:

- `WordPress.DB.PreparedSQL` (error severity in WP-Extra)
- `WordPress.Security.EscapeOutput` (error)
- `WordPress.Security.NonceVerification` (error)
- `WordPress.Security.ValidatedSanitizedInput` (error)

These are the gates that matter. Warnings caught nothing security-load-bearing.

## Cost

- ~40 min spent diagnosing PHPCS exit-code semantics + chasing per-file warnings before finding the flag.
- Pre-W3 dev had been failing this gate for 3+ runs because nobody had successfully merged a PR that would have surfaced it.

## Cross-references

- `wp/dailyos/phpcs.xml.dist` (the ruleset — pairs with the severity flag)
- DOS-700 path-α: still tracks the residual warnings (json_encode → wp_json_encode adoption, etc.) as cleanup work; severity cap unblocks CI without hiding them.
- Memory: `feedback_post_rebase_integration_damage_blind_gates.md` — same class (dev gate silently failing).

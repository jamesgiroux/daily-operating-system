# Design System Version

**Current:** `0.0.0` — scaffolding only

**Stability statement:** Pre-stable. Expect breaking changes until 1.0.0. Pre-1.0, treat any minor bump as potentially breaking (standard semver-zero convention).

## Versioning rules

The design system is versioned independently of the app (`v1.4.x`). It tracks the **contract** of the four tiers:

- Token names and what they mean
- Primitive APIs and variant sets
- Pattern composition contracts and variant sets
- Surface specs (less binding, but renames are noted)

| Bump | Triggers | Examples |
|---|---|---|
| **Patch** `0.1.0 → 0.1.1` | Doc clarifications, internal refactors of reference renders, additive variants, no consumer behavior changes | Add a new variant to an existing primitive; rewrite a spec for clarity |
| **Minor** `0.1.0 → 0.2.0` | New primitive, new pattern, new surface spec, additive token, additive behavior | Promote `TrustBand` pattern; add `--color-trust-needs-verification` token |
| **Major** `0.x → 1.0`, `1.0 → 2.0` | Renamed/removed token, removed variant, changed pattern API, removed entry, restructured directory | Rename `--color-account-turmeric` to `--color-account-primary`; drop the `kind="ghost"` variant from `Button` |

## When to bump

Any PR that modifies `.docs/design/` files (other than typo fixes) MUST:

1. Update `VERSION.md` to the new version
2. Add a `CHANGELOG.md` entry for the bump
3. Tag the commit `design-system-v<version>` after merge to `dev`

## Tag namespace

Design system tags use the `design-system-v*` prefix to keep them distinct from app version tags (`v1.4.x`). They live in the same git repo but in their own namespace.

## Roadmap to 1.0.0

`1.0.0` is reached when:

- All four tiers (tokens, primitives, patterns, surfaces) have at least one canonical entry each
- The reference renders (`reference/tokens.html`, `primitives.html`, `patterns.html`) are populated and accurate
- The naming reconciliation track has shipped (no major rename candidates pending — or all known candidates have been intentionally deferred with notes)
- The four foundational audits have synthesized and their findings are reflected in canonical entries

Until then, expect movement.

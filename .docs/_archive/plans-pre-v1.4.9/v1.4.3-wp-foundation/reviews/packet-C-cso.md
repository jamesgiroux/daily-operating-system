# CSO review — L0 Packet C (W1 C1 Starter Kit)

**Verdict: CONDITIONAL APPROVE (advisory only — no new trust boundaries; one factual amendment forced; four sleeper concerns flagged with mitigations)**

Reviewer: CSO mode (Claude)
Date: 2026-05-18
Packet: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md` V1.0
Branch: `docs/v143-l0-packets`
Linear: DOS-678
Source SHAs read (working tree):
- `wp/dailyos/includes/class-dailyos-plugin.php:72, 149-163` (register_blocks — actual implementation is `glob()` directory scan, NOT explicit enumeration)
- `wp/dailyos/blocks/account-overview/` (full directory — confirmed shape match to packet templates)
- `src-tauri/abilities-runtime/src/abilities/` (registry surface — confirmed `#[ability(...)]` is a procedural macro, not arbitrary PHP)
- `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php` (fake-runtime-client test pattern — confirmed)
Threat-model anchor: v1.4.2 "Threat model: local-to-local" + Packet A cycle-2 closed (same-UID developer surface).

Packet §14 declares "CSO advisory only — no new trust boundaries." **I confirm that framing.** The kit is a code-generation toolchain executed at developer-UID against substrate primitives that already enforce their own boundaries (`#[ability(...)]` macro is the authorization fence; `commit_composition` is the mutation fence; `class-dailyos-runtime-client` is the transport fence). The kit does not author new claims/tables/transports/scopes/actors. None of the kit's generated artifacts grant runtime authority that the substrate hasn't already audited.

That said: a code-generation kit IS a software supply-chain surface. Five concerns warrant text amendments before implementation. None blocks the packet.

## Summary table

| # | Concern | Severity | Disposition |
|---|---|---|---|
| 1 | §6.6 + §4 + §12 Q3 cite non-existent `register_blocks_from_metadata` enumeration; actual code is `glob()` directory scan | **MEDIUM (factual)** | Amendment required: replace §6.6 direction with directory-scan reality; CLI no longer needs to edit `class-dailyos-plugin.php`; §12 Q3 dissolves |
| 2 | Translator generates PHP from TSX — codegen injection vector | **LOW** | Add §9 CI invariant: generated `render.php` must pass `php -l` + a static-analysis pass; templates use only `esc_html`/`esc_attr`/`wp_kses_post` for any string interpolation |
| 3 | Token-to-theme.json generator partial-write hazard | **LOW** | Add §6 directional decision: atomic write via tmpfile+rename; on conflict, leave existing `theme.json` untouched and exit non-zero |
| 4 | Integration harness Rust→PHP shell-out — code-execution surface | **LOW** | Add §5.5 contract: PHP harness accepts ONLY (projection-JSON path, ability-name, expected-substring-list); no eval, no include of user-supplied paths; validate ability-name against registered allowlist |
| 5 | Render-functions.php template inheriting Packet B §5.6 typed-error switch verbatim | **LOW** | Add §9 CI invariant: bytewise grep gate already proposed (AC #11); also assert the switch's `default` arm preserves Packet B's safe-fallback (no leaking internal codes to HTML) |
| 6 | Block scaffold templates bypassing authorization | **N/A — confirmed safe** | All scaffolded paths consume `project_composition_for_surface`, which routes through `surface_runtime` authorization. No template offers a direct-DB primitive. |
| 7 | CLI runs as developer UID — same-UID escalation framing | **N/A — confirmed safe** | Developer UID owns the plugin file already; insertion is shape-equivalent to a manual edit. No privilege boundary crossed. |

## Concern 1 — §6.6 + §4 + §12 Q3 cite a non-existent explicit enumeration (FACTUAL)

**Severity: MEDIUM. Forces packet amendment, but in the safer direction — no CLI-driven mutation of plugin core.**

Packet §4 reuse audit and §6.6 directional decision both reference `class-dailyos-plugin.php:154-170` as a "block-metadata enumeration" the CLI must insert into, and §6.6 claims "W4-F precedent and reviewers (CSO+code-reviewer) at W4-F preferred explicit-allowlist over directory-scan." I read `class-dailyos-plugin.php`. The actual implementation at `:149-163` is:

```php
public function register_blocks(): void {
    if ( ! function_exists( 'register_block_type_from_metadata' ) ) {
        return;
    }
    $block_files = glob( DAILYOS_PLUGIN_DIR . 'blocks/*/block.json' );
    if ( false === $block_files ) { return; }
    foreach ( $block_files as $block_file ) {
        register_block_type_from_metadata( dirname( $block_file ) );
    }
}
```

That is a `glob()` directory scan. There is no explicit enumeration to insert into, and there is no W4-F decision establishing an allowlist precedent (I did not find one in `.docs/plans/v1.4.1-waves.md` or the W4-F PR description either). The packet either invented the precedent or recalled a hypothetical alternative as decided fact.

**Security implication: this is the better answer.** A CLI that doesn't modify a versioned PHP source file is strictly safer than one that does. No code-insertion vector exists if no code insertion happens. AC #3 ("CLI scaffold registers block without manual editing") is already satisfied by the existing directory-scan — the CLI just needs to drop `block.json` into `blocks/<name>/`.

**Required amendment (V1.1):**

> **§6.6 — rewrite:** "The CLI does NOT modify `class-dailyos-plugin.php`. Block registration is automatic via the existing `register_blocks()` glob over `wp/dailyos/blocks/*/block.json` at `class-dailyos-plugin.php:149-163`. The CLI's responsibility is limited to creating the block directory with valid `block.json` + companions. If at any point the substrate moves to explicit enumeration (e.g., for ordering control), this packet's CLI design must be revisited."

> **§4 reuse audit — fix row 7:** `WP block registration | wp/dailyos/includes/class-dailyos-plugin.php register_blocks() — auto-discovery via glob() | :149-163`.

> **§5.1 — drop responsibility #3:** CLI no longer updates `class-dailyos-plugin.php`. AC #3 reframed: "CLI-scaffolded block appears in WP block editor without any PHP file edits."

> **§12 Q3 — dissolves.** No insertion anchors needed.

This dissolves the largest source of automated-mutation risk in the packet AND makes the CLI simpler.

## Concern 2 — Translator generates PHP from TSX (LOW)

The TSX→PHP translator (§5.7) extracts JSX structure and emits `<div>`/`<span>` trees. A carefully-crafted TSX file could embed strings that, if not escaped at emission time, become a PHP injection vector. Threat model: developer-UID, so the practical impact is "developer can write PHP that does what developer-UID PHP can do" — not an escalation. **But:** the kit's whole pitch is "no one reads substrate source to author blocks"; a translator that emits unescaped strings teaches an unsafe default that future block authors will copy.

**Required amendments (V1.1):**

> **§5.7 — add explicit emission discipline:** All translator-emitted PHP MUST use `esc_html()` / `esc_attr()` / `wp_kses_post()` (per WordPress data-sanitization conventions) for ANY string interpolation from TSX source. The translator MUST NOT emit raw PHP string concatenation into HTML output. Static literals are fine; anything traced back to a TSX prop or expression is wrapped.

> **§9 — add CI invariant #7:** `php -l` lint on every generated `render-functions.php` + a grep gate asserting no `echo $` outside an `esc_*`/`wp_kses_*` wrapper inside `wp/dailyos/blocks/*/render*.php`.

## Concern 3 — Token-to-theme.json generator partial-write hazard (LOW)

`generate-theme-json.mjs` (§5.6) writes `wp/dailyos/theme/theme.json`. A malformed token source mid-merge could produce a half-written file that breaks WP theme loading entirely. Plugin doesn't start, blocks don't render, integration fixtures fail with confusing errors.

**Required amendment (V1.1):**

> **§5.6 — add atomicity contract:** Generator writes to `wp/dailyos/theme/theme.json.tmp` then `rename()` (atomic on POSIX). On any merge error or conflict, the generator leaves the existing `theme.json` untouched and exits non-zero with a diagnostic naming the conflict. CI's `--check` mode (§9 invariant #4) verifies output validity (JSON parses + required top-level keys present) BEFORE the rename.

## Concern 4 — Integration harness Rust→PHP shell-out (LOW)

§5.5 has Rust shell out to `StarterKitIntegrationTest.php`. The PHP harness must accept ONLY data (projection JSON, expected substrings, ability name) — never code, never a path to be `include`d, never an eval surface. Same-UID developer model means the practical impact is limited, but a harness that takes arbitrary paths is a class of latent bug (CI runs as a CI service account, not always the same as the developer; a malformed fixture could surface unexpected file reads).

**Required amendment (V1.1):**

> **§5.5 — add PHP-harness contract:** The PHP harness accepts exactly three argv inputs: (a) absolute path to a projection-JSON file (must live under repo root + `tests/blocks/fixtures/` — validated, rejected otherwise), (b) ability name (must match `^dailyos/[a-z][a-z0-9-]+$` and resolve to a registered block in `wp/dailyos/blocks/`), (c) expected-substring list (read from a sibling `.expected.json` file). No `eval`, no `include`/`require` of paths derived from argv, no `exec`/`shell_exec`/`passthru`. The harness terminates with exit code 1 on any input-validation failure BEFORE invoking the renderer.

## Concern 5 — Render-functions.php template inheriting Packet B §5.6 switch verbatim (LOW)

AC #11 already asserts grep-gate parity on the 5 switch arms. Add one more assertion: the `default` arm preserves Packet B's safe-fallback (returns a generic user-visible error, does NOT leak internal error codes to the rendered HTML). The fingerprinting concern raised in Packet B CSO concern 3 already concluded the codes are already in the wire format — but a future maintainer editing the template's default arm to "log the code for debugging" would regress that.

**Required amendment (V1.1):**

> **§9 invariant #2 — extend grep gate:** The grep gate covers the 5 switch arms PLUS the `default` arm's user-facing fallback message + the explicit absence of internal-code interpolation. Pseudo-pattern: assert the default branch echoes only literal strings (no `$code`, no `$detail`, no `$error->getCode()`).

## Concern 6 — Block scaffold authorization bypass (CONFIRMED SAFE)

I traced the scaffold's call shape. Every generated `render-functions.php` calls `project_composition_for_surface` via the signed runtime client. Authorization runs inside `surface_runtime` BEFORE the projection is returned. No template offers a path to `commit_composition`, direct DB access, or an unsigned client. New block authors cannot bypass authorization because the authorization fence is at the substrate boundary, not the template boundary. The template only sees post-authorization projections.

## Concern 7 — Same-UID CLI scaffold (CONFIRMED SAFE, post-Concern-1 amendment)

With Concern 1 amended (CLI no longer edits `class-dailyos-plugin.php`), the CLI's only mutation is creating files under `wp/dailyos/blocks/<name>/` and (optionally, with `--ability`) under `src-tauri/abilities-runtime/src/abilities/`. Developer-UID owns those directories; the CLI is shape-equivalent to `cp -r templates/simple blocks/foo`. No privilege boundary crossed.

If Concern 1 is rejected and the CLI does keep editing `class-dailyos-plugin.php`, the residual risk is "developer's CLI scaffold writes a non-malicious-but-buggy registration line into plugin core" — same-UID, same audit-via-git-diff, same blast radius as a manual edit. Still not an escalation, but the dissolved version is cleaner.

## Reviewer panel coordination

- **codex challenge:** should stress the translator's ability to emit safe PHP under deliberately adversarial TSX inputs (XSS-shaped prop strings, embedded `<?php` blocks in comments, prop names that collide with PHP superglobals, etc.).
- **codex consult:** should verify the `php -l` + grep gates proposed in Concern 2 are actually wireable in CI (CI containers have PHP available; the runtime client mock is sufficient for harness exec).
- **code-reviewer:** should verify the atomic-write pattern in §5.6 and the input-validation in §5.5 land as actual code, not just packet text.
- **DX review:** Concern 1's amendment makes the CLI simpler — should be a pure DX win.

## L0 closure recommendation

CONDITIONAL APPROVE: convergent on V1.1 with the five amendments above folded in. None requires a second cycle; all are text-level. If the author confirms Concern 1's factual correction lands AND the four LOW concerns get the proposed §-level amendments, CSO returns APPROVE on the next read.

**Confirming the §14 framing: NO NEW TRUST BOUNDARY is created by this packet.** The kit produces artifacts that consume existing boundaries (substrate ability macro, runtime authorization, signed transport, atomic mutation). The amendments above harden the kit's own software supply-chain hygiene; they do not introduce new boundaries either.

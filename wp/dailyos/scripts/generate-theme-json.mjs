#!/usr/bin/env node
// Token-to-theme.json generator (W1 Group 3 — DOS-678 §5.6).
// Reads canonical CSS custom properties from src/styles/design-tokens.css,
// resolves --foo: var(--bar) alias chains to terminal values, and emits
// wp/dailyos/theme/theme.json. The kit's generated theme.json is the seed
// for the W3 magazine theme.

import { readFileSync, writeFileSync, existsSync, mkdirSync, renameSync, unlinkSync } from 'node:fs';
import { dirname, resolve as resolvePath } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolvePath(SCRIPT_DIR, '..', '..', '..');
const TOKENS_CSS = resolvePath(REPO_ROOT, 'src', 'styles', 'design-tokens.css');
const OUT_DIR = resolvePath(REPO_ROOT, 'wp', 'dailyos', 'theme');
const OUT_FILE = resolvePath(OUT_DIR, 'theme.json');
const OUT_TMP = `${OUT_FILE}.tmp`;

const args = process.argv.slice(2);
const CHECK_MODE = args.includes('--check');

function fail(message, exitCode = 1) {
	process.stderr.write(`[generate-theme-json] ${message}\n`);
	process.exit(exitCode);
}

// Parse `--name: value;` declarations from the canonical CSS.
// Supports terminal values (hex, rgba, px, raw scalars) and var(--other) aliases.
function parseTokens(css) {
	const tokens = new Map(); // name → { rawValue, terminalOrAlias }
	const declRe = /--([a-z0-9-]+)\s*:\s*([^;]+);/gi;
	let m;
	while ((m = declRe.exec(css)) !== null) {
		const name = m[1];
		const rawValue = m[2].trim();
		if (tokens.has(name)) {
			const prior = tokens.get(name).rawValue;
			if (prior !== rawValue) {
				fail(
					`token --${name} redeclared with conflicting value (was: "${prior}", now: "${rawValue}"). ` +
					`Same-name-different-terminal-value conflicts cannot be reconciled by alias resolution.`
				);
			}
			continue;
		}
		tokens.set(name, { rawValue });
	}
	return tokens;
}

// Resolve --foo → var(--bar) → terminal value via DFS with cycle detection.
function resolveToken(name, tokens, seen = new Set()) {
	if (seen.has(name)) {
		fail(`alias cycle detected involving --${name} (chain: ${[...seen, name].join(' → ')})`);
	}
	seen.add(name);
	const entry = tokens.get(name);
	if (!entry) return null; // dangling alias
	const value = entry.rawValue;
	const varRe = /^var\(\s*--([a-z0-9-]+)\s*(?:,\s*([^)]+))?\)$/i;
	const aliasMatch = value.match(varRe);
	if (aliasMatch) {
		const target = aliasMatch[1];
		const resolved = resolveToken(target, tokens, seen);
		if (resolved !== null) return resolved;
		// dangling target; honor fallback if present.
		return aliasMatch[2] ? aliasMatch[2].trim() : null;
	}
	return value;
}

// Categorize tokens into theme.json sections.
// Heuristics by prefix; unknown prefixes route to settings.custom.dailyos.tokens.
function categorize(tokens) {
	const palette = []; // settings.color.palette
	const spacing = []; // settings.spacing.spacingSizes
	const custom = {}; // settings.custom.dailyos.tokens (everything else)
	const skipped = []; // ghost tokens that resolved to null

	for (const [name, entry] of tokens) {
		const terminal = resolveToken(name, tokens);
		if (terminal === null) {
			skipped.push({ name, reason: 'dangling alias or missing target' });
			continue;
		}
		if (name.startsWith('color-')) {
			// theme.json palette expects { slug, name, color: <hex|css-color> }.
			// Slug = full token name without the "color-" prefix; user-facing name is
			// title-cased from the slug for editor display.
			const slug = name.slice('color-'.length);
			palette.push({
				slug,
				name: humanize(slug),
				color: terminal,
			});
		} else if (name.startsWith('space-')) {
			const slug = name.slice('space-'.length);
			spacing.push({
				slug,
				name: humanize(slug),
				size: terminal,
			});
		} else {
			custom[name] = terminal;
		}
	}

	// Deterministic ordering for stable diffs.
	palette.sort((a, b) => a.slug.localeCompare(b.slug));
	spacing.sort((a, b) => a.slug.localeCompare(b.slug));
	const customSorted = Object.fromEntries(
		Object.entries(custom).sort(([a], [b]) => a.localeCompare(b))
	);

	return { palette, spacing, custom: customSorted, skipped };
}

function humanize(slug) {
	return slug
		.split('-')
		.map((s) => (s.length ? s[0].toUpperCase() + s.slice(1) : s))
		.join(' ');
}

// Build theme.json document. WordPress block theme schema v3.
function buildThemeJson({ palette, spacing, custom }) {
	return {
		$schema: 'https://schemas.wp.org/trunk/theme.json',
		version: 3,
		settings: {
			color: {
				palette,
				custom: true,
				customDuotone: false,
				customGradient: false,
				defaultPalette: false,
				link: true,
			},
			spacing: {
				spacingSizes: spacing,
				units: ['px', 'rem', 'em', '%'],
			},
			custom: {
				dailyos: {
					generator: 'wp/dailyos/scripts/generate-theme-json.mjs',
					source: 'src/styles/design-tokens.css',
					tokens: custom,
				},
			},
		},
	};
}

function canonicalize(json) {
	// Stable JSON with 2-space indent + trailing newline; identical to `pnpm` formatting.
	return JSON.stringify(json, null, '\t') + '\n';
}

function main() {
	if (!existsSync(TOKENS_CSS)) {
		fail(`canonical CSS source missing: ${TOKENS_CSS}`);
	}
	const css = readFileSync(TOKENS_CSS, 'utf8');
	const tokens = parseTokens(css);
	if (tokens.size === 0) {
		fail(`no token declarations parsed from ${TOKENS_CSS}`);
	}
	const categorized = categorize(tokens);
	const themeJson = buildThemeJson(categorized);
	const serialized = canonicalize(themeJson);

	if (CHECK_MODE) {
		if (!existsSync(OUT_FILE)) {
			fail(`--check: ${OUT_FILE} does not exist; run without --check to generate.`, 2);
		}
		const existing = readFileSync(OUT_FILE, 'utf8');
		if (existing !== serialized) {
			fail(`--check: generated theme.json differs from existing. Run \`pnpm dailyos:generate-theme-json\` to update.`, 2);
		}
		process.stdout.write(
			`[generate-theme-json] --check OK (${categorized.palette.length} palette / ${categorized.spacing.length} spacing / ${Object.keys(categorized.custom).length} custom)\n`
		);
		return;
	}

	if (!existsSync(OUT_DIR)) mkdirSync(OUT_DIR, { recursive: true });
	writeFileSync(OUT_TMP, serialized);
	// Validate that the tmpfile parses back to JSON; abort the rename on failure.
	try {
		JSON.parse(readFileSync(OUT_TMP, 'utf8'));
	} catch (err) {
		try { unlinkSync(OUT_TMP); } catch { /* noop */ }
		fail(`generated theme.json failed JSON re-parse: ${err.message}`);
	}
	renameSync(OUT_TMP, OUT_FILE);

	process.stdout.write(
		`[generate-theme-json] wrote ${OUT_FILE} ` +
		`(${categorized.palette.length} palette / ${categorized.spacing.length} spacing / ${Object.keys(categorized.custom).length} custom`
	);
	if (categorized.skipped.length > 0) {
		process.stdout.write(`; skipped ${categorized.skipped.length} dangling`);
	}
	process.stdout.write(`)\n`);
}

try {
	main();
} catch (err) {
	try { if (existsSync(OUT_TMP)) unlinkSync(OUT_TMP); } catch { /* noop */ }
	fail(err.stack || err.message || String(err));
}

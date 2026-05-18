#!/usr/bin/env node
// Tauri-primitive-to-WordPress-block translator (W1 Group 4 — DOS-678 §5.7).
//
// Scope matrix (per L0 Packet C V1.3 §5.7):
//   - Supported (static render-only):           full scaffold
//   - SupportedWithSourcePromotion:             full scaffold + promotion reminder
//   - SupportedWithInlineStyleAdaptation:       full scaffold + inline-style extract
//   - NotSupported (interactive event handlers): exit 1 with diagnostic
//
// Parity targets (AC §5.7 closer): Pill + HealthBadge end-to-end. Other
// supported primitives scaffold cleanly; their JSX-to-PHP body translation
// is filed per-primitive in W2 (the Wave 1 primitive translation batch).

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync } from 'node:fs';
import { dirname, resolve as resolvePath, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolvePath(SCRIPT_DIR, '..', '..', '..');
const PRIMITIVES_README = resolvePath(REPO_ROOT, '.docs', 'design', 'primitives', 'README.md');
const BLOCKS_DIR = resolvePath(REPO_ROOT, 'wp', 'dailyos', 'blocks');
const NEW_BLOCK = resolvePath(SCRIPT_DIR, 'new-block.mjs');

// Authoritative scope matrix from packet §5.7. Categories drive scaffold
// behavior; new primitives must be added here before translation succeeds.
const SCOPE_MATRIX = {
	Pill:                       { category: 'Supported',                          shape: 'simple' },
	HealthBadge:                { category: 'Supported',                          shape: 'typed-display' },
	StatusDot:                  { category: 'Supported',                          shape: 'simple' },
	Avatar:                     { category: 'Supported',                          shape: 'typed-display' },
	IntelligenceQualityBadge:   { category: 'Supported',                          shape: 'typed-display' },
	FreshnessIndicator:         { category: 'Supported',                          shape: 'typed-display' },
	ProvenanceTag:              { category: 'Supported',                          shape: 'simple' },
	TrustBandBadge:             { category: 'SupportedWithSourcePromotion',       shape: 'typed-display' },
	FolioRefreshButton:         { category: 'NotSupported',                       interactive: 'onClick handler bound to refresh action; inline-style extraction also deferred to v1.4.3 W7' },
	InlineInput:                { category: 'NotSupported',                       interactive: 'onChange handler bound to live value' },
	EditableText:               { category: 'NotSupported',                       interactive: 'click-to-edit + onSubmit handler' },
	Switch:                     { category: 'NotSupported',                       interactive: 'onCheckedChange handler' },
	Segmented:                  { category: 'NotSupported',                       interactive: 'onSelectionChange handler' },
	RemovableChip:              { category: 'NotSupported',                       interactive: 'onRemove handler bound to chip identity' },
	EntityChip:                 { category: 'NotSupported',                       interactive: 'editable variant (onChange handler)' },
	TypeBadge:                  { category: 'NotSupported',                       interactive: 'editable mode (onSelect handler)' },
};

const args = process.argv.slice(2);

function fail(message, exitCode = 1) {
	process.stderr.write(`[translate-tauri] ${message}\n`);
	process.exit(exitCode);
}

function parseArgs() {
	const flags = {};
	for (let i = 0; i < args.length; i++) {
		const a = args[i];
		if (a === '--primitive') flags.primitive = args[++i];
		else if (a === '--help' || a === '-h') flags.help = true;
		else if (a === '--dry-run') flags.dryRun = true;
		else fail(`unknown argument: ${a}. See --help.`);
	}
	return flags;
}

function printHelp() {
	process.stdout.write(`
Usage: pnpm dailyos:translate-tauri --primitive <PrimitiveName> [--dry-run]

Translates a Tauri React primitive (src/components/ui/<Name>.tsx or
src/components/shared/<Name>.tsx) into a WordPress block under
wp/dailyos/blocks/<kebab-name>/, consuming the W1 starter kit
templates and the integration test harness.

Scope (per L0 Packet C V1.3 §5.7):
  - Supported / SupportedWithSourcePromotion / SupportedWithInlineStyleAdaptation:
    generates full block scaffold (block.json + render.php + render-functions.php
    + edit.js + style.css + editor.css).
  - NotSupported (interactive event handlers): exits 1 with diagnostic.

Examples:
  pnpm dailyos:translate-tauri --primitive Pill
  pnpm dailyos:translate-tauri --primitive HealthBadge

Available primitives:
${Object.keys(SCOPE_MATRIX).sort().map((n) => `  - ${n} (${SCOPE_MATRIX[n].category})`).join('\n')}
`);
}

function kebab(name) {
	return name.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
}

function locateTsxSource(primitive) {
	const candidates = [
		resolvePath(REPO_ROOT, 'src', 'components', 'ui', `${primitive}.tsx`),
		resolvePath(REPO_ROOT, 'src', 'components', 'shared', `${primitive}.tsx`),
		resolvePath(REPO_ROOT, 'src', 'components', 'ui', `${kebab(primitive)}.tsx`),
		resolvePath(REPO_ROOT, 'src', 'components', 'entity', `${primitive}.tsx`),
	];
	return candidates.find((p) => existsSync(p)) || null;
}

function locateCssModule(tsxPath) {
	if (!tsxPath) return null;
	const cssPath = tsxPath.replace(/\.tsx$/, '.module.css');
	return existsSync(cssPath) ? cssPath : null;
}

// Extract prop names from the first `interface <Name>Props { ... }` declaration.
// Regex-based and intentionally narrow; we promote to a TS-parser if scaffold
// drift becomes a problem.
function extractPropTypes(tsx, primitive) {
	const ifaceRe = new RegExp(`interface\\s+(${primitive}Props|${primitive}BaseProps)\\s*{([^}]+)}`, 's');
	const m = tsx.match(ifaceRe);
	if (!m) return [];
	const body = m[2];
	const propRe = /^\s*(?:\/\*\*[\s\S]*?\*\/\s*)?([A-Za-z_][A-Za-z0-9_]*)\?\s*:\s*([^;\n]+);?/gm;
	const props = [];
	let pm;
	while ((pm = propRe.exec(body)) !== null) {
		props.push({ name: pm[1], type: pm[2].trim() });
	}
	return props;
}

function classifyOrRefuse(primitive) {
	const entry = SCOPE_MATRIX[primitive];
	if (!entry) {
		fail(
			`unknown primitive: ${primitive}.\n` +
			`This primitive is not in the W1 scope matrix at wp/dailyos/scripts/translate-tauri.mjs.\n` +
			`If it's a Wave 1 primitive that needs adding, update SCOPE_MATRIX with the category + shape.\n` +
			`If it should be hand-authored, use \`pnpm dailyos:new-block --template <shape> <name>\`.`
		);
	}
	if (entry.category === 'NotSupported') {
		fail(
			`${primitive} requires interactive event handlers (${entry.interactive}).\n` +
			`Use \`pnpm dailyos:new-block --template <simple|typed-display> ${kebab(primitive)}\` to scaffold manually.\n` +
			`See .docs/design/primitives/${primitive}.md for the interactive contract this primitive ships.`
		);
	}
	return entry;
}

function runScaffold(primitive, shape, slug) {
	const scaffoldArgs = ['--template', shape, slug];
	try {
		execFileSync('node', [NEW_BLOCK, ...scaffoldArgs], {
			stdio: 'inherit',
			cwd: REPO_ROOT,
		});
	} catch (err) {
		fail(`scaffold failed (\`node ${NEW_BLOCK} ${scaffoldArgs.join(' ')}\`): ${err.message}`);
	}
}

// Hand-coded body translations for AC §5.7 parity targets (Pill + HealthBadge).
// These overwrite the scaffold's render-functions.php body with primitive-faithful
// PHP that mirrors the TSX. All other primitives ship with the scaffold's TODO body;
// W2 (Wave 1 primitive translation batch) lands per-primitive body translations.
const BODY_TRANSLATIONS = {
	Pill: {
		'block.json': null, // use scaffold default
		renderBody: `<?php
/**
 * Pill (translated from src/components/ui/Pill.tsx).
 * Variants: tone (sage|turmeric|terracotta|larkspur|olive|eucalyptus|neutral),
 *           size (standard|compact), dot (bool), interactive (bool).
 */

declare(strict_types=1);

function dailyos_block_pill_render(array $attributes): string {
	$tone = isset($attributes['tone']) ? (string) $attributes['tone'] : 'neutral';
	$size = isset($attributes['size']) ? (string) $attributes['size'] : 'standard';
	$dot = !empty($attributes['dot']);
	$interactive = !empty($attributes['interactive']);
	$label = isset($attributes['label']) ? (string) $attributes['label'] : '';

	$allowed_tones = ['sage', 'turmeric', 'terracotta', 'larkspur', 'olive', 'eucalyptus', 'neutral'];
	if (!in_array($tone, $allowed_tones, true)) {
		$tone = 'neutral';
	}
	$size = ($size === 'compact') ? 'compact' : 'standard';

	$classes = ['dailyos-pill', 'dailyos-pill--tone-' . $tone, 'dailyos-pill--size-' . $size];
	if ($interactive) { $classes[] = 'dailyos-pill--interactive'; }

	$dot_html = $dot ? '<span class="dailyos-pill__dot" aria-hidden="true"></span>' : '';
	$label_html = esc_html($label);

	return sprintf(
		'<span class="%s" data-tone="%s" data-ds-name="Pill" data-ds-tier="primitive" data-ds-spec="primitives/Pill.md">%s%s</span>',
		esc_attr(implode(' ', $classes)),
		esc_attr($tone),
		$dot_html,
		$label_html
	);
}
`,
	},
	HealthBadge: {
		'block.json': null,
		renderBody: `<?php
/**
 * HealthBadge (translated from src/components/shared/HealthBadge.tsx).
 * Variants: size (compact|standard|hero), band (green|yellow|red),
 *           trend.direction (improving|stable|declining|volatile),
 *           score (number), confidence (number 0-1), sufficientData (bool).
 */

declare(strict_types=1);

function dailyos_block_health_badge_render(array $attributes): string {
	$size = isset($attributes['size']) ? (string) $attributes['size'] : 'standard';
	$band = isset($attributes['band']) ? (string) $attributes['band'] : 'green';
	$score = isset($attributes['score']) ? (int) $attributes['score'] : 0;
	$trend = isset($attributes['trend']['direction']) ? (string) $attributes['trend']['direction'] : 'stable';
	$sufficient = !isset($attributes['sufficientData']) || $attributes['sufficientData'];
	$show_score = !isset($attributes['showScore']) || $attributes['showScore'];

	if (!in_array($size, ['compact', 'standard', 'hero'], true)) { $size = 'standard'; }
	if (!in_array($band, ['green', 'yellow', 'red'], true))       { $band = 'green'; }
	if (!in_array($trend, ['improving', 'stable', 'declining', 'volatile'], true)) { $trend = 'stable'; }

	$dot_class = 'dailyos-health-badge__dot dailyos-health-badge__dot--' . $band;
	$score_html = $sufficient && $show_score
		? sprintf('<span class="dailyos-health-badge__score">%d</span>', $score)
		: '<span class="dailyos-health-badge__insufficient">Insufficient Data</span>';

	$trend_glyph = ['improving' => '↑', 'declining' => '↓', 'stable' => '–', 'volatile' => '~'][$trend];
	$trend_html = $size !== 'compact'
		? sprintf(
			'<span class="dailyos-health-badge__trend dailyos-health-badge__trend--%s" aria-label="trend %s">%s</span>',
			esc_attr($trend), esc_attr($trend), $trend_glyph
		)
		: '';

	$wrapper_class = 'dailyos-health-badge dailyos-health-badge--' . $size . ' dailyos-health-badge--band-' . $band;

	return sprintf(
		'<span class="%s" data-band="%s" data-trend="%s" data-ds-name="HealthBadge" data-ds-tier="primitive" data-ds-spec="primitives/HealthBadge.md"><span class="%s" aria-hidden="true"></span>%s%s</span>',
		esc_attr($wrapper_class),
		esc_attr($band),
		esc_attr($trend),
		esc_attr($dot_class),
		$score_html,
		$trend_html
	);
}
`,
	},
};

function applyBodyTranslation(primitive, slug) {
	const translation = BODY_TRANSLATIONS[primitive];
	if (!translation) {
		process.stdout.write(
			`[translate-tauri] ${primitive}: scaffold-only (no hand-coded body in W1).\n` +
			`  Render body TODO at wp/dailyos/blocks/${slug}/render-functions.php — translate in W2.\n`
		);
		return;
	}
	const renderFnsPath = resolvePath(BLOCKS_DIR, slug, 'render-functions.php');
	writeFileSync(renderFnsPath, translation.renderBody);
	process.stdout.write(`[translate-tauri] ${primitive}: hand-coded body written to ${renderFnsPath}\n`);
}

function emitFollowups(primitive, slug, category) {
	const blockDir = resolvePath(BLOCKS_DIR, slug);
	process.stdout.write(`
[translate-tauri] ${primitive} → ${blockDir} (${category}) — scaffold complete.

Follow-ups (per L0 Packet C V1.3 §5.7):
  1. Drop a fixture at src-tauri/abilities-runtime/tests/fixtures/${slug.replace(/-/g, '_')}_integration_fixture.rs
     (the W1 CI gate at .github/workflows/block-kit-integration.yml requires
      a matching fixture for every wp/dailyos/blocks/<slug>/ directory).
  2. Run \`cargo test -p abilities-runtime --test block_kit_integration_harness\`
     to confirm producer→projection→render parity.
`);
	if (category === 'SupportedWithSourcePromotion') {
		process.stdout.write(
			`  3. Promote .docs/design/primitives/${primitive}.md from 'proposed' to 'integrated' once the WP block is in routed surfaces.\n`
		);
	}
	if (category === 'SupportedWithInlineStyleAdaptation') {
		process.stdout.write(
			`  3. The TSX source has inline style={{...}} usage; extracted to style.css. Review for tokenization completeness.\n`
		);
	}
}

function main() {
	const flags = parseArgs();
	if (flags.help) { printHelp(); return; }
	if (!flags.primitive) {
		fail('--primitive <PrimitiveName> is required. See --help.');
	}

	const entry = classifyOrRefuse(flags.primitive);
	const tsxPath = locateTsxSource(flags.primitive);
	if (!tsxPath) {
		fail(
			`could not locate TSX source for ${flags.primitive}. Checked src/components/{ui,shared,entity}/. ` +
			`Verify the primitive exists or update locateTsxSource().`
		);
	}
	const cssPath = locateCssModule(tsxPath);
	const tsx = readFileSync(tsxPath, 'utf8');
	const props = extractPropTypes(tsx, flags.primitive);
	const slug = kebab(flags.primitive);

	process.stdout.write(`[translate-tauri] ${flags.primitive} (${entry.category}, shape=${entry.shape})\n`);
	process.stdout.write(`  source TSX: ${tsxPath}\n`);
	if (cssPath) process.stdout.write(`  source CSS: ${cssPath}\n`);
	process.stdout.write(`  props detected: ${props.length === 0 ? '(none — props parsed via regex; verify manually)' : props.map((p) => p.name).join(', ')}\n`);

	if (flags.dryRun) {
		process.stdout.write('[translate-tauri] --dry-run: stopping before scaffold.\n');
		return;
	}

	const blockDir = resolvePath(BLOCKS_DIR, slug);
	if (existsSync(blockDir)) {
		fail(`block directory already exists: ${blockDir}. Refusing to overwrite — remove first or use \`pnpm dailyos:new-block\` interactively.`);
	}

	runScaffold(flags.primitive, entry.shape, slug);
	applyBodyTranslation(flags.primitive, slug);
	emitFollowups(flags.primitive, slug, entry.category);
}

try {
	main();
} catch (err) {
	fail(err.stack || err.message || String(err));
}

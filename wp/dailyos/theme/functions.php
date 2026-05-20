<?php
/**
 * DailyOS Magazine theme — chrome enqueue + per-surface chrome_config glue.
 *
 * @package dailyos-magazine
 */

namespace DailyOS\Theme;

const VERSION = '1.4.4';

/**
 * Asset enqueue chain: baseline tokens (plugin priority 9) → design tokens →
 * token aliases (Option C bridge) → chrome modules → fonts → chrome.js.
 *
 * Cascade contract: token-aliases.css MUST enqueue AFTER the WP-emitted
 * preset/custom stylesheet so `:root` last-declaration-wins lets aliases
 * resolve raw `--color-*` names to `--wp--preset--color--*`. Enforced via
 * `wp_enqueue_style` dependency graph below.
 *
 * AC #24 guard rationale: chrome.js DOM-injects FolioBar / FloatingNavIsland /
 * AtmosphereLayer over the page. Customizer renders its own preview chrome
 * (toolbar, breadcrumbs, controls) inside the preview iframe — injecting our
 * chrome on top produces stacked-chrome visual conflict. The guard prevents
 * that visual stacking, not a security concern.
 *
 * §5.5 bounded-behavior: this function MUST NOT write to the database, call
 * abilities-runtime / MCP / Rust runtime, branch on claim / trust / provenance
 * / signal / sensitivity, make HTTP calls to runtime, register block types,
 * REST routes, CPTs, or admin pages, or use top-level execution.
 */
function enqueue_chrome_assets(): void {
	if ( is_customize_preview() ) {
		return;
	}

	$base = get_stylesheet_directory_uri() . '/assets/chrome';

	// 1. Fonts — local @font-face declarations (eliminates FOUT).
	wp_enqueue_style( 'dailyos-fonts', $base . '/fonts.css', array(), VERSION );

	// 2. Design tokens — raw `--color-*`, `--folio-*`, `--space-*`, etc. as
	// fallback values for stock-theme contexts. Theme.json emits the same
	// values as `--wp--preset--*` / `--wp--custom--dailyos--tokens--*`.
	wp_enqueue_style(
		'dailyos-tokens',
		$base . '/styles/design-tokens.css',
		array( 'dailyos-fonts' ),
		VERSION
	);

	// 3. Token aliases — Option C bridge. Declares raw `--color-spice-turmeric`
	// etc. as `var(--wp--preset--color--spice-turmeric)` so chrome modules
	// resolve to theme.json values under WP cascade. Depends on tokens so
	// raw declarations are available as last-resort fallbacks.
	wp_enqueue_style(
		'dailyos-aliases',
		$base . '/styles/token-aliases.css',
		array( 'dailyos-tokens' ),
		VERSION
	);

	// 4. Chrome modules — each declares dependency on dailyos-aliases so the
	// bridge layer is guaranteed to load first.
	wp_enqueue_style( 'dailyos-magazine', $base . '/styles/MagazinePageLayout.module.css', array( 'dailyos-aliases' ), VERSION );
	wp_enqueue_style( 'dailyos-atmosphere', $base . '/styles/AtmosphereLayer.module.css', array( 'dailyos-aliases' ), VERSION );
	wp_enqueue_style( 'dailyos-folio', $base . '/styles/FolioBar.module.css', array( 'dailyos-aliases' ), VERSION );
	wp_enqueue_style( 'dailyos-nav', $base . '/styles/FloatingNavIsland.module.css', array( 'dailyos-aliases' ), VERSION );
	wp_enqueue_style( 'dailyos-pill', $base . '/styles/Pill.module.css', array( 'dailyos-aliases' ), VERSION );

	// 4b. WP-only chrome overlay — admin-bar offset for FolioBar. Drops the
	// folio bar below #wpadminbar (z-index 99999) instead of fighting on
	// z-index. Per L0 packet §10 WP-only overlay exception; not synced
	// from canonical.
	wp_enqueue_style( 'dailyos-chrome-wp-overlay', $base . '/wp-overlay-admin-bar.css', array( 'dailyos-folio' ), VERSION );

	// 4b-ii. DayStrip-aware pageContainer offset overlay. Briefing surfaces
	// add a fixed DayStrip below the FolioBar; the page container below
	// needs `+ 72px` margin-top to clear both bars. Mirrors the canonical
	// briefing-d-spine surface override; pattern-class scoped via the
	// `MagazinePageLayout_pageContainerWithDayStrip` modifier.
	wp_enqueue_style(
		'dailyos-chrome-wp-overlay-day-strip',
		$base . '/wp-overlay-day-strip.css',
		array( 'dailyos-magazine' ),
		VERSION
	);

	// 4c. Design-system primitives + patterns + reference modules. Lifted
	// verbatim from .docs/design/reference/_shared/styles/ so blocks (and
	// future render paths) have the full canonical scaffold available.
	// Auto-enqueued via the helper below so dropping a new file in those
	// dirs needs no functions.php edit.
	enqueue_styles_dir( 'dailyos-primitive', '/styles/primitives/', array( 'dailyos-aliases' ) );
	enqueue_styles_dir( 'dailyos-pattern', '/styles/patterns/', array( 'dailyos-aliases' ) );
	enqueue_styles_dir( 'dailyos-ref', '/styles/reference/', array( 'dailyos-aliases' ) );

	// 5. Chrome injector — emits FolioBar / NavIsland / Atmosphere from
	// body.dataset.* attributes. Includes Patch 9a DOM idempotency guard
	// so duplicate inject() calls (preview reload, partial refresh) only
	// render chrome once.
	wp_register_script(
		'dailyos-chrome',
		$base . '/chrome.js',
		array(),
		VERSION,
		true
	);
	wp_add_inline_script(
		'dailyos-chrome',
		'window.dailyosChrome = ' . wp_json_encode( chrome_config() ) . ';',
		'before'
	);
	wp_enqueue_script( 'dailyos-chrome' );
}
add_action( 'wp_enqueue_scripts', __NAMESPACE__ . '\enqueue_chrome_assets' );

/**
 * Enqueue every `*.module.css` (and bare `*.css`) under a sub-path of
 * `assets/` as its own stylesheet handle, keyed by lowercased base name.
 *
 * Used to wire the lifted design-system primitives + patterns + reference
 * modules in bulk without listing each in `functions.php`. Drop a new
 * `<Name>.module.css` into the target dir and it auto-enqueues on next
 * page load (no PHP edit required).
 *
 * Handle format: `<prefix>-<lowercased-basename>`. E.g.
 * `EntityChip.module.css` under `styles/primitives/` with prefix
 * `dailyos-primitive` enqueues as `dailyos-primitive-entitychip`.
 *
 * Dev-time tradeoff: one HTTP request per module file. Acceptable for the
 * scaffold stage. Future work (build-time concat, conditional per-block
 * enqueue) lands when bandwidth becomes the cost driver.
 *
 * @param string              $prefix      Handle prefix (e.g. `dailyos-primitive`).
 * @param string              $relative    Path under `assets/`, leading + trailing slash.
 * @param array<string,mixed> $deps        Enqueue dependencies (each file inherits the same).
 * @return void
 */
function enqueue_styles_dir( string $prefix, string $relative, array $deps ): void {
	$theme_dir = get_stylesheet_directory();
	$theme_uri = get_stylesheet_directory_uri();
	$abs_dir   = $theme_dir . '/assets' . $relative;
	if ( ! is_dir( $abs_dir ) ) {
		return;
	}
	$files = glob( $abs_dir . '*.css' );
	if ( false === $files || empty( $files ) ) {
		return;
	}
	sort( $files, SORT_STRING );
	foreach ( $files as $abs_file ) {
		$basename = basename( $abs_file );
		$slug     = strtolower( preg_replace( '/\.module\.css$|\.css$/', '', $basename ) );
		$handle   = $prefix . '-' . $slug;
		$src      = $theme_uri . '/assets' . $relative . $basename;
		wp_enqueue_style( $handle, $src, $deps, VERSION );
	}
}

/**
 * Per-surface chrome config — emitted as `window.dailyosChrome` and merged
 * into `body.dataset.*` by chrome.js (via Patch 2). Drives FolioBar label,
 * breadcrumbs, tint, and actions slot.
 *
 * V1.4.4 W3 scope: `dailyos_account` CPT is the primary surface. Stubs for
 * `dailyos_briefing`, `dailyos_person`, `dailyos_project`, `dailyos_meeting`
 * resolve to neutral defaults pending W2 entity surface work.
 *
 * @return array<string,string> Chrome config map (kebab-case keys become
 *                              `body.dataset.<camelCase>` post-merge).
 */
function chrome_config(): array {
	$base = array(
		'active-page'         => 'home',
		'tint'                => 'turmeric',
		'folio-label'         => 'DailyOS',
		'folio-crumbs'        => 'Today',
		'folio-date'          => '',
		'folio-home-href'     => home_url( '/' ),
		'nav-home-id'         => 'today',
		'nav-home-label'      => 'Today',
		'nav-home-href'       => home_url( '/' ),
		'nav-items-json'      => wp_json_encode( array() ),
		'folio-refresh-title' => 'Refresh',
	);

	// dailyos_account — primary v1.4.4 W3 surface.
	if ( is_singular( 'dailyos_account' ) ) {
		$post = get_post();
		return array_merge(
			$base,
			array(
				'active-page'  => 'accounts',
				'tint'         => 'turmeric',
				'folio-label'  => 'Account',
				'folio-crumbs' => $post ? get_the_title( $post ) : 'Account',
			)
		);
	}

	if ( is_post_type_archive( 'dailyos_account' ) ) {
		return array_merge(
			$base,
			array(
				'active-page'  => 'accounts',
				'tint'         => 'turmeric',
				'folio-label'  => 'Accounts',
				'folio-crumbs' => 'Accounts',
			)
		);
	}

	// Stub branches for W2 entity CPTs — neutral defaults until W2 ships.
	// Project surface gets its tint resolved at W2 L0 alongside the other entity tints.
	if ( is_singular( 'dailyos_briefing' ) ) {
		// Match the canonical DailyBriefingDSpine reference (briefing-d-spine.html):
		// FolioBar carries date + actions + chapters; DayStrip renders as a
		// secondary chrome bar via the briefing template part. Readiness pills
		// stay omitted until W2 wires real briefing claim data — the FolioBar
		// renders readiness only when `folio-readiness` is populated.
		return array_merge(
			$base,
			array(
				'active-page'         => 'today',
				'tint'                => 'turmeric',
				'folio-label'         => 'Daily Briefing',
				'folio-crumbs'        => 'Today',
				'folio-date'          => strtoupper( wp_date( 'l, F j, Y' ) ),
				'folio-actions'       => 'refresh',
				'folio-refresh-title' => 'Refresh briefing',
				'chapters'            => 'lead:alignleft:Lead|schedule:calendar:Today|moving:activity:Moving|watch:eye:Watch',
			)
		);
	}
	if ( is_singular( 'dailyos_person' ) ) {
		return array_merge(
			$base,
			array(
				'active-page' => 'people',
				'folio-label' => 'Person',
			)
		);
	}
	if ( is_singular( 'dailyos_project' ) ) {
		return array_merge(
			$base,
			array(
				'active-page' => 'projects',
				'folio-label' => 'Project',
			)
		);
	}
	if ( is_singular( 'dailyos_meeting' ) ) {
		return array_merge(
			$base,
			array(
				'active-page' => 'meetings',
				'folio-label' => 'Meeting',
			)
		);
	}

	return $base;
}

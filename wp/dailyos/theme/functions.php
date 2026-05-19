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
		return array_merge(
			$base,
			array(
				'active-page' => 'briefings',
				'folio-label' => 'Briefing',
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

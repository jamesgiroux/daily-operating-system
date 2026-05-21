<?php
/**
 * Accounts index list-shell server-side render.
 *
 * Per L0 Packet W2 V1.2.1 §5.5. The shell renders a minimal wrapper + first-
 * page slot. Client-side pagination is driven by `view.js` via
 * `wp.abilities.executeAbility('list_accounts', …)` consuming W1's
 * `Paginated<T>` + `CursorState` shape. The server-rendered HTML is the no-JS
 * fallback / loading skeleton; the hydrated React shell takes over on the
 * client.
 *
 * Producer named in this shell: `list_accounts`. Per AC-L.1, if W1 has not yet
 * landed this ability (substrate gap), the runtime returns an error which
 * surfaces as the `error` state in the hook output — the gap is loud, not
 * silently masked.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var string               $content    Inner content (unused; list shell has no inner blocks).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

$page_size = isset( $attributes['page_size'] ) ? (int) $attributes['page_size'] : 25;
if ( $page_size < 1 || $page_size > 200 ) {
	$page_size = 25;
}
$watermark = isset( $attributes['watermark'] ) ? (string) $attributes['watermark'] : '';

$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
	? get_block_wrapper_attributes(
		array(
			'class'                 => 'wp-block-dailyos-accounts-index',
			'data-ds-tier'          => 'pattern',
			'data-ds-name'          => 'AccountsIndex',
			'data-dailyos-surface'  => 'accounts_index',
			'data-dailyos-list-of'  => 'account',
			'data-dailyos-ability'  => 'list_accounts',
			'data-dailyos-page-size' => (string) $page_size,
			'data-dailyos-watermark' => $watermark,
		)
	)
	: 'class="wp-block-dailyos-accounts-index" data-dailyos-surface="accounts_index" data-dailyos-list-of="account" data-dailyos-ability="list_accounts" data-dailyos-page-size="' . (int) $page_size . '" data-dailyos-watermark="' . esc_attr( $watermark ) . '"';

$out  = '<section ' . $wrapper_attrs . '>';
$out .= '<div class="dailyos-accounts-index__mount" data-dailyos-accounts-index-mount>';
// No-JS fallback / pre-hydration skeleton. Empty state per §10 invariant is a
// quiet chip with data-empty-reason; surfaces post-mount once the hook
// resolves.
$out .= '<span class="dailyos-empty-chip" data-empty-reason="hydrating" aria-live="polite">'
	. esc_html__( 'Loading accounts…', 'dailyos' )
	. '</span>';
$out .= '</div>';
$out .= '</section>';

// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- wrapper attrs + escaped labels above.
echo $out;

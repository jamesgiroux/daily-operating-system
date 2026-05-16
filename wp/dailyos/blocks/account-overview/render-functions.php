<?php
/**
 * Account overview block server-side render.
 *
 * Per W4-A V3 §6.3 / AC §13: every render calls the runtime through
 * `DailyOS_Runtime_Client::project_composition_for_surface(...)`. The
 * substrate side owns the cache and the scope-identity authority; PHP
 * never derives or inspects scopes, never invokes W4-D directly, and
 * never serializes the projection body into block attributes.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content (empty for dynamic blocks).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_account_overview_render' ) ) {
	/**
	 * Render the account-overview block from attributes. The same function
	 * services both the block-registration render path and the editor
	 * preview REST route. Returns rendered HTML.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_account_overview_render( array $attributes ): string {
		$composition_id      = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';

		if ( '' === $composition_id ) {
			return '<div class="wp-block-dailyos-account-overview is-empty">'
				. esc_html__( 'No account context to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		// Duck-typed acceptance so PHPUnit can inject lightweight fakes
		// without subclassing the final transport class. Production code
		// passes a real DailyOS_Runtime_Client through the filter from
		// class-dailyos-plugin.php.
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return '<div class="wp-block-dailyos-account-overview is-empty">'
				. esc_html__( 'No account context to show here.', 'dailyos' )
				. '</div>';
		}

		$cache_hint_param = '' !== $cache_hint_token ? $cache_hint_token : null;
		$response         = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			$cache_hint_param
		);

		if ( is_wp_error( $response ) ) {
			return dailyos_account_overview_render_verification_banner();
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] )
			? $response['projection']
			: null;
		if ( null === $projection ) {
			return dailyos_account_overview_render_verification_banner();
		}

		$delivered_state         = function_exists( 'get_option' )
			? get_option( 'dailyos_composition_versions', [] )
			: [];
		$delivered_state_version = is_array( $delivered_state ) && isset( $delivered_state[ $composition_id ] )
			? (int) $delivered_state[ $composition_id ]
			: (int) ( $projection['composition_version'] ?? 0 );
		$is_stale                = $delivered_state_version > $composition_version;

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-account-overview',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'AccountOverview',
				]
			)
			: 'class="wp-block-dailyos-account-overview"';

		$blocks = isset( $projection['blocks'] ) && is_array( $projection['blocks'] )
			? $projection['blocks']
			: [];

		$out = '<section ' . $wrapper_attrs . '>';

		if ( $is_stale ) {
			$out .= dailyos_account_overview_render_stale_banner();
		}

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$out .= dailyos_account_overview_render_block( $block );
		}

		if ( empty( $blocks ) ) {
			$out .= '<p class="dailyos-empty">' . esc_html__( 'No account context to show here.', 'dailyos' ) . '</p>';
		}

		$out .= '<hr class="dailyos-finis-marker" aria-hidden="true" />';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Render a single projected block.
	 *
	 * @param array<string, mixed> $block Projected block payload.
	 * @return string Rendered HTML.
	 */
	function dailyos_account_overview_render_block( array $block ): string {
		$type          = isset( $block['block_type'] ) ? (string) $block['block_type'] : 'unknown';
		$trust         = isset( $block['trust_band'] ) ? (string) $block['trust_band'] : 'needs_verification';
		$visible_bands = [ 'likely_current', 'use_with_caution', 'needs_verification' ];
		if ( ! in_array( $trust, $visible_bands, true ) ) {
			$trust = 'needs_verification';
		}
		$label = isset( $block['title'] ) ? (string) $block['title'] : ucfirst( str_replace( '_', ' ', $type ) );
		$body  = isset( $block['summary'] ) ? (string) $block['summary'] : '';

		$out  = '<article class="dailyos-block dailyos-block-' . esc_attr( $type ) . '">';
		$out .= '<header><h3>' . esc_html( $label ) . '</h3>';
		$out .= '<span data-ds-tier="primitive" data-ds-name="TrustBandBadge" data-ds-spec="primitives/TrustBandBadge.md" data-ds-trust-band="' . esc_attr( $trust ) . '">';
		$out .= esc_html( dailyos_trust_band_label( $trust ) );
		$out .= '</span></header>';
		if ( '' !== $body ) {
			$out .= '<p>' . esc_html( $body ) . '</p>';
		}
		$out .= '</article>';
		return $out;
	}

	/**
	 * Gets the display label for a trust band.
	 *
	 * @param string $band Trust band.
	 * @return string Trust-band label.
	 */
	function dailyos_trust_band_label( string $band ): string {
		switch ( $band ) {
			case 'likely_current':
				return __( 'Likely current', 'dailyos' );
			case 'use_with_caution':
				return __( 'Use with caution', 'dailyos' );
			case 'needs_verification':
			default:
				return __( 'Needs verification', 'dailyos' );
		}
	}

	/**
	 * Renders the stale report banner.
	 *
	 * @return string Rendered banner HTML.
	 */
	function dailyos_account_overview_render_stale_banner(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="StaleReportBanner" data-ds-spec="patterns/StaleReportBanner.md" class="dailyos-stale-banner">'
			. '<p>' . esc_html__( 'Newer context has arrived. Refresh to bring this in.', 'dailyos' ) . '</p>'
			. '</aside>';
	}

	/**
	 * Renders the verification banner.
	 *
	 * @return string Rendered banner HTML.
	 */
	function dailyos_account_overview_render_verification_banner(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="ConsistencyFindingBanner" data-ds-spec="patterns/ConsistencyFindingBanner.md" class="dailyos-verification-banner">'
			. '<p>' . esc_html__( "Something about this account doesn't line up. Verify before acting.", 'dailyos' ) . '</p>'
			. '</aside>';
	}
}

<?php
/**
 * TrustBandBadge block server-side render.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_trust_band_badge_render' ) ) {
	/**
	 * Render the TrustBandBadge block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_trust_band_badge_render( array $attributes ): string {
		$composition_id = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		if ( '' === $composition_id ) {
			return dailyos_trust_band_badge_render_payload( $attributes );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return dailyos_trust_band_badge_render_payload( $attributes );
		}

		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';
		$response            = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			'' !== $cache_hint_token ? $cache_hint_token : null
		);

		return dailyos_trust_band_badge_render_from_projection( $response, $attributes );
	}

	/**
	 * Render TrustBandBadge from a projected composition response.
	 *
	 * @param mixed                $response Runtime response or WP_Error.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_trust_band_badge_render_from_projection( mixed $response, array $attributes ): string {
		if ( is_wp_error( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_trust_band_badge_render_payload( $attributes );
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] ) ? $response['projection'] : null;
		if ( null === $projection ) {
			return dailyos_trust_band_badge_render_payload( $attributes );
		}

		$payload = dailyos_trust_band_badge_select_payload( $projection, $attributes );
		return dailyos_trust_band_badge_render_payload( $payload ?? $attributes );
	}

	/**
	 * Select this primitive's payload from a projected composition.
	 *
	 * @param array<string, mixed> $projection Projected composition.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array<string, mixed>|null
	 */
	function dailyos_trust_band_badge_select_payload( array $projection, array $attributes ): ?array {
		$blocks           = isset( $projection['blocks'] ) && is_array( $projection['blocks'] ) ? $projection['blocks'] : [];
		$requested_block  = isset( $attributes['block_id'] ) ? (string) $attributes['block_id'] : '';
		$fallback_payload = null;

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$type = isset( $block['selected_known_type_id'] ) ? (string) $block['selected_known_type_id'] : '';
			if ( 'dailyos/trust-band-badge' !== $type ) {
				continue;
			}
			$payload = isset( $block['payload'] ) && is_array( $block['payload'] ) ? $block['payload'] : [];
			if ( ! isset( $payload['band'] ) && isset( $block['trust_band'] ) ) {
				$payload['band'] = (string) $block['trust_band'];
			}
			if ( '' !== $requested_block && isset( $block['block_id'] ) && $requested_block === (string) $block['block_id'] ) {
				return $payload;
			}
			$fallback_payload = $payload;
		}

		return $fallback_payload;
	}

	/**
	 * Render a TrustBandBadge payload.
	 *
	 * @param array<string, mixed> $payload Projected primitive payload.
	 * @return string
	 */
	function dailyos_trust_band_badge_render_payload( array $payload ): string {
		$band = isset( $payload['band'] ) ? (string) $payload['band'] : 'use_with_caution';
		if ( ! in_array( $band, [ 'likely_current', 'use_with_caution', 'needs_verification' ], true ) ) {
			$band = 'needs_verification';
		}

		$labels  = [
			'likely_current'     => esc_html__( 'Likely current', 'dailyos' ),
			'use_with_caution'   => esc_html__( 'Use with caution', 'dailyos' ),
			'needs_verification' => esc_html__( 'Needs verification', 'dailyos' ),
		];
		$label   = isset( $payload['label'] ) && '' !== (string) $payload['label'] ? (string) $payload['label'] : $labels[ $band ];
		$compact = ! empty( $payload['compact'] );
		$class   = 'dailyos-trust-band-badge' . ( $compact ? ' dailyos-trust-band-badge--compact' : '' );

		// DOS-692 a11y: aria-label per band + screen-reader announcement of
		// band transitions via aria-live. The rationale tooltip (if provided
		// by surface composition) is referenced via aria-describedby. The
		// badge is keyboard-focusable so screen-reader users can land on it
		// and announce the band state.
		$aria_labels   = [
			'likely_current'     => __( 'Trust band: likely current', 'dailyos' ),
			'use_with_caution'   => __( 'Trust band: use with caution', 'dailyos' ),
			'needs_verification' => __( 'Trust band: needs verification', 'dailyos' ),
		];
		$aria_label    = $aria_labels[ $band ];
		$rationale_id  = isset( $payload['rationale_id'] ) && is_string( $payload['rationale_id'] )
			? trim( $payload['rationale_id'] )
			: '';
		$describedby   = '' !== $rationale_id ? sprintf( ' aria-describedby="%s"', esc_attr( $rationale_id ) ) : '';

		return sprintf(
			'<span class="%s" data-band="%s" data-ds-name="TrustBandBadge" data-ds-tier="primitive" data-ds-spec="primitives/TrustBandBadge.md" role="status" tabindex="0" aria-label="%s" aria-live="polite"%s><span class="dailyos-trust-band-badge__dot" aria-hidden="true"></span>%s</span>',
			esc_attr( $class ),
			esc_attr( $band ),
			esc_attr( $aria_label ),
			$describedby,
			esc_html( $label )
		);
	}
}

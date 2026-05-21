<?php
/**
 * TrendStrip primitive server-side render (DOS-688).
 *
 * Translates Tauri React TrendStrip into a Gutenberg primitive block.
 *
 * Acceptance (W2 V1.2.1 §5.7 DOS-688):
 *  - AC-688.3: Visual parity matrix — wrapper carries data-ds-name + spec
 *    pointer; consumers compose with HealthBadge / ScoreBand / TrustBandBadge.
 *  - AC-688.4: Consumes envelope's trust band — no raw factor values surface
 *    on the headline; SVG sparkline is rendered from a quiet direction token,
 *    not from numeric factor scores.
 *
 * Voice-rule (DOS-325): the only headline text is the configured label
 * (defaults to "Trend") — never a raw number.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_trend_strip_render' ) ) {
	/**
	 * Render the TrendStrip primitive.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param array<string, mixed> $ctx        Block context (entityType, entityId,
	 *                                          envelopeHandle).
	 * @return string Rendered HTML.
	 */
	function dailyos_trend_strip_render( array $attributes, array $ctx = [] ): string {
		$handle = isset( $ctx['dailyos/envelopeHandle'] ) ? (string) $ctx['dailyos/envelopeHandle'] : '';
		$envelope = null;
		if ( '' !== $handle && function_exists( 'dailyos_envelope_cache_get' ) ) {
			$envelope = dailyos_envelope_cache_get( $handle );
		}

		$label = isset( $attributes['label'] ) ? (string) $attributes['label'] : '';
		if ( '' === $label ) {
			$label = __( 'Trend', 'dailyos' );
		}

		$direction = dailyos_trend_strip_resolve_direction( $envelope );
		$band      = dailyos_trend_strip_resolve_band( $envelope );

		$wrapper_class = sprintf(
			'wp-block-dailyos-trend-strip dailyos-trend-strip dailyos-trend-strip--%s dailyos-trend-strip--band-%s',
			$direction,
			$band
		);

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'         => $wrapper_class,
					'data-ds-tier'  => 'primitive',
					'data-ds-name'  => 'TrendStrip',
					'data-ds-spec'  => 'primitives/TrendStrip.md',
					'data-direction'=> $direction,
					'data-band'     => $band,
					'role'          => 'img',
					'aria-label'    => sprintf(
						/* translators: 1: label, 2: trend direction */
						__( '%1$s trend %2$s', 'dailyos' ),
						$label,
						$direction
					),
				]
			)
			: sprintf(
				'class="%s" data-ds-tier="primitive" data-ds-name="TrendStrip" data-ds-spec="primitives/TrendStrip.md" data-direction="%s" data-band="%s" role="img" aria-label="%s"',
				esc_attr( $wrapper_class ),
				esc_attr( $direction ),
				esc_attr( $band ),
				esc_attr( $label . ' trend ' . $direction )
			);

		$sparkline = dailyos_trend_strip_sparkline_svg( $direction );

		return sprintf(
			'<span %s><span class="dailyos-trend-strip__label">%s</span>%s</span>',
			$wrapper_attrs,
			esc_html( $label ),
			$sparkline
		);
	}

	/**
	 * Resolve a quiet trend direction token from the envelope.
	 *
	 * Reads only the band-derived direction; never surfaces raw factor scores.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return string One of: improving | stable | declining | volatile.
	 */
	function dailyos_trend_strip_resolve_direction( ?array $envelope ): string {
		if ( ! is_array( $envelope ) ) {
			return 'stable';
		}
		$candidates = [];
		if ( isset( $envelope['health'] ) && is_array( $envelope['health'] ) ) {
			$h = $envelope['health'];
			if ( isset( $h['trend'] ) ) {
				$candidates[] = $h['trend'];
			}
			if ( isset( $h['direction'] ) ) {
				$candidates[] = $h['direction'];
			}
		}
		if ( isset( $envelope['trust_band'] ) && is_array( $envelope['trust_band'] ) ) {
			$tb = $envelope['trust_band'];
			if ( isset( $tb['direction'] ) ) {
				$candidates[] = $tb['direction'];
			}
		}
		foreach ( $candidates as $cand ) {
			if ( ! is_string( $cand ) ) {
				continue;
			}
			$normalized = strtolower( trim( $cand ) );
			if ( in_array( $normalized, [ 'improving', 'stable', 'declining', 'volatile' ], true ) ) {
				return $normalized;
			}
		}
		return 'stable';
	}

	/**
	 * Resolve a trust band token from the envelope. Falls back to
	 * use_with_caution when no band-aggregate is present.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return string One of: likely_current | use_with_caution | needs_verification.
	 */
	function dailyos_trend_strip_resolve_band( ?array $envelope ): string {
		if ( ! is_array( $envelope ) ) {
			return 'use_with_caution';
		}
		$candidates = [];
		if ( isset( $envelope['trust_band'] ) ) {
			$candidates[] = is_array( $envelope['trust_band'] ) && isset( $envelope['trust_band']['band'] )
				? $envelope['trust_band']['band']
				: $envelope['trust_band'];
		}
		if ( isset( $envelope['health'] ) && is_array( $envelope['health'] ) && isset( $envelope['health']['aggregate_band'] ) ) {
			$candidates[] = $envelope['health']['aggregate_band'];
		}
		foreach ( $candidates as $cand ) {
			if ( ! is_string( $cand ) ) {
				continue;
			}
			$normalized = strtolower( trim( $cand ) );
			if ( in_array( $normalized, [ 'likely_current', 'use_with_caution', 'needs_verification' ], true ) ) {
				return $normalized;
			}
		}
		return 'use_with_caution';
	}

	/**
	 * Render a quiet sparkline shape from a direction token. No data points
	 * are surfaced as text; the SVG path is deterministic per direction.
	 *
	 * @param string $direction Trend direction token.
	 * @return string SVG markup.
	 */
	function dailyos_trend_strip_sparkline_svg( string $direction ): string {
		$paths = [
			'improving' => 'M0 18 L8 14 L16 12 L24 8 L32 6 L40 4',
			'stable'    => 'M0 12 L8 11 L16 12 L24 11 L32 12 L40 11',
			'declining' => 'M0 4 L8 6 L16 10 L24 12 L32 16 L40 18',
			'volatile'  => 'M0 12 L8 6 L16 14 L24 4 L32 16 L40 8',
		];
		$d = isset( $paths[ $direction ] ) ? $paths[ $direction ] : $paths['stable'];

		return sprintf(
			'<svg class="dailyos-trend-strip__spark" viewBox="0 0 40 22" width="40" height="22" aria-hidden="true" focusable="false"><path d="%s" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path></svg>',
			esc_attr( $d )
		);
	}
}

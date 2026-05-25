<?php
/**
 * HealthBadge block server-side render.
 *
 * Runtime projection supplies the typed primitive payload; PHP only formats
 * the display-safe HealthBadge markup.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_health_badge_render' ) ) {
	/**
	 * Render the HealthBadge block from runtime projection or direct attrs.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_health_badge_render( array $attributes ): string {
		$composition_id = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		if ( '' === $composition_id ) {
			return dailyos_health_badge_render_payload( $attributes );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return dailyos_health_badge_render_runtime_unavailable_notice();
		}

		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';
		$response            = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			'' !== $cache_hint_token ? $cache_hint_token : null
		);

		return dailyos_health_badge_render_from_projection( $response, $attributes );
	}

	/**
	 * Render HealthBadge from a projected composition response.
	 *
	 * @param mixed                $response Runtime response or WP_Error.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_health_badge_render_from_projection( mixed $response, array $attributes ): string {
		if ( is_wp_error( $response ) ) {
			return dailyos_health_badge_render_runtime_unavailable_notice();
		}
		if ( isset( $response['ok'] ) && false === $response['ok'] ) {
			$code = isset( $response['error']['code'] ) ? (string) $response['error']['code'] : '';
			if ( in_array( $code, [ 'runtime_unavailable', 'runtime_request_failed', 'runtime_invalid_json', 'runtime_http_error' ], true ) ) {
				return dailyos_health_badge_render_runtime_unavailable_notice();
			}
			return dailyos_health_badge_render_payload( $attributes );
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] ) ? $response['projection'] : null;
		if ( null === $projection ) {
			return dailyos_health_badge_render_payload( $attributes );
		}

		$payload = dailyos_health_badge_select_payload( $projection, $attributes );
		return dailyos_health_badge_render_payload( $payload ?? $attributes );
	}

	/**
	 * Select this primitive's payload from a projected composition.
	 *
	 * @param array<string, mixed> $projection Projected composition.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array<string, mixed>|null
	 */
	function dailyos_health_badge_select_payload( array $projection, array $attributes ): ?array {
		$blocks           = isset( $projection['blocks'] ) && is_array( $projection['blocks'] ) ? $projection['blocks'] : [];
		$requested_block  = isset( $attributes['block_id'] ) ? (string) $attributes['block_id'] : '';
		$fallback_payload = null;

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$type = isset( $block['selected_known_type_id'] ) ? (string) $block['selected_known_type_id'] : '';
			if ( 'dailyos/health-badge' !== $type ) {
				continue;
			}
			$payload = isset( $block['payload'] ) && is_array( $block['payload'] ) ? $block['payload'] : [];
			if ( '' !== $requested_block && isset( $block['block_id'] ) && $requested_block === (string) $block['block_id'] ) {
				return $payload;
			}
			$fallback_payload = $payload;
		}

		return $fallback_payload;
	}

	/**
	 * Render a HealthBadge payload.
	 *
	 * DOS-693 label discipline (DOS-325 voice rule):
	 *  - Apply the canonical 4-label vocabulary shared with ScoreBand
	 *    (On Track | Watching | Action Needed | No Read).
	 *  - Headline is the canonical label — never a raw numeric score.
	 *  - Numeric score still rendered as a separate secondary span when
	 *    sufficientData is true; the label takes the primary slot.
	 *  - Distinguishing context comes from the surrounding component label
	 *    (e.g., "Account health" vs "Renewal score"), not from divergent
	 *    band vocabularies (per W2 V1.2.1 §5.7 DOS-693 packet decision).
	 *
	 * @param array<string, mixed> $payload Projected primitive payload.
	 * @return string
	 */
	function dailyos_health_badge_render_payload( array $payload ): string {
		$size       = isset( $payload['size'] ) ? (string) $payload['size'] : 'standard';
		$band       = isset( $payload['band'] ) ? (string) $payload['band'] : 'green';
		$score      = isset( $payload['score'] ) ? (int) round( (float) $payload['score'] ) : 0;
		$trend      = isset( $payload['trend'] ) && is_array( $payload['trend'] ) ? $payload['trend'] : [];
		$direction  = isset( $trend['direction'] ) ? (string) $trend['direction'] : 'stable';
		$sufficient = isset( $payload['sufficientData'] ) && true === (bool) $payload['sufficientData'];
		$show_score = ! isset( $payload['showScore'] ) || true === (bool) $payload['showScore'];

		if ( ! in_array( $size, [ 'compact', 'standard', 'hero' ], true ) ) {
			$size = 'standard';
		}
		if ( ! in_array( $band, [ 'green', 'yellow', 'red' ], true ) ) {
			$band = 'green';
		}
		if ( ! in_array( $direction, [ 'improving', 'stable', 'declining', 'volatile' ], true ) ) {
			$direction = 'stable';
		}

		// DOS-693: canonical band-label vocabulary via vocabulary registry
		// (ADR-0083). Colour-band tokens map to discipline labels.
		$band_label_map = [
			'green'  => __( 'On Track', 'dailyos' ),
			'yellow' => __( 'Watching', 'dailyos' ),
			'red'    => __( 'Action Needed', 'dailyos' ),
		];
		$canonical_label = $band_label_map[ $band ];
		// Insufficient data state maps to the "No Read" vocabulary slot
		// (shared with ScoreBand) per DOS-693 AC.
		if ( ! $sufficient ) {
			$canonical_label = __( 'No Read', 'dailyos' );
		}

		$dot_class  = 'dailyos-health-badge__dot dailyos-health-badge__dot--' . $band;

		// Headline = canonical label. Numeric score moves to a secondary
		// slot rendered AFTER the label, and only when sufficientData is
		// true AND showScore is true (DOS-325: no raw numerics on headline).
		$label_html = sprintf(
			'<span class="dailyos-health-badge__label">%s</span>',
			esc_html( $canonical_label )
		);
		$score_html = $sufficient && $show_score
			? sprintf( '<span class="dailyos-health-badge__score" aria-hidden="true">%d</span>', $score )
			: '';

		$trend_glyphs = [
			'improving' => '&uarr;',
			'declining' => '&darr;',
			'stable'    => '&minus;',
			'volatile'  => '~',
		];
		$trend_html   = 'compact' !== $size
			? sprintf(
				'<span class="dailyos-health-badge__trend dailyos-health-badge__trend--%s" aria-label="%s">%s</span>',
				esc_attr( $direction ),
				esc_attr( 'trend ' . $direction ),
				$trend_glyphs[ $direction ]
			)
			: '';

		$wrapper_class = 'dailyos-health-badge dailyos-health-badge--' . $size . ' dailyos-health-badge--band-' . $band;

		return sprintf(
			'<span class="%s" data-band="%s" data-trend="%s" data-ds-name="HealthBadge" data-ds-tier="primitive" data-ds-spec="primitives/HealthBadge.md" data-label="%s"><span class="%s" aria-hidden="true"></span>%s%s%s</span>',
			esc_attr( $wrapper_class ),
			esc_attr( $band ),
			esc_attr( $direction ),
			esc_attr( $canonical_label ),
			esc_attr( $dot_class ),
			$label_html,
			$score_html,
			$trend_html
		);
	}

	/**
	 * Render the runtime-unavailable diagnostic for composed health renders.
	 */
	function dailyos_health_badge_render_runtime_unavailable_notice(): string {
		return '<span class="dailyos-health-badge dailyos-health-badge--runtime-unavailable" data-empty-reason="runtime_unavailable" data-ds-name="HealthBadge" data-ds-tier="primitive" data-ds-spec="primitives/HealthBadge.md" role="status">'
			. esc_html__( 'Runtime unavailable', 'dailyos' )
			. '</span>';
	}
}

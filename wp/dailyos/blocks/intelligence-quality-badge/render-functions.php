<?php
/**
 * IntelligenceQualityBadge block server-side render.
 *
 * Variant and label are derived from the projected claim quality score.
 * Optional tooltip text is generated on demand from display-safe values only.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_intelligence_quality_badge_render' ) ) {
	/**
	 * Render the IntelligenceQualityBadge block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_intelligence_quality_badge_render( array $attributes ): string {
		$composition_id = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		if ( '' === $composition_id ) {
			return dailyos_intelligence_quality_badge_render_payload( $attributes );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return dailyos_intelligence_quality_badge_render_payload( $attributes );
		}

		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';
		$response            = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			'' !== $cache_hint_token ? $cache_hint_token : null
		);

		return dailyos_intelligence_quality_badge_render_from_projection( $response, $attributes );
	}

	/**
	 * Render IntelligenceQualityBadge from a projected composition response.
	 *
	 * @param mixed                $response Runtime response or WP_Error.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_intelligence_quality_badge_render_from_projection( mixed $response, array $attributes ): string {
		if ( is_wp_error( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_intelligence_quality_badge_render_payload( $attributes );
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] ) ? $response['projection'] : null;
		if ( null === $projection ) {
			return dailyos_intelligence_quality_badge_render_payload( $attributes );
		}

		$payload = dailyos_intelligence_quality_badge_select_payload( $projection, $attributes );
		return dailyos_intelligence_quality_badge_render_payload( $payload ?? $attributes );
	}

	/**
	 * Select this primitive's payload from a projected composition.
	 *
	 * @param array<string, mixed> $projection Projected composition.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array<string, mixed>|null
	 */
	function dailyos_intelligence_quality_badge_select_payload( array $projection, array $attributes ): ?array {
		$blocks           = isset( $projection['blocks'] ) && is_array( $projection['blocks'] ) ? $projection['blocks'] : [];
		$requested_block  = isset( $attributes['block_id'] ) ? (string) $attributes['block_id'] : '';
		$fallback_payload = null;

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$type = isset( $block['selected_known_type_id'] ) ? (string) $block['selected_known_type_id'] : '';
			if ( 'dailyos/intelligence-quality-badge' !== $type ) {
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
	 * Render a projected IntelligenceQualityBadge payload.
	 *
	 * @param array<string, mixed> $payload Projected primitive payload.
	 * @return string
	 */
	function dailyos_intelligence_quality_badge_render_payload( array $payload ): string {
		$score           = isset( $payload['qualityScore'] ) ? (float) $payload['qualityScore'] : 0.0;
		$score           = max( 0.0, min( 1.0, $score ) );
		$level           = dailyos_intelligence_quality_badge_level( $score );
		$labels          = [
			'sparse'     => esc_html__( 'Sparse', 'dailyos' ),
			'developing' => esc_html__( 'Limited', 'dailyos' ),
			'ready'      => esc_html__( 'Ready', 'dailyos' ),
			'fresh'      => esc_html__( 'Fresh', 'dailyos' ),
		];
		$label           = $labels[ $level ];
		$show_label      = ! empty( $payload['showLabel'] );
		$has_new_signals = ! empty( $payload['hasNewSignals'] );
		$title_attr      = '';
		if ( ! empty( $payload['showTooltip'] ) ) {
			$title_attr = ' title="' . esc_attr( dailyos_intelligence_quality_badge_tooltip( $label, $payload ) ) . '"';
		}

		$out  = sprintf(
			'<span class="dailyos-intelligence-quality-badge" data-quality-level="%s" data-ds-name="IntelligenceQualityBadge" data-ds-tier="primitive" data-ds-spec="primitives/IntelligenceQualityBadge.md"%s>',
			esc_attr( $level ),
			$title_attr
		);
		$out .= '<span class="dailyos-intelligence-quality-badge__dotShell">';
		$out .= '<span class="dailyos-intelligence-quality-badge__dot" aria-hidden="true"></span>';
		if ( $has_new_signals ) {
			$out .= '<span class="dailyos-intelligence-quality-badge__newSignalDot" aria-hidden="true"></span>';
		}
		$out .= '</span>';
		if ( $show_label ) {
			$out .= '<span class="dailyos-intelligence-quality-badge__label">' . esc_html( $label ) . '</span>';
		}
		$out .= '</span>';
		return $out;
	}

	/**
	 * Map a numeric quality score to a discrete intelligence-quality level token.
	 *
	 * @param float $score Quality score in [0,1].
	 * @return string
	 */
	function dailyos_intelligence_quality_badge_level( float $score ): string {
		if ( $score < 0.25 ) {
			return 'sparse';
		}
		if ( $score < 0.6 ) {
			return 'developing';
		}
		if ( $score < 0.85 ) {
			return 'ready';
		}
		return 'fresh';
	}

	/**
	 * Build the tooltip text for the intelligence-quality badge, including last-update time.
	 *
	 * @param string               $label   Level label shown in the badge.
	 * @param array<string, mixed> $payload Block payload.
	 * @return string
	 */
	function dailyos_intelligence_quality_badge_tooltip( string $label, array $payload ): string {
		$raw_time  = isset( $payload['lastEnriched'] ) && '' !== (string) $payload['lastEnriched'] ? (string) $payload['lastEnriched'] : (string) ( $payload['enrichedAt'] ?? '' );
		$timestamp = '' !== $raw_time ? strtotime( $raw_time ) : false;
		if ( false === $timestamp ) {
			return $label . ' - Not yet updated';
		}
		return $label . ' - Last updated: ' . gmdate( 'M j, Y, g:ia', (int) $timestamp );
	}
}

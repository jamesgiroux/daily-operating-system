<?php
/**
 * FreshnessIndicator block server-side render.
 *
 * Relative/absolute labels are computed at render time so client markup never
 * carries raw timestamps unless a future DOS-477 policy explicitly permits it.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

require_once dirname( __DIR__ ) . '/_shared/chrome/render-empty.php';

if ( ! function_exists( 'dailyos_freshness_indicator_render' ) ) {
	/**
	 * Render the FreshnessIndicator block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_freshness_indicator_render( array $attributes ): string {
		$composition_id = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		if ( '' === $composition_id ) {
			return dailyos_freshness_indicator_render_payload( $attributes );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return dailyos_freshness_indicator_render_payload( $attributes );
		}

		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';
		$response            = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			'' !== $cache_hint_token ? $cache_hint_token : null
		);

		return dailyos_freshness_indicator_render_from_projection( $response, $attributes );
	}

	/**
	 * Render FreshnessIndicator from a projected composition response.
	 *
	 * @param mixed                $response Runtime response or WP_Error.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_freshness_indicator_render_from_projection( mixed $response, array $attributes ): string {
		if ( is_wp_error( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_freshness_indicator_render_payload( $attributes );
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] ) ? $response['projection'] : null;
		if ( null === $projection ) {
			return dailyos_freshness_indicator_render_payload( $attributes );
		}

		$payload = dailyos_freshness_indicator_select_payload( $projection, $attributes );
		return dailyos_freshness_indicator_render_payload( $payload ?? $attributes );
	}

	/**
	 * Select this primitive's payload from a projected composition.
	 *
	 * @param array<string, mixed> $projection Projected composition.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array<string, mixed>|null
	 */
	function dailyos_freshness_indicator_select_payload( array $projection, array $attributes ): ?array {
		$blocks           = isset( $projection['blocks'] ) && is_array( $projection['blocks'] ) ? $projection['blocks'] : [];
		$requested_block  = isset( $attributes['block_id'] ) ? (string) $attributes['block_id'] : '';
		$fallback_payload = null;

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$type = isset( $block['selected_known_type_id'] ) ? (string) $block['selected_known_type_id'] : '';
			if ( 'dailyos/freshness-indicator' !== $type ) {
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
	 * Render a projected FreshnessIndicator payload.
	 *
	 * @param array<string, mixed> $payload Projected primitive payload.
	 * @return string
	 */
	function dailyos_freshness_indicator_render_payload( array $payload ): string {
		$raw_time  = isset( $payload['at'] ) && '' !== (string) $payload['at'] ? (string) $payload['at'] : (string) ( $payload['enrichedAt'] ?? '' );
		$timestamp = '' !== $raw_time ? strtotime( $raw_time ) : false;
		$format    = isset( $payload['format'] ) ? (string) $payload['format'] : 'relative';
		$variant   = isset( $payload['variant'] ) ? (string) $payload['variant'] : 'inline';
		$threshold = isset( $payload['stalenessThreshold'] ) ? max( 1.0, (float) $payload['stalenessThreshold'] ) : 48.0;

		if ( ! in_array( $format, [ 'relative', 'absolute', 'both' ], true ) ) {
			$format = 'relative';
		}
		if ( ! in_array( $variant, [ 'inline', 'strip' ], true ) ) {
			$variant = ! empty( $payload['fragments'] ) || isset( $payload['enrichedAt'] ) ? 'strip' : 'inline';
		}

		if ( false === $timestamp ) {
			return dailyos_chrome_render_empty( null );
		}

		$staleness = dailyos_freshness_indicator_staleness( (int) $timestamp, $threshold );
		if ( 'strip' === $variant ) {
			return dailyos_freshness_indicator_render_strip( $payload, (int) $timestamp, $staleness );
		}

		$label = dailyos_freshness_indicator_inline_label( (int) $timestamp, $format, $staleness );
		return sprintf(
			'<span class="dailyos-freshness-indicator dailyos-freshness-indicator--inline" data-staleness="%s" data-ds-name="FreshnessIndicator" data-ds-tier="primitive" data-ds-spec="primitives/FreshnessIndicator.md"><span class="dailyos-freshness-indicator__timeText">%s</span></span>',
			esc_attr( $staleness ),
			esc_html( $label )
		);
	}

	/**
	 * Render the multi-part strip variant for the freshness-indicator block.
	 *
	 * @param array<string, mixed> $payload   Block payload.
	 * @param int                  $timestamp Source timestamp.
	 * @param string               $staleness Resolved staleness token.
	 * @return string
	 */
	function dailyos_freshness_indicator_render_strip( array $payload, int $timestamp, string $staleness ): string {
		$parts     = [];
		$fragments = isset( $payload['fragments'] ) && is_array( $payload['fragments'] ) ? $payload['fragments'] : [];
		foreach ( $fragments as $fragment ) {
			if ( is_string( $fragment ) && '' !== $fragment ) {
				$parts[] = [
					'text'  => $fragment,
					'stale' => false,
				];
			} elseif ( is_array( $fragment ) && isset( $fragment['text'] ) && '' !== (string) $fragment['text'] ) {
				$parts[] = [
					'text'  => (string) $fragment['text'],
					'stale' => ! empty( $fragment['stale'] ),
				];
			}
		}

		$verb        = isset( $payload['verb'] ) && '' !== (string) $payload['verb'] ? (string) $payload['verb'] : 'Updated';
		$date_format = isset( $payload['dateFormat'] ) ? (string) $payload['dateFormat'] : 'relative';
		$time_label  = 'relative' === $date_format
			? $verb . ' ' . dailyos_freshness_indicator_relative_age( $timestamp )
			: $verb . ' ' . gmdate( 'M j', $timestamp );
		$parts[]     = [
			'text'  => $time_label,
			'stale' => 'stale' === $staleness,
		];

		$out = sprintf(
			'<span class="dailyos-freshness-indicator dailyos-freshness-indicator--strip" data-staleness="%s" data-ds-name="FreshnessIndicator" data-ds-tier="primitive" data-ds-spec="primitives/FreshnessIndicator.md">',
			esc_attr( $staleness )
		);
		foreach ( $parts as $index => $part ) {
			$out .= '<span class="dailyos-freshness-indicator__part"' . ( $part['stale'] ? ' data-stale="true"' : '' ) . '>';
			if ( $index > 0 ) {
				$out .= '<span class="dailyos-freshness-indicator__separator" aria-hidden="true">&middot;</span>';
			}
			$out .= '<span class="dailyos-freshness-indicator__text' . ( count( $parts ) - 1 === $index ? ' dailyos-freshness-indicator__timeText' : '' ) . '">' . esc_html( (string) $part['text'] ) . '</span>';
			$out .= '</span>';
		}
		$out .= '</span>';
		return $out;
	}

	/**
	 * Compose the inline label for the freshness-indicator block based on format.
	 *
	 * @param int    $timestamp Source timestamp.
	 * @param string $format    Date format token (relative|absolute|both).
	 * @param string $staleness Resolved staleness token.
	 * @return string
	 */
	function dailyos_freshness_indicator_inline_label( int $timestamp, string $format, string $staleness ): string {
		$relative       = dailyos_freshness_indicator_relative_age( $timestamp );
		$relative_label = 'stale' === $staleness
			? 'stale ' . preg_replace( '/\s+ago$/', '', $relative )
			: $relative;

		if ( 'absolute' === $format ) {
			return gmdate( 'M j, g:ia', $timestamp );
		}
		if ( 'both' === $format ) {
			return $relative_label . ' - ' . gmdate( 'M j, g:ia', $timestamp );
		}
		return $relative_label;
	}

	/**
	 * Compute a short relative-age label (e.g. "5m ago", "2h ago") from a timestamp.
	 *
	 * @param int $timestamp Source timestamp.
	 * @return string
	 */
	function dailyos_freshness_indicator_relative_age( int $timestamp ): string {
		$diff_minutes = (int) floor( max( 0, time() - $timestamp ) / 60 );
		$diff_hours   = (int) floor( $diff_minutes / 60 );
		$diff_days    = (int) floor( $diff_hours / 24 );

		if ( $diff_minutes < 1 ) {
			return 'now';
		}
		if ( $diff_minutes < 60 ) {
			return $diff_minutes . 'm ago';
		}
		if ( $diff_hours < 24 ) {
			return $diff_hours . 'h ago';
		}
		if ( $diff_days < 7 ) {
			return $diff_days . 'd';
		}
		if ( $diff_days < 30 ) {
			return (int) floor( $diff_days / 7 ) . 'w';
		}
		return (int) floor( $diff_days / 30 ) . 'mo';
	}

	/**
	 * Classify content staleness as fresh/aging/stale relative to a threshold.
	 *
	 * @param int   $timestamp       Source timestamp.
	 * @param float $threshold_hours Aging threshold in hours.
	 * @return string
	 */
	function dailyos_freshness_indicator_staleness( int $timestamp, float $threshold_hours ): string {
		$age_hours = max( 0, time() - $timestamp ) / 3600;
		if ( $age_hours < $threshold_hours ) {
			return 'fresh';
		}
		if ( $age_hours < $threshold_hours * 2 ) {
			return 'aging';
		}
		return 'stale';
	}
}

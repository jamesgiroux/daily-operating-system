<?php
/**
 * Avatar block server-side render.
 *
 * The runtime resolves avatar source data. PHP does not invoke Tauri; it only
 * renders the projected display-safe URL through --dailyos-avatar-bg-url.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

require_once dirname( __DIR__ ) . '/_shared/chrome/render-empty.php';

if ( ! function_exists( 'dailyos_avatar_render' ) ) {
	/**
	 * Render the Avatar block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_avatar_render( array $attributes ): string {
		$composition_id = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		if ( '' === $composition_id ) {
			return dailyos_avatar_render_payload( $attributes );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return dailyos_avatar_render_payload( $attributes );
		}

		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';
		$response            = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			'' !== $cache_hint_token ? $cache_hint_token : null
		);

		return dailyos_avatar_render_from_projection( $response, $attributes );
	}

	/**
	 * Render Avatar from a projected composition response.
	 *
	 * @param mixed                $response Runtime response or WP_Error.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_avatar_render_from_projection( mixed $response, array $attributes ): string {
		if ( is_wp_error( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_avatar_render_payload( $attributes );
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] ) ? $response['projection'] : null;
		if ( null === $projection ) {
			return dailyos_avatar_render_payload( $attributes );
		}

		$payload = dailyos_avatar_select_payload( $projection, $attributes );
		return dailyos_avatar_render_payload( $payload ?? $attributes );
	}

	/**
	 * Select this primitive's payload from a projected composition.
	 *
	 * @param array<string, mixed> $projection Projected composition.
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array<string, mixed>|null
	 */
	function dailyos_avatar_select_payload( array $projection, array $attributes ): ?array {
		$blocks           = isset( $projection['blocks'] ) && is_array( $projection['blocks'] ) ? $projection['blocks'] : [];
		$requested_block  = isset( $attributes['block_id'] ) ? (string) $attributes['block_id'] : '';
		$fallback_payload = null;

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$type = isset( $block['selected_known_type_id'] ) ? (string) $block['selected_known_type_id'] : '';
			if ( 'dailyos/avatar' !== $type ) {
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
	 * Render a projected Avatar payload.
	 *
	 * @param array<string, mixed> $payload Projected primitive payload.
	 * @return string
	 */
	function dailyos_avatar_render_payload( array $payload ): string {
		$name      = isset( $payload['name'] ) ? trim( (string) $payload['name'] ) : '';
		$photo_url = isset( $payload['photoUrl'] ) ? trim( (string) $payload['photoUrl'] ) : '';
		$size      = isset( $payload['size'] ) ? (float) $payload['size'] : 32.0;
		if ( $size < 16.0 || $size > 160.0 ) {
			$size = 32.0;
		}

		$font_size = max( $size * 0.4, 10.0 );
		$label     = '' !== $name ? $name : esc_html__( 'Person', 'dailyos' );
		$initials  = dailyos_avatar_initials( $label );
		$vars      = sprintf(
			'--dailyos-avatar-size: %spx; --dailyos-avatar-font-size: %spx;',
			dailyos_avatar_number( $size ),
			dailyos_avatar_number( $font_size )
		);

		$css_url = dailyos_avatar_css_url_value( $photo_url );
		if ( '' !== $css_url ) {
			$vars .= ' --dailyos-avatar-bg-url: ' . $css_url . ';';
			return sprintf(
				'<span class="dailyos-avatar dailyos-avatarImage" role="img" aria-label="%s" style="%s" data-ds-name="Avatar" data-ds-tier="primitive" data-ds-spec="primitives/Avatar.md"></span>',
				esc_attr( $label ),
				esc_attr( $vars )
			);
		}

		return sprintf(
			'<span class="dailyos-avatar dailyos-avatarFallback" role="img" aria-label="%s" style="%s" data-ds-name="Avatar" data-ds-tier="primitive" data-ds-spec="primitives/Avatar.md">%s</span>',
			esc_attr( $label ),
			esc_attr( $vars ),
			dailyos_chrome_render_empty( $initials )
		);
	}

	function dailyos_avatar_initials( string $label ): string {
		$label = trim( $label );
		if ( '' === $label ) {
			return '?';
		}
		$first = function_exists( 'mb_substr' ) ? mb_substr( $label, 0, 1 ) : substr( $label, 0, 1 );
		$upper = function_exists( 'mb_strtoupper' ) ? mb_strtoupper( $first ) : strtoupper( $first );
		return '' !== $upper ? $upper : '?';
	}

	function dailyos_avatar_number( float $value ): string {
		$formatted = rtrim( rtrim( sprintf( '%.2F', $value ), '0' ), '.' );
		return '' !== $formatted ? $formatted : '0';
	}

	function dailyos_avatar_css_url_value( string $url ): string {
		if ( '' === $url ) {
			return '';
		}
		if ( ! preg_match( '#^(https?://|data:image/)#', $url ) ) {
			return '';
		}
		$escaped = str_replace( '\\', '\\\\', $url );
		$escaped = str_replace( '"', '\"', $escaped );
		$escaped = str_replace( "'", "\'", $escaped );
		$escaped = str_replace( [ "\n", "\r" ], '', $escaped );
		return 'url("' . $escaped . '")';
	}
}
